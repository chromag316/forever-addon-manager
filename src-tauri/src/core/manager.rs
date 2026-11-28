//! Scanning, installing, updating and removing addons, plus the saved state.

use super::curseforge::{self, Api};
use super::fingerprint;
use super::models::*;
use super::platform;
use super::sources::{self, GitSpec};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

type R<T> = Result<T, String>;

pub struct Manager {
    pub state: AppState,
    dir: PathBuf,
    pub http: reqwest::Client,
    pending: HashMap<i64, PendingUpdate>,
}

fn temp_path(prefix: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
}

fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            if entry.file_name() != ".git" {
                copy_dir(&entry.path(), &target)?;
            }
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// Moves a directory, falling back to copy+delete across volumes.
fn move_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    if fs::rename(src, dst).is_ok() {
        return Ok(());
    }
    copy_dir(src, dst)?;
    fs::remove_dir_all(src)
}

pub(crate) fn local_id(key: &str) -> i64 {
    -((fingerprint::murmur2(key.to_lowercase().as_bytes(), false) & 0x7fff_ffff) as i64) - 1
}

fn is_zip(path: &str) -> bool {
    path.trim().to_lowercase().ends_with(".zip")
}

fn short_ref(r: &str) -> String {
    if r.len() == 40 { r[..7].to_string() } else { r.to_string() }
}

impl Manager {
    pub fn load() -> Self {
        let dir = platform::data_dir();
        let _ = fs::create_dir_all(&dir);
        let state = fs::read(dir.join("state.json")).ok()
            .and_then(|d| serde_json::from_slice::<AppState>(&d).ok())
            .unwrap_or_default();
        Manager { state, dir, http: curseforge::http_client(), pending: HashMap::new() }
    }

    /// A manager with its data and AddOns folders in `root`, for tests.
    #[cfg(test)]
    pub fn at(root: &Path) -> Self {
        let dir = root.join("data");
        let _ = fs::create_dir_all(&dir);
        let mut state = AppState::default();
        state.settings.addons_path = root.join("AddOns").to_string_lossy().to_string();
        Manager { state, dir, http: curseforge::http_client(), pending: HashMap::new() }
    }

    pub fn save(&self) {
        if let Ok(json) = serde_json::to_vec_pretty(&self.state) {
            let tmp = self.dir.join("state.json.tmp");
            if fs::write(&tmp, json).is_ok() {
                let _ = fs::rename(tmp, self.dir.join("state.json"));
            }
        }
    }

    // ---- API key (a file readable only by the user) ----

    pub fn api_key(&self) -> String {
        if let Ok(k) = std::env::var("CURSEFORGE_API_KEY") {
            if !k.is_empty() {
                return curseforge::clean_key(&k);
            }
        }
        // Cleaned on read too, so a key saved with invisible characters still works.
        fs::read_to_string(self.dir.join("api-key")).map(|s| curseforge::clean_key(&s)).unwrap_or_default()
    }

    pub fn set_api_key(&self, key: &str) -> R<()> {
        let cleaned = curseforge::clean_key(key);
        let k = cleaned.as_str();
        let path = self.dir.join("api-key");
        if k.is_empty() {
            let _ = fs::remove_file(path);
            return Ok(());
        }
        fs::write(&path, k).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
        }
        Ok(())
    }

    pub fn api(&self) -> Api {
        Api::new(self.api_key())
    }

    // ---- AddOns folder ----

    pub fn addons_dir(&self) -> PathBuf {
        PathBuf::from(&self.state.settings.addons_path)
    }

    pub fn addon_folders(&self) -> Vec<String> {
        let mut v: Vec<String> = fs::read_dir(self.addons_dir()).map(|rd| rd.flatten()
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| !n.starts_with('.'))
            .collect()).unwrap_or_default();
        v.sort_by_key(|s| s.to_lowercase());
        v
    }

    /// Folders no tracked addon owns.
    pub fn untracked(&self) -> Vec<String> {
        let owned: HashSet<String> = self.state.installed.iter().flat_map(|a| a.folders.iter().map(|f| f.to_lowercase())).collect();
        self.addon_folders().into_iter().filter(|f| !owned.contains(&f.to_lowercase())).collect()
    }

    fn safe_folder(&self, name: &str) -> R<PathBuf> {
        if name.is_empty() || name.contains('/') || name.contains('\\') || name == "." || name == ".." {
            return Err(format!("Unsafe folder name: {name}"));
        }
        Ok(self.addons_dir().join(name))
    }

    /// Deletes a folder unless another tracked addon also ships it.
    fn remove_folder(&self, folder: &str, owner: i64) -> R<()> {
        let shared = self.state.installed.iter()
            .any(|a| a.mod_id != owner && a.folders.iter().any(|f| f.eq_ignore_ascii_case(folder)));
        if shared {
            return Ok(());
        }
        let p = self.safe_folder(folder)?;
        if p.exists() {
            fs::remove_dir_all(&p).map_err(|e| format!("Couldn't remove {folder}: {e}"))?;
        }
        Ok(())
    }

    /// Puts addon folders into AddOns, replacing existing ones and removing folders the
    /// previous version of this addon had but the new one doesn't.
    fn place(&self, items: &[(PathBuf, String)], copy: bool, owner: i64) -> R<Vec<String>> {
        fs::create_dir_all(self.addons_dir()).map_err(|e| format!("Can't create the AddOns folder: {e}"))?;
        let names: Vec<String> = items.iter().map(|(_, n)| n.clone()).collect();
        if let Some(old) = self.state.installed.iter().find(|a| a.mod_id == owner) {
            for f in &old.folders {
                if !names.iter().any(|n| n.eq_ignore_ascii_case(f)) {
                    self.remove_folder(f, owner)?;
                }
            }
        }
        for (src, name) in items {
            let dest = self.safe_folder(name)?;
            if dest.exists() {
                fs::remove_dir_all(&dest).map_err(|e| format!("Couldn't replace {name}: {e}"))?;
            }
            let res = if copy { copy_dir(src, &dest) } else { move_dir(src, &dest) };
            res.map_err(|e| format!("Couldn't install {name}: {e}"))?;
        }
        Ok(names)
    }

    fn store(&mut self, rec: InstalledAddon) {
        self.state.installed.retain(|a| a.mod_id != rec.mod_id);
        self.state.installed.push(rec);
        self.state.installed.sort_by_key(|a| a.name.to_lowercase());
        self.save();
    }

    // ---- Scan ----

    /// Fingerprint every folder and match against CurseForge. Returns how many matched.
    pub async fn scan(&mut self) -> R<usize> {
        // Git/folder installs are tracked by their source, not by fingerprints.
        let local_owned: HashSet<String> = self.state.installed.iter().filter(|a| a.kind() != AddonSource::Curseforge)
            .flat_map(|a| a.folders.iter().map(|f| f.to_lowercase())).collect();
        let dir = self.addons_dir();
        let folders: Vec<String> = self.addon_folders().into_iter().filter(|f| !local_owned.contains(&f.to_lowercase())).collect();
        let by_print: HashMap<u32, String> = tokio::task::spawn_blocking(move || {
            folders.into_iter().filter_map(|f| fingerprint::fingerprint(&dir.join(&f)).map(|fp| (fp, f))).collect()
        }).await.map_err(|e| e.to_string())?;

        let api = self.api();
        let prints: Vec<u32> = by_print.keys().copied().collect();
        let result = api.match_fingerprints(&prints).await?;
        let ids: Vec<i64> = result.exact_matches.iter().map(|m| m.id).collect();
        let mods: HashMap<i64, CfMod> = api.mods(&ids).await?.into_iter().map(|m| (m.id, m)).collect();
        let previous: HashMap<i64, InstalledAddon> = self.state.installed.iter().map(|a| (a.mod_id, a.clone())).collect();

        let mut found: HashMap<i64, InstalledAddon> = HashMap::new();
        for m in &result.exact_matches {
            let folders: Vec<String> = m.file.modules.iter().flatten().map(|x| x.name.clone()).collect();
            // Only claim the addon if all of its folders are present.
            if folders.is_empty() || !folders.iter().all(|n| by_print.values().any(|v| v.eq_ignore_ascii_case(n))) {
                continue;
            }
            let mut rec = record(mods.get(&m.id), m.id, &m.file);
            rec.ignored = previous.get(&m.id).map(|p| p.ignored).unwrap_or(false);
            found.insert(m.id, rec);
        }
        // Keep entries whose folders still exist but didn't match (Git/folder installs, local edits).
        for (id, old) in previous {
            if !found.contains_key(&id) && old.folders.iter().all(|f| self.addons_dir().join(f).exists()) {
                found.insert(id, old);
            }
        }
        let mut list: Vec<InstalledAddon> = found.into_values().collect();
        list.sort_by_key(|a| a.name.to_lowercase());
        self.state.installed = list;
        self.save();
        Ok(result.exact_matches.len())
    }

    // ---- Updates ----

    /// Checks CurseForge and Git addons. A CurseForge failure is reported in `error` but
    /// doesn't stop the Git checks.
    pub async fn check_updates(&mut self) -> UpdateCheck {
        let mut updates = vec![];
        let error = self.check_curseforge_updates(&mut updates).await.err();
        let gits: Vec<InstalledAddon> = self.state.installed.iter()
            .filter(|a| !a.ignored && a.kind() == AddonSource::Git).cloned().collect();
        for addon in gits {
            // One unreachable repo shouldn't block the other checks.
            if let Ok(Some(u)) = self.git_update(&addon).await {
                updates.push(u);
            }
        }
        self.pending = updates.iter().map(|u| (u.mod_id, u.clone())).collect();
        UpdateCheck { updates, error }
    }

    async fn check_curseforge_updates(&self, updates: &mut Vec<PendingUpdate>) -> R<()> {
        let s = self.state.settings.clone();
        let tracked: Vec<InstalledAddon> = self.state.installed.iter()
            .filter(|a| !a.ignored && a.kind() == AddonSource::Curseforge).cloned().collect();
        if tracked.is_empty() || self.api_key().is_empty() {
            return Ok(());
        }
        let api = self.api();
        let ids: Vec<i64> = tracked.iter().map(|a| a.mod_id).collect();
        for m in api.mods(&ids).await? {
            let Some(addon) = tracked.iter().find(|a| a.mod_id == m.id) else { continue };
            if let Some(best) = api.best_file(&m, s.game_version_type_id, s.release_type).await? {
                if best.id > addon.file_id {
                    let website_only = best.download_url.is_none() || m.allow_mod_distribution == Some(false);
                    updates.push(PendingUpdate {
                        mod_id: addon.mod_id, name: addon.name.clone(),
                        version_label: best.display_name.clone().unwrap_or_else(|| best.file_name.clone()),
                        website_only,
                        download_page: website_only.then(|| m.download_page(Some(best.id))).flatten(),
                        commit: None, file: Some(best),
                    });
                }
            }
        }
        Ok(())
    }

    /// A pending update for a Git addon, labelled with its .toc version when we can read it.
    async fn git_update(&self, addon: &InstalledAddon) -> R<Option<PendingUpdate>> {
        let Some(src) = &addon.source_url else { return Ok(None) };
        let spec = GitSpec::parse(src)?;
        let pending = |label: String, commit: Option<String>| PendingUpdate {
            mod_id: addon.mod_id, name: addon.name.clone(), version_label: label,
            website_only: false, download_page: None, commit, file: None,
        };
        if addon.git_from_release == Some(true) {
            // If GitHub doesn't answer (e.g. its 60 requests/hour limit), skip this round
            // instead of comparing the installed tag with a commit ID and showing a false update.
            return Ok(match sources::latest_release(&self.http, &spec).await {
                Some((tag, _)) if Some(&tag) != addon.git_ref.as_ref() => Some(pending(tag, None)),
                _ => None,
            });
        }
        let s = spec.clone();
        let sha = tokio::task::spawn_blocking(move || sources::remote_head(&s)).await.map_err(|e| e.to_string())??;
        if Some(&sha) == addon.git_ref.as_ref() {
            return Ok(None);
        }
        let version = match addon.folders.first() {
            Some(folder) => sources::github_toc_version(&self.http, &spec, &sha, folder).await,
            None => None,
        };
        // Show the version only when it differs from what's installed; otherwise the commit.
        Ok(Some(match version {
            Some(v) if Some(&v) != addon.display_name.as_ref() => pending(v, None),
            _ => pending(short_ref(&sha), Some(short_ref(&sha))),
        }))
    }

    pub async fn update(&mut self, mod_id: i64) -> R<()> {
        let Some(u) = self.pending.get(&mod_id).cloned() else { return Ok(()) };
        if let Some(file) = &u.file {
            self.install_file(None, file).await?;
        } else {
            self.reinstall(mod_id).await?;
        }
        self.pending.remove(&mod_id);
        Ok(())
    }

    /// Installs every pending update that can be downloaded here; website-only ones are
    /// listed as skipped.
    pub async fn update_all(&mut self) -> UpdateAllResult {
        let mut all: Vec<PendingUpdate> = self.pending.values().cloned().collect();
        all.sort_by_key(|u| u.name.to_lowercase());
        let mut result = UpdateAllResult::default();
        for u in all {
            if u.website_only {
                result.skipped.push(u.name);
                continue;
            }
            match self.update(u.mod_id).await {
                Ok(()) => result.updated.push(u.name),
                Err(_) => result.failed.push(u.name),
            }
        }
        result
    }

    // ---- CurseForge installs ----

    /// Installs the newest suitable file for a mod, plus its required dependencies. Returns
    /// the required dependencies that have to be downloaded from the CurseForge website.
    pub async fn install_mod(&mut self, mod_id: i64) -> R<Vec<WebsiteOnlyAddon>> {
        let api = self.api();
        let s = self.state.settings.clone();
        let mut queue = vec![mod_id];
        let mut seen = HashSet::new();
        let mut website_only = vec![];
        let mut first = true;
        while let Some(id) = queue.pop() {
            if !seen.insert(id) {
                continue;
            }
            let Some(m) = api.mods(&[id]).await?.into_iter().next() else { continue };
            // Prefer the chosen channel; if there is none (e.g. beta-only), take the newest file.
            let file = match api.best_file(&m, s.game_version_type_id, s.release_type).await? {
                Some(f) => Some(f),
                None => api.best_file(&m, s.game_version_type_id, 3).await?,
            };
            if m.allow_mod_distribution == Some(false) || file.as_ref().is_some_and(|f| f.download_url.is_none()) {
                if first {
                    return Err(format!("{}'s author only allows downloads from CurseForge. Download the zip from its page and drop it on this window.", m.name));
                }
                website_only.push(WebsiteOnlyAddon { name: m.name.clone(), download_page: m.download_page(file.as_ref().map(|f| f.id)) });
                continue;
            }
            let Some(file) = file else {
                if first { return Err(format!("No WoW Forever version of {} was found.", m.name)) } else { continue }
            };
            self.install_file(Some(&m), &file).await?;
            first = false;
            for d in file.dependencies.iter().flatten().filter(|d| d.relation_type == 3) {
                if !self.state.installed.iter().any(|a| a.mod_id == d.mod_id) {
                    queue.push(d.mod_id);
                }
            }
        }
        Ok(website_only)
    }

    async fn install_file(&mut self, m: Option<&CfMod>, file: &CfFile) -> R<()> {
        let name = m.map(|m| m.name.clone()).or(file.display_name.clone()).unwrap_or_else(|| file.file_name.clone());
        let Some(url) = &file.download_url else {
            return Err(format!("{name}'s author only allows downloads from CurseForge. Download the zip from its page and drop it on this window."));
        };
        let zip = temp_path("fam-dl").with_extension("zip");
        curseforge::download(&self.http, url, &zip).await?;
        let staging = temp_path("fam-x");
        let (z, st) = (zip.clone(), staging.clone());
        let res = tokio::task::spawn_blocking(move || sources::unzip(&z, &st)).await.map_err(|e| e.to_string());
        let _ = fs::remove_file(&zip);
        res??;
        let items: Vec<(PathBuf, String)> = fs::read_dir(&staging).map(|rd| rd.flatten()
            .filter(|e| e.path().is_dir())
            .map(|e| (e.path(), e.file_name().to_string_lossy().to_string()))
            .filter(|(_, n)| !n.starts_with('.') && n != "__MACOSX").collect()).unwrap_or_default();
        if items.is_empty() {
            let _ = fs::remove_dir_all(&staging);
            return Err("The download contained no addon folders.".into());
        }
        let placed = self.place(&items, false, file.mod_id);
        let _ = fs::remove_dir_all(&staging);
        let names = placed?;

        let old = self.state.installed.iter().find(|a| a.mod_id == file.mod_id).cloned();
        let mut rec = record(m, file.mod_id, file);
        if rec.folders.is_empty() {
            rec.folders = names;
        }
        if let Some(old) = old {
            rec.ignored = old.ignored;
            if m.is_none() {
                rec.name = old.name;
                rec.author = old.author;
                rec.website_url = old.website_url;
                rec.thumbnail_url = old.thumbnail_url;
            }
        }
        self.store(rec);
        Ok(())
    }

    // ---- Git, folder and zip installs ----

    /// Latest GitHub release zip if there is one, otherwise a shallow clone.
    pub async fn install_git(&mut self, input: &str) -> R<()> {
        let spec = GitSpec::parse(input)?;
        let id = local_id(&spec.url);
        let staging = temp_path("fam-git");
        fs::create_dir_all(&staging).map_err(|e| e.to_string())?;
        let result = self.install_git_into(&spec, id, input, &staging).await;
        let _ = fs::remove_dir_all(&staging);
        result
    }

    async fn install_git_into(&mut self, spec: &GitSpec, id: i64, input: &str, staging: &Path) -> R<()> {
        let (git_ref, from_release) = match (spec.branch.is_none(), sources::latest_release(&self.http, spec).await) {
            (true, Some((tag, url))) => {
                let zip = staging.join("release.zip");
                curseforge::download(&self.http, &url, &zip).await?;
                let (z, st) = (zip.clone(), staging.join("x"));
                tokio::task::spawn_blocking(move || sources::unzip(&z, &st)).await.map_err(|e| e.to_string())??;
                (tag, true)
            }
            _ => {
                let (sp, dir) = (spec.clone(), staging.join("x").join(&spec.repo));
                let sha = tokio::task::spawn_blocking(move || sources::clone(&sp, &dir)).await.map_err(|e| e.to_string())??;
                (sha, false)
            }
        };
        let found = sources::addon_folders(&staging.join("x"), 0);
        if found.is_empty() {
            return Err(format!("No WoW addon (.toc file) found in {}.", spec.url));
        }
        let names = self.place(&found, false, id)?;
        let meta = sources::metadata(&self.addons_dir().join(&names[0]));
        let mut rec = self.local_record(id, &names, &meta, &spec.repo);
        rec.source = Some(AddonSource::Git);
        rec.source_url = Some(input.trim().to_string());
        rec.display_name = meta.1.clone().or_else(|| Some(short_ref(&git_ref)));
        rec.git_ref = Some(git_ref);
        rec.git_from_release = Some(from_release);
        rec.git_branch = spec.branch.clone();
        rec.website_url = spec.owner.as_ref().map(|o| format!("https://github.com/{o}/{}", spec.repo));
        self.store(rec);
        Ok(())
    }

    /// Copies an addon folder (or a folder containing several addons) into AddOns.
    pub fn install_folder(&mut self, path: &str) -> R<()> {
        let root = PathBuf::from(path.trim());
        if root == self.addons_dir() {
            return Err("That's the AddOns folder itself; choose an addon folder.".into());
        }
        let found = sources::addon_folders(&root, 0);
        if found.is_empty() {
            return Err(format!("No WoW addon (.toc file) found in {}.", root.display()));
        }
        let id = local_id(&format!("folder:{}", root.display()));
        let names = self.place(&found, true, id)?;
        let meta = sources::metadata(&self.addons_dir().join(&names[0]));
        let fallback = root.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let mut rec = self.local_record(id, &names, &meta, &fallback);
        rec.source = Some(AddonSource::Folder);
        rec.source_url = Some(root.display().to_string());
        self.store(rec);
        Ok(())
    }

    /// Installs a zip the user downloaded themselves. If CurseForge recognises it, it's tracked
    /// as a CurseForge addon (with update checks); otherwise as a local addon from this zip.
    pub async fn install_zip(&mut self, zip: &str) -> R<()> {
        let zip = zip.trim();
        if !Path::new(zip).is_file() {
            return Err(format!("{zip} doesn't exist any more. Download the zip again and drop it on this window."));
        }
        let staging = temp_path("fam-zip");
        let (z, st) = (PathBuf::from(zip), staging.clone());
        let res = tokio::task::spawn_blocking(move || sources::unzip(&z, &st)).await.map_err(|e| e.to_string());
        let found = res.and_then(|r| r).map(|_| sources::addon_folders(&staging, 0));
        let outcome = match found {
            Ok(found) if found.is_empty() => Err("The zip contains no WoW addon.".to_string()),
            Ok(found) => {
                // One id per addon (by its main folder), so a newer zip replaces the older one.
                let id = local_id(&format!("zip:{}", found[0].1));
                // Forget an earlier local install of these folders so the scan can match them.
                self.state.installed.retain(|a| a.mod_id != id);
                self.place(&found, false, id).map(|names| (id, names))
            }
            Err(e) => Err(e),
        };
        let _ = fs::remove_dir_all(&staging);
        let (id, names) = outcome?;
        if !self.api_key().is_empty() {
            self.scan().await?;
        }
        let claimed = self.state.installed.iter().any(|a| a.folders.iter().any(|f| f.eq_ignore_ascii_case(&names[0])));
        if !claimed {
            let meta = sources::metadata(&self.addons_dir().join(&names[0]));
            let mut rec = self.local_record(id, &names, &meta, &names[0]);
            rec.source = Some(AddonSource::Folder);
            rec.source_url = Some(zip.to_string());
            self.store(rec);
        }
        Ok(())
    }

    pub async fn reinstall(&mut self, mod_id: i64) -> R<()> {
        let Some(a) = self.state.installed.iter().find(|a| a.mod_id == mod_id).cloned() else { return Ok(()) };
        match (a.kind(), a.source_url) {
            (AddonSource::Git, Some(src)) => self.install_git(&src).await,
            (AddonSource::Folder, Some(src)) if is_zip(&src) => self.install_zip(&src).await,
            (AddonSource::Folder, Some(src)) => self.install_folder(&src),
            _ => Ok(()),
        }
    }

    fn local_record(&self, id: i64, folders: &[String], meta: &(Option<String>, Option<String>, Option<String>), fallback: &str) -> InstalledAddon {
        let ignored = self.state.installed.iter().find(|a| a.mod_id == id).map(|a| a.ignored).unwrap_or(false);
        InstalledAddon {
            mod_id: id, name: meta.0.clone().unwrap_or_else(|| fallback.to_string()), author: meta.2.clone(),
            website_url: None, thumbnail_url: None, file_id: 0, file_name: folders.join(", "),
            display_name: meta.1.clone(), file_date: None, folders: folders.to_vec(), dependency_ids: vec![],
            ignored, source: None, source_url: None, git_ref: None, git_from_release: None, git_branch: None,
        }
    }

    pub fn uninstall(&mut self, mod_id: i64) -> R<()> {
        let Some(a) = self.state.installed.iter().find(|a| a.mod_id == mod_id).cloned() else { return Ok(()) };
        for f in &a.folders {
            self.remove_folder(f, mod_id)?;
        }
        self.state.installed.retain(|x| x.mod_id != mod_id);
        self.pending.remove(&mod_id);
        self.save();
        Ok(())
    }

    pub fn set_ignored(&mut self, mod_id: i64, ignored: bool) {
        if let Some(a) = self.state.installed.iter_mut().find(|a| a.mod_id == mod_id) {
            a.ignored = ignored;
        }
        if ignored {
            self.pending.remove(&mod_id);
        }
        self.save();
    }
}

fn record(m: Option<&CfMod>, mod_id: i64, file: &CfFile) -> InstalledAddon {
    InstalledAddon {
        mod_id,
        name: m.map(|m| m.name.clone()).or(file.display_name.clone()).unwrap_or_else(|| file.file_name.clone()),
        author: m.and_then(|m| m.authors.as_ref()?.first().map(|a| a.name.clone())),
        website_url: m.and_then(|m| m.website_url()),
        thumbnail_url: m.and_then(|m| m.logo.as_ref()?.thumbnail_url.clone()),
        file_id: file.id,
        file_name: file.file_name.clone(),
        display_name: file.display_name.clone(),
        file_date: file.file_date.clone(),
        folders: file.modules.iter().flatten().map(|x| x.name.clone()).collect(),
        dependency_ids: file.dependencies.iter().flatten().filter(|d| d.relation_type == 3).map(|d| d.mod_id).collect(),
        ignored: false,
        source: None, source_url: None, git_ref: None, git_from_release: None, git_branch: None,
    }
}
