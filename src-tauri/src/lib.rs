//! Tauri commands: the bridge between the window (ui/) and the addon logic (core/).

mod core;

use crate::core::manager::Manager;
use crate::core::models::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tauri::State;
use tokio::sync::Mutex;

type Shared<'a> = State<'a, Mutex<Manager>>;
type R<T> = Result<T, String>;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AddonView {
    #[serde(flatten)]
    addon: InstalledAddon,
    kind: AddonSource,
    version: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Overview {
    installed: Vec<AddonView>,
    untracked: Vec<String>,
    settings: Settings,
    auto_updates: bool,
    has_key: bool,
    addons_path_exists: bool,
    app_version: String,
}

#[tauri::command]
async fn overview(app: tauri::AppHandle, m: Shared<'_>) -> R<Overview> {
    let m = m.lock().await;
    Ok(Overview {
        installed: m.state.installed.iter().cloned()
            .map(|a| AddonView { kind: a.kind(), version: a.version(), addon: a }).collect(),
        untracked: m.untracked(),
        settings: m.state.settings.clone(),
        auto_updates: m.state.settings.auto_updates(),
        has_key: !m.api_key().is_empty(),
        addons_path_exists: m.addons_dir().is_dir(),
        app_version: app.package_info().version.to_string(),
    })
}

#[tauri::command]
async fn scan(m: Shared<'_>) -> R<usize> {
    m.lock().await.scan().await
}

#[tauri::command]
async fn check_updates(m: Shared<'_>) -> R<UpdateCheck> {
    Ok(m.lock().await.check_updates().await)
}

#[tauri::command]
async fn update(m: Shared<'_>, mod_id: i64) -> R<()> {
    m.lock().await.update(mod_id).await
}

#[tauri::command]
async fn update_all(m: Shared<'_>) -> R<UpdateAllResult> {
    Ok(m.lock().await.update_all().await)
}

#[tauri::command]
async fn install_mod(m: Shared<'_>, mod_id: i64) -> R<Vec<WebsiteOnlyAddon>> {
    m.lock().await.install_mod(mod_id).await
}

#[tauri::command]
async fn install_git(m: Shared<'_>, url: String) -> R<()> {
    m.lock().await.install_git(&url).await
}

#[tauri::command]
async fn install_folder(m: Shared<'_>, path: String) -> R<()> {
    m.lock().await.install_folder(&path)
}

#[tauri::command]
async fn install_zips(m: Shared<'_>, paths: Vec<String>) -> R<()> {
    let mut m = m.lock().await;
    for p in paths {
        m.install_zip(&p).await?;
    }
    Ok(())
}

#[tauri::command]
async fn reinstall(m: Shared<'_>, mod_id: i64) -> R<()> {
    m.lock().await.reinstall(mod_id).await
}

#[tauri::command]
async fn uninstall(m: Shared<'_>, mod_id: i64) -> R<()> {
    m.lock().await.uninstall(mod_id)
}

#[tauri::command]
async fn set_ignored(m: Shared<'_>, mod_id: i64, ignored: bool) -> R<()> {
    m.lock().await.set_ignored(mod_id, ignored);
    Ok(())
}

/// Next batch of popular / recently updated Forever addons (CurseForge search isn't
/// available to personal API keys).
#[tauri::command]
async fn browse(m: Shared<'_>, excluding: Vec<i64>) -> R<Vec<CfMod>> {
    let (api, gvt, rt) = {
        let m = m.lock().await;
        (m.api(), m.state.settings.game_version_type_id, m.state.settings.release_type)
    };
    let page = api.featured(gvt, &excluding).await?;
    let mut seen: HashSet<i64> = excluding.into_iter().collect();
    Ok(page.featured.into_iter().chain(page.popular).chain(page.recently_updated)
        .filter(|x| seen.insert(x.id))
        .map(|mut x| { x.annotate(gvt, rt); x })
        .collect())
}

#[tauri::command]
async fn lookup(m: Shared<'_>, mod_id: i64) -> R<Option<CfMod>> {
    let (api, gvt, rt) = {
        let m = m.lock().await;
        (m.api(), m.state.settings.game_version_type_id, m.state.settings.release_type)
    };
    Ok(api.mods(&[mod_id]).await?.into_iter().next().map(|mut x| { x.annotate(gvt, rt); x }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SettingsInput {
    addons_path: String,
    release_type: i64,
    game_version_type_id: i64,
    auto_updates: bool,
}

#[tauri::command]
async fn save_settings(m: Shared<'_>, settings: SettingsInput) -> R<()> {
    let mut m = m.lock().await;
    let s = &mut m.state.settings;
    s.addons_path = settings.addons_path.trim().to_string();
    s.release_type = settings.release_type.clamp(1, 3);
    s.game_version_type_id = settings.game_version_type_id;
    s.auto_update = Some(settings.auto_updates);
    m.save();
    Ok(())
}

#[tauri::command]
async fn set_key(m: Shared<'_>, key: String) -> R<()> {
    m.lock().await.set_api_key(&key)
}

/// Checks a key with CurseForge without revealing it: the given one (before saving it),
/// or the saved one when `key` is omitted.
#[tauri::command]
async fn test_key(m: Shared<'_>, key: Option<String>) -> R<String> {
    let api = match key {
        Some(k) => crate::core::curseforge::Api::new(k), // Api::new cleans the key
        None => m.lock().await.api(),
    };
    Ok(api.diagnose_key().await)
}

#[tauri::command]
async fn version_types(m: Shared<'_>) -> R<Vec<CfVersionType>> {
    let api = m.lock().await.api();
    let mut v = api.version_types().await?;
    v.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(v)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(Mutex::new(Manager::load()))
        .invoke_handler(tauri::generate_handler![
            overview, scan, check_updates, update, update_all, install_mod, install_git, install_folder,
            install_zips, reinstall, uninstall, set_ignored, browse, lookup, save_settings, set_key,
            test_key, version_types
        ])
        .run(tauri::generate_context!())
        .expect("error while running Forever AddOn Manager");
}
