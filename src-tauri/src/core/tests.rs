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
