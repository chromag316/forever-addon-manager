//! Git URLs, GitHub releases, zip extraction, and finding addon folders in a directory.

use super::platform;
use regex::Regex;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub struct GitSpec {
    pub url: String,
    pub owner: Option<String>, // GitHub only
    pub repo: String,
    pub branch: Option<String>,
}

impl GitSpec {
    pub fn parse(input: &str) -> Result<GitSpec, String> {
        let bad = || format!("\"{}\" doesn't look like a Git URL.", input.trim());
        let mut s = input.trim().to_string();
        if s.is_empty() {
            return Err(bad());
        }
        // "owner/repo" shorthand for GitHub
        if !s.contains(':') && !s.contains('.') && s.split('/').count() == 2 && !s.contains(' ') {
            s = format!("https://github.com/{s}");
        }
        if s.starts_with("github.com/") {
            s = format!("https://{s}");
        }
        let lower = s.to_lowercase();
        for prefix in ["https://github.com/", "http://github.com/", "https://www.github.com/"] {
            if let Some(rest) = lower.strip_prefix(prefix).map(|_| &s[prefix.len()..]) {
                let parts: Vec<&str> = rest.split('/').filter(|p| !p.is_empty()).collect();
                if parts.len() < 2 {
                    return Err(bad());
                }
                let owner = parts[0].to_string();
                let repo = parts[1].trim_end_matches(".git").to_string();
                let branch = (parts.len() >= 4 && parts[2] == "tree").then(|| parts[3..].join("/"));
                return Ok(GitSpec { url: format!("https://github.com/{owner}/{repo}.git"), owner: Some(owner), repo, branch });
            }
        }
        let ssh_like = Regex::new(r"^[\w.-]+@[\w.-]+:").unwrap().is_match(&s);
        let looks_git = ["https://", "http://", "ssh://", "git://"].iter().any(|p| lower.starts_with(p)) || ssh_like;
        if !looks_git || s.contains(' ') {
            return Err(bad());
        }
        let repo = s.rsplit(['/', ':']).next().unwrap_or("addon").trim_end_matches(".git").to_string();
        Ok(GitSpec { url: s, owner: None, repo, branch: None })
    }
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}
#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

/// Latest GitHub release that has a .zip asset (preferring a Forever build).
pub async fn latest_release(http: &reqwest::Client, spec: &GitSpec) -> Option<(String, String)> {
    let owner = spec.owner.as_ref()?;
    let url = format!("https://api.github.com/repos/{owner}/{}/releases/latest", spec.repo);
    let resp = http.get(url).header("Accept", "application/vnd.github+json")
        .timeout(super::curseforge::API_TIMEOUT).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let rel: Release = resp.json().await.ok()?;
    let zips: Vec<&Asset> = rel.assets.iter().filter(|a| a.name.to_lowercase().ends_with(".zip")).collect();
    let other = ["classic", "bcc", "tbc", "wrath", "wotlk", "cata", "mists", "mop", "vanilla", "era"];
    let pick = zips.iter().find(|a| a.name.to_lowercase().contains("forever"))
        .or_else(|| zips.iter().find(|a| !other.iter().any(|o| a.name.to_lowercase().contains(o))))
        .or(zips.first())?;
    Some((rel.tag_name, pick.browser_download_url.clone()))
}

pub fn git(args: &[&str]) -> Result<String, String> {
    let out = platform::command("git").args(args).env("GIT_TERMINAL_PROMPT", "0").output()
        .map_err(|_| "Git isn't installed. Install it from https://git-scm.com/downloads".to_string())?;
    if out.status.success() {
        return Ok(String::from_utf8_lossy(&out.stdout).to_string());
    }
    let msg = String::from_utf8_lossy(&out.stderr).to_string();
    if msg.contains("could not read Username") || msg.contains("not found") {
        return Err("Git: repository not found, or it's private.".into());
    }
    Err(format!("Git: {}", msg.lines().rev().find(|l| l.starts_with("fatal:")).unwrap_or(msg.trim())))
}

/// Commit SHA at the tip of the branch (or the default branch).
pub fn remote_head(spec: &GitSpec) -> Result<String, String> {
    let r = spec.branch.as_ref().map(|b| format!("refs/heads/{b}")).unwrap_or_else(|| "HEAD".into());
    let out = git(&["ls-remote", &spec.url, &r])?;
    out.split_whitespace().next().filter(|s| s.len() == 40).map(String::from)
        .ok_or_else(|| format!("Git: couldn't read {r} of {}", spec.url))
}

/// Shallow clone; returns the commit SHA and removes the .git folder.
pub fn clone(spec: &GitSpec, dir: &Path) -> Result<String, String> {
    let d = dir.to_string_lossy().to_string();
    let mut args = vec!["clone", "--depth", "1", "--recurse-submodules", "--shallow-submodules"];
    if let Some(b) = &spec.branch {
        args.extend(["--branch", b.as_str()]);
    }
    args.extend([spec.url.as_str(), d.as_str()]);
    git(&args)?;
    let sha = git(&["-C", &d, "rev-parse", "HEAD"])?.trim().to_string();
    let _ = fs::remove_dir_all(dir.join(".git"));
    Ok(sha)
}

/// Extracts a zip safely (entries can't escape `dest`).
pub fn unzip(zip_path: &Path, dest: &Path) -> Result<(), String> {
    let file = fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Couldn't read the zip: {e}"))?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let Some(rel) = entry.enclosed_name() else { continue };
        let out = dest.join(rel);
        if entry.is_dir() {
            fs::create_dir_all(&out).map_err(|e| e.to_string())?;
        } else {
            if let Some(p) = out.parent() {
                fs::create_dir_all(p).map_err(|e| e.to_string())?;
            }
            let mut f = fs::File::create(&out).map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut f).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn flavor_suffix() -> Regex {
    Regex::new(r"(?i)[-_](bcc|tbc|mainline|classic|vanilla|wrath|wotlkc|cata|mists|forever|camelot)$").unwrap()
}

fn toc_files(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(dir).map(|rd| rd.flatten().map(|e| e.path())
        .filter(|p| p.extension().map(|e| e.eq_ignore_ascii_case("toc")).unwrap_or(false)).collect()).unwrap_or_default();
    v.sort();
    v
}

fn base_name(toc: &Path) -> String {
    let stem = toc.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    flavor_suffix().replace(&stem, "").to_string()
}

/// (source dir, folder name it needs in AddOns). Handles: the dir is an addon; it contains
/// addon folders; or those are wrapped in an extra level (e.g. "repo-main/").
pub fn addon_folders(root: &Path, depth: usize) -> Vec<(PathBuf, String)> {
    let tocs = toc_files(root);
    if !tocs.is_empty() {
        let dir_name = root.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        // WoW only loads a folder whose name matches its .toc.
        let name = tocs.iter().map(|t| base_name(t)).find(|b| b.eq_ignore_ascii_case(&dir_name))
            .unwrap_or_else(|| base_name(&tocs[0]));
        return vec![(root.to_path_buf(), name)];
    }
    if depth >= 3 {
        return vec![];
    }
    let mut subdirs: Vec<PathBuf> = fs::read_dir(root).map(|rd| rd.flatten().map(|e| e.path())
        .filter(|p| p.is_dir())
        .filter(|p| { let n = p.file_name().unwrap().to_string_lossy(); !n.starts_with('.') && n != "__MACOSX" })
        .collect()).unwrap_or_default();
    subdirs.sort();
    let direct: Vec<&PathBuf> = subdirs.iter().filter(|d| !toc_files(d).is_empty()).collect();
    if !direct.is_empty() {
        return direct.into_iter().flat_map(|d| addon_folders(d, depth + 1)).collect();
    }
    subdirs.iter().flat_map(|d| addon_folders(d, depth + 1)).collect()
}

/// "## Title", "## Version" and "## Author" from the addon's .toc (colour codes stripped).
pub fn metadata(folder: &Path) -> (Option<String>, Option<String>, Option<String>) {
    let Some(toc) = toc_files(folder).into_iter().next() else { return (None, None, None) };
    let Ok(bytes) = fs::read(toc) else { return (None, None, None) };
    let text = String::from_utf8_lossy(&bytes);
    (toc_field(&text, "Title"), toc_field(&text, "Version"), toc_field(&text, "Author"))
}

/// A `## Key: value` line from a .toc file, without color codes. None when missing, empty or
/// an unreplaced packager token like `@project-version@`.
pub fn toc_field(text: &str, key: &str) -> Option<String> {
    let colors = Regex::new(r"\|c[0-9a-fA-F]{8}|\|r").unwrap();
    for line in text.lines() {
        let l = line.trim().trim_start_matches('\u{FEFF}');
        let Some(rest) = l.strip_prefix("##") else { continue };
        let Some((k, v)) = rest.split_once(':') else { continue };
        if k.trim().eq_ignore_ascii_case(key) {
            let v = colors.replace_all(v.trim(), "").trim().to_string();
            if v.is_empty() || (v.starts_with('@') && v.ends_with('@')) {
                return None;
            }
            return Some(v);
        }
    }
    None
}

/// `## Version` of a GitHub addon at a given commit, read from raw.githubusercontent.com
/// (doesn't count against GitHub's API rate limit). Tries the usual .toc locations.
pub async fn github_toc_version(http: &reqwest::Client, spec: &GitSpec, sha: &str, folder: &str) -> Option<String> {
    let owner = spec.owner.as_ref()?;
    let candidates = [
        format!("{folder}.toc"),
        format!("{folder}_Forever.toc"),
        format!("{folder}-Forever.toc"),
        format!("{folder}/{folder}.toc"),
    ];
    for path in candidates {
        let url = format!("https://raw.githubusercontent.com/{owner}/{}/{sha}/{path}", spec.repo);
        let Ok(resp) = http.get(url).timeout(super::curseforge::API_TIMEOUT).send().await else { return None };
        if resp.status().is_success() {
            let text = resp.text().await.ok()?;
            return toc_field(&text, "Version");
        }
    }
    None
}
