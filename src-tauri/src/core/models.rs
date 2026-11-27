//! Data model. The saved state uses the same JSON layout as the earlier macOS app, so an
//! existing `state.json` keeps working.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AddonSource {
    Curseforge,
    Git,
    Folder,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InstalledAddon {
    pub mod_id: i64,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub website_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,
    pub file_id: i64,
    pub file_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_date: Option<String>,
    pub folders: Vec<String>,
    #[serde(default)]
    pub dependency_ids: Vec<i64>,
    #[serde(default)]
    pub ignored: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<AddonSource>,
    #[serde(rename = "sourceURL", default, skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_from_release: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,
}

impl InstalledAddon {
    pub fn kind(&self) -> AddonSource {
        self.source.unwrap_or(AddonSource::Curseforge)
    }
    pub fn version(&self) -> String {
        self.display_name.clone().unwrap_or_else(|| self.file_name.clone())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub addons_path: String,
    /// CurseForge game version type for WoW Forever.
    #[serde(default = "default_gvt")]
    pub game_version_type_id: i64,
    /// 1 = release, 2 = beta, 3 = alpha (newest allowed channel).
    #[serde(default = "default_release")]
    pub release_type: i64,
    /// Install updates automatically; on when never set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_update: Option<bool>,
}

fn default_gvt() -> i64 {
    88568
}
fn default_release() -> i64 {
    1
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            addons_path: crate::core::platform::default_addons_path(),
            game_version_type_id: default_gvt(),
            release_type: default_release(),
            auto_update: None,
        }
    }
}

impl Settings {
    pub fn auto_updates(&self) -> bool {
        self.auto_update.unwrap_or(true)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AppState {
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub installed: Vec<InstalledAddon>,
}

// ---- CurseForge API (subset) ----

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct CfAuthor {
    pub name: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct CfLogo {
    pub thumbnail_url: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct CfLinks {
    pub website_url: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CfFileIndex {
    pub file_id: i64,
    pub release_type: i64,
    pub game_version_type_id: Option<i64>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CfMod {
    pub id: i64,
    pub name: String,
    pub slug: Option<String>,
    pub summary: Option<String>,
    pub links: Option<CfLinks>,
    pub authors: Option<Vec<CfAuthor>>,
    pub logo: Option<CfLogo>,
    pub download_count: Option<f64>,
    pub latest_files: Option<Vec<CfFile>>,
    pub latest_files_indexes: Option<Vec<CfFileIndex>>,
    pub allow_mod_distribution: Option<bool>,
    /// Set by `annotate`: the author only allows downloads on the CurseForge website.
    #[serde(default)]
    pub website_only: bool,
    /// Set by `annotate`: the file `best_index` picks for the chosen game version.
    #[serde(default)]
    pub target_file_id: Option<i64>,
}

impl CfMod {
    /// Newest indexed file for this game version at or below the release type.
    pub fn best_index(&self, gvt: i64, release_type: i64) -> Option<&CfFileIndex> {
        self.latest_files_indexes.iter().flatten()
            .filter(|i| i.game_version_type_id == Some(gvt) && i.release_type <= release_type)
            .max_by_key(|i| i.file_id)
    }

    /// Fills `website_only` and `target_file_id` for the UI. Website-only when the author
    /// turned off third-party downloads, or when the file CurseForge lists has no download URL.
    pub fn annotate(&mut self, gvt: i64, release_type: i64) {
        let idx = self.best_index(gvt, release_type).or_else(|| self.best_index(gvt, 3)).map(|i| i.file_id);
        let no_url = idx.and_then(|id| self.latest_files.iter().flatten().find(|f| f.id == id))
            .map(|f| f.download_url.is_none()).unwrap_or(false);
        self.target_file_id = idx;
        self.website_only = self.allow_mod_distribution == Some(false) || no_url;
    }

    /// CurseForge page that downloads this file (or the project page without a file).
    pub fn download_page(&self, file_id: Option<i64>) -> Option<String> {
        let base = self.website_url()?;
        Some(match file_id {
            Some(id) => format!("{}/download/{id}", base.trim_end_matches('/')),
            None => base,
        })
    }

    pub fn website_url(&self) -> Option<String> {
        if let Some(u) = self.links.as_ref().and_then(|l| l.website_url.clone()) {
            return Some(u);
        }
        self.slug.as_ref().map(|s| format!("https://www.curseforge.com/wow/addons/{s}"))
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CfModule {
    pub name: String,
    pub fingerprint: u32,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CfDependency {
    pub mod_id: i64,
    pub relation_type: i64,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CfFile {
    pub id: i64,
    pub mod_id: i64,
    pub display_name: Option<String>,
    pub file_name: String,
    pub release_type: i64,
    pub file_date: Option<String>,
    pub download_url: Option<String>,
    pub dependencies: Option<Vec<CfDependency>>,
    pub modules: Option<Vec<CfModule>>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CfFingerprintMatch {
    pub id: i64,
    pub file: CfFile,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct CfFingerprintResult {
    pub exact_matches: Vec<CfFingerprintMatch>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CfFeatured {
    pub featured: Vec<CfMod>,
    pub popular: Vec<CfMod>,
    pub recently_updated: Vec<CfMod>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct CfVersionType {
    pub id: i64,
    pub name: String,
}

/// A newer version is available: a CurseForge file or a new Git tag/commit.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PendingUpdate {
    pub mod_id: i64,
    pub name: String,
    pub version_label: String,
    /// CurseForge author disabled third-party downloads.
    pub website_only: bool,
    /// Where to download a website-only update by hand.
    pub download_page: Option<String>,
    /// Short commit ID when a Git update has no version of its own to show.
    pub commit: Option<String>,
    #[serde(skip)]
    pub file: Option<CfFile>,
}

/// Result of an update check. A CurseForge failure doesn't hide Git/folder updates.
#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheck {
    pub updates: Vec<PendingUpdate>,
    pub error: Option<String>,
}

/// A required dependency that has to be downloaded from the CurseForge website.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WebsiteOnlyAddon {
    pub name: String,
    pub download_page: Option<String>,
}

/// Outcome of "Update all".
#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAllResult {
    pub updated: Vec<String>,
    pub failed: Vec<String>,
    /// Website-only updates that weren't installed.
    pub skipped: Vec<String>,
}
