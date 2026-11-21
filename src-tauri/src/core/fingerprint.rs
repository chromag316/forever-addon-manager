//! CurseForge fingerprinting: MurmurHash2 (seed 1) over each file with whitespace bytes removed,
//! combined into one fingerprint per addon folder.

use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn is_ws(b: u8) -> bool {
    b == 9 || b == 10 || b == 13 || b == 32
}

pub fn murmur2(data: &[u8], normalize: bool) -> u32 {
    const M: u32 = 0x5bd1_e995;
    let len = if normalize { data.iter().filter(|b| !is_ws(**b)).count() } else { data.len() } as u32;
    let mut h: u32 = 1 ^ len;
    let mut k: u32 = 0;
    let mut shift = 0u32;
    for &b in data {
        if normalize && is_ws(b) {
            continue;
        }
        k |= (b as u32) << shift;
        shift += 8;
        if shift == 32 {
            let mut kk = k.wrapping_mul(M);
            kk ^= kk >> 24;
            kk = kk.wrapping_mul(M);
            h = h.wrapping_mul(M) ^ kk;
            k = 0;
            shift = 0;
        }
    }
    if shift > 0 {
        h = (h ^ k).wrapping_mul(M);
    }
    h ^= h >> 13;
    h = h.wrapping_mul(M);
    h ^ (h >> 15)
}

const FLAVORS: [&str; 11] =
    ["bcc", "tbc", "mainline", "classic", "vanilla", "wrath", "wotlkc", "cata", "mists", "forever", "camelot"];

/// `Folder/Folder.toc` or `Folder/Folder_<flavor>.toc` (also `-` or `|` as separator).
fn is_root_toc(folder: &str, file: &str) -> bool {
    let (f, n) = (folder.to_lowercase(), file.to_lowercase());
    let Some(stem) = n.strip_suffix(".toc") else { return false };
    if stem == f {
        return true;
    }
    let Some(rest) = stem.strip_prefix(&f) else { return false };
    let mut chars = rest.chars();
    matches!(chars.next(), Some('-' | '_' | '|')) && FLAVORS.contains(&chars.as_str())
}

fn regexes() -> &'static [Regex; 4] {
    static R: OnceLock<[Regex; 4]> = OnceLock::new();
    R.get_or_init(|| {
        [
            Regex::new(r#"(?i)<(?:Include|Script)\s+file=["']([^"']+)["']\s*/>"#).unwrap(),
            Regex::new(r"(?im)^\s*([^\r\n]+?\.(?:xml|lua))\s*$").unwrap(),
            Regex::new(r"(?s)<!--.*?-->").unwrap(),
            Regex::new(r"(?m)\s*#.*$").unwrap(),
        ]
    })
}

/// Resolve a relative path case-insensitively (WoW and CurseForge ignore case).
fn resolve(dir: &Path, rel: &str) -> Option<PathBuf> {
    let mut current = dir.to_path_buf();
    for comp in rel.split(['/', '\\']).filter(|c| !c.is_empty() && *c != ".") {
        let direct = current.join(comp);
        if direct.exists() {
            current = direct;
            continue;
        }
        let lower = comp.to_lowercase();
        let hit = fs::read_dir(&current).ok()?.flatten().find(|e| e.file_name().to_string_lossy().to_lowercase() == lower)?;
        current = hit.path();
    }
    Some(current)
}

/// The files CurseForge counts as part of an addon: its .toc files, Bindings.xml, and
/// everything they include (transitively).
pub fn matching_files(folder: &Path) -> Vec<PathBuf> {
    let name = folder.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let Ok(entries) = fs::read_dir(folder) else { return vec![] };
    let mut roots: Vec<PathBuf> = entries
        .flatten()
        .filter(|e| {
            let f = e.file_name().to_string_lossy().to_string();
            is_root_toc(&name, &f) || f.eq_ignore_ascii_case("bindings.xml")
        })
        .map(|e| e.path())
        .collect();
    roots.sort_by_key(|p| p.to_string_lossy().to_lowercase());

    let [xml_inc, toc_inc, xml_comment, toc_comment] = regexes();
    let mut seen = HashSet::new();
    let mut result = vec![];
    let mut stack: Vec<PathBuf> = roots.into_iter().rev().collect();
    while let Some(path) = stack.pop() {
        if !path.is_file() || !seen.insert(path.to_string_lossy().to_lowercase()) {
            continue;
        }
        result.push(path.clone());
        let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        if ext != "xml" && ext != "toc" {
            continue;
        }
        let Ok(bytes) = fs::read(&path) else { continue };
        let raw = String::from_utf8_lossy(&bytes);
        let (text, re) = if ext == "xml" {
            (xml_comment.replace_all(&raw, "").to_string(), xml_inc)
        } else {
            (toc_comment.replace_all(&raw, "").to_string(), toc_inc)
        };
        let dir = path.parent().unwrap_or(folder).to_path_buf();
        let mut includes = vec![];
        for cap in re.captures_iter(&text) {
            let inc = cap[1].trim();
            if inc.contains("..") {
                continue;
            }
            if let Some(p) = resolve(&dir, inc) {
                includes.push(p);
            }
        }
        // Process includes in file order (depth-first, like the original).
        stack.extend(includes.into_iter().rev());
    }
    result
}

/// Fingerprint for one addon folder, or None if it has no .toc.
pub fn fingerprint(folder: &Path) -> Option<u32> {
    let files = matching_files(folder);
    if files.is_empty() {
        return None;
    }
    let mut hashes: Vec<u32> = files.iter().filter_map(|f| fs::read(f).ok()).map(|d| murmur2(&d, true)).collect();
    hashes.sort_unstable();
    let joined: String = hashes.iter().map(|h| h.to_string()).collect();
    Some(murmur2(joined.as_bytes(), true))
}
