use super::fingerprint::{fingerprint, matching_files, murmur2};
use super::sources::{addon_folders, metadata, unzip, GitSpec};
use std::collections::HashSet;
use std::path::PathBuf;

/// TestAddon/{TestAddon.toc, main.lua, notes.txt, sub/x.xml, sub/y.lua}; the .toc includes
/// `sub\x.xml` (backslash) and x.xml includes y.lua plus a commented-out file.
const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/test-addon.zip");
/// Fingerprint of the fixture from the earlier implementation, which matched CurseForge's own
/// fingerprints for real addons.
const FIXTURE_FINGERPRINT: u32 = 1_076_336_427;

fn extract() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fam-test-{}-{:?}", std::process::id(), std::thread::current().id()));
    let _ = std::fs::remove_dir_all(&dir);
    unzip(&PathBuf::from(FIXTURE), &dir).expect("unzip");
    dir
}

#[test]
fn unzip_and_fingerprint() {
    let dir = extract();
    let addon = dir.join("TestAddon");
    let files: HashSet<String> = matching_files(&addon).iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string()).collect();
    let expected: HashSet<String> = ["TestAddon.toc", "main.lua", "x.xml", "y.lua"].iter().map(|s| s.to_string()).collect();
    assert_eq!(files, expected);
    assert_eq!(fingerprint(&addon), Some(FIXTURE_FINGERPRINT));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn layout_and_metadata() {
    let dir = extract();
    let found = addon_folders(&dir, 0);
    assert_eq!(found.iter().map(|(_, n)| n.as_str()).collect::<Vec<_>>(), vec!["TestAddon"]);
    let (title, version, _) = metadata(&dir.join("TestAddon"));
    assert_eq!(title.as_deref(), Some("Test Addon"));
    assert_eq!(version.as_deref(), Some("1.2.3"));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn murmur_ignores_whitespace() {
    assert_eq!(murmur2(b"a b\r\nc\t", true), murmur2(b"abc", true));
    assert_ne!(murmur2(b"abc", true), murmur2(b"abd", true));
}

#[test]
fn git_spec_parsing() {
    let s = GitSpec::parse("Stanzilla/AdvancedInterfaceOptions").unwrap();
    assert_eq!(s.owner.as_deref(), Some("Stanzilla"));
    assert_eq!(s.url, "https://github.com/Stanzilla/AdvancedInterfaceOptions.git");
    let b = GitSpec::parse("https://github.com/user/addon/tree/dev").unwrap();
    assert_eq!((b.repo.as_str(), b.branch.as_deref()), ("addon", Some("dev")));
    let o = GitSpec::parse("https://gitlab.com/someone/thing.git").unwrap();
    assert_eq!((o.owner, o.repo.as_str()), (None, "thing"));
    assert!(GitSpec::parse("not a url").is_err());
}

#[test]
fn local_ids_match_earlier_app() {
    // Ids the earlier macOS app stored for these Git addons; they must not change.
    let spec = GitSpec::parse("https://github.com/chromag316/Forged_ForeverAuctionHouse.git").unwrap();
    assert_eq!(super::manager::local_id(&spec.url), -618_085_760);
}

/// Run with `cargo test -- --ignored` on a machine with WoW Forever installed: compares
/// against the fingerprints CurseForge itself stored for these addons.
#[test]
#[ignore]
fn real_addons_match_curseforge() {
    let base = PathBuf::from(super::platform::default_addons_path());
    let known = [("EZFishing", 2_125_812_604u32), ("Spoken", 1_734_559_835), ("SpokenBossesAudio", 3_175_957_937),
        ("SpokenContributions", 2_820_630_352), ("SpokenQuestsAudioAlliance", 2_699_492_522),
        ("SpokenQuestsAudioGossip", 3_566_748_306), ("SpokenQuestsAudioShared", 2_948_392_232),
        ("SpokenReadablesAudio", 615_218_563), ("Spoken_Bosses", 1_238_064_593), ("Spoken_Places", 3_426_682_842),
        ("Spoken_Quests", 845_085_455), ("Spoken_Readables", 2_780_527_273)];
    for (folder, fp) in known {
        assert_eq!(fingerprint(&base.join(folder)), Some(fp), "{folder}");
    }
}

/// Live check against CurseForge with the saved API key: `cargo test -- --ignored network`.
#[test]
#[ignore]
fn network_smoke() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let m = super::manager::Manager::load();
        let api = m.api();
        let types = tokio::time::timeout(std::time::Duration::from_secs(20), api.version_types()).await
            .expect("timed out reaching CurseForge (firewall?)").expect("api error");
        assert!(types.iter().any(|t| t.id == 88568), "WoW Forever version type missing");
        let page = api.featured(88568, &[]).await.expect("featured");
        assert!(!page.popular.is_empty());
    });
}

#[test]
fn keys_are_cleaned_of_pasted_junk() {
    use super::curseforge::clean_key;
    let key = "$2a$10$abcDEF123";
    for pasted in [" $2a$10$abcDEF123\n", "\"$2a$10$abcDEF123\"", "$2a$10$abc\u{200B}DEF123", "\u{FEFF}$2a$10$abcDEF123\u{00A0}"] {
        assert_eq!(clean_key(pasted), key, "{pasted:?}");
    }
}

fn cf_mod(allow: serde_json::Value, download_url: serde_json::Value) -> super::models::CfMod {
    serde_json::from_value(serde_json::json!({
        "id": 61284, "name": "Details! Damage Meter",
        "links": { "websiteUrl": "https://www.curseforge.com/wow/addons/details" },
        "allowModDistribution": allow,
        "latestFiles": [{ "id": 200, "modId": 61284, "fileName": "d.zip", "releaseType": 1, "downloadUrl": download_url }],
        "latestFilesIndexes": [
            { "fileId": 200, "releaseType": 1, "gameVersionTypeId": 88568 },
            { "fileId": 300, "releaseType": 3, "gameVersionTypeId": 88568 },
            { "fileId": 400, "releaseType": 1, "gameVersionTypeId": 517 }
        ]
    })).expect("mod json")
}

#[test]
fn website_only_rule() {
    let url = serde_json::json!("https://edge.forgecdn.net/files/0/200/d.zip");
    // Author turned off third-party downloads.
    let mut m = cf_mod(serde_json::json!(false), serde_json::Value::Null);
    m.annotate(88568, 1);
    assert!(m.website_only);
    assert_eq!(m.target_file_id, Some(200), "release file for Forever, not alpha or another flavour");
    assert_eq!(m.download_page(m.target_file_id).as_deref(), Some("https://www.curseforge.com/wow/addons/details/download/200"));
    // Flag missing, but the file has no download URL.
    let mut m = cf_mod(serde_json::Value::Null, serde_json::Value::Null);
    m.annotate(88568, 1);
    assert!(m.website_only);
    // Allowed and downloadable.
    let mut m = cf_mod(serde_json::json!(true), url.clone());
    m.annotate(88568, 1);
    assert!(!m.website_only);
    // Alpha channel picks the newer alpha file.
    let mut m = cf_mod(serde_json::json!(true), url);
    m.annotate(88568, 3);
    assert_eq!(m.target_file_id, Some(300));
}

#[test]
fn toc_version_field() {
    use super::sources::toc_field;
    let toc = "\u{FEFF}## Interface: 16001\n## Title: |cfff07d30Forged|r Forever Auction House\n## Version: 0.9.0\n";
    assert_eq!(toc_field(toc, "Version").as_deref(), Some("0.9.0"));
    assert_eq!(toc_field(toc, "Title").as_deref(), Some("Forged Forever Auction House"));
    assert_eq!(toc_field("## Version: @project-version@", "Version"), None);
    assert_eq!(toc_field("## Interface: 1", "Version"), None);
}

#[test]
fn app_version_compare() {
    use super::app_update::{is_newer, tag_from_location};
    assert!(is_newer("1.1.0", "1.0.0"));
    assert!(is_newer("v1.10.0", "1.9.2"));
    assert!(is_newer("2", "1.9.9"));
    assert!(is_newer("1.1.0", "1.1.0-beta"));
    assert!(!is_newer("1.0.0", "1.0.0"));
    assert!(!is_newer("1.1.0-beta", "1.1.0"));
    assert!(!is_newer("0.9.0", "1.0.0"));
    assert!(!is_newer("nightly", "1.0.0"));
    assert_eq!(tag_from_location("https://github.com/chromag316/forever-addon-manager/releases/tag/v1.2.0").as_deref(), Some("v1.2.0"));
    assert_eq!(tag_from_location("https://github.com/chromag316/forever-addon-manager/releases"), None);
}

/// Without a CurseForge key (or for an addon CurseForge doesn't know), a zip install is
/// tracked as a local addon that can be reinstalled from the same zip.
#[tokio::test]
async fn zip_install_is_tracked_locally() {
    if std::env::var("CURSEFORGE_API_KEY").is_ok_and(|k| !k.is_empty()) {
        return; // this test is about the no-key path
    }
    let root = std::env::temp_dir().join(format!("fam-zip-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let mut m = super::manager::Manager::at(&root);
    m.install_zip(FIXTURE).await.expect("install");
    m.install_zip(FIXTURE).await.expect("install again");
    assert_eq!(m.state.installed.len(), 1, "installing the same addon again replaces it");
    let a = &m.state.installed[0];
    assert_eq!(a.kind(), super::models::AddonSource::Folder);
    assert_eq!(a.source_url.as_deref(), Some(FIXTURE));
    assert_eq!(a.folders, vec!["TestAddon".to_string()]);
    assert!(root.join("AddOns/TestAddon/TestAddon.toc").is_file());
    m.reinstall(a.mod_id).await.expect("reinstall from zip");
    assert!(m.untracked().is_empty());
    assert!(m.install_zip(&root.join("missing.zip").to_string_lossy()).await.unwrap_err().contains("doesn't exist"));
    let _ = std::fs::remove_dir_all(&root);
}
