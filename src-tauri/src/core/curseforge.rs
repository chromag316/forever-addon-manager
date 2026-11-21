//! Client for the official CurseForge Core API (https://docs.curseforge.com).

use super::models::*;
use futures_util::StreamExt;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::json;
use std::path::Path;
use tokio::io::AsyncWriteExt;

pub const WOW_GAME_ID: i64 = 1;
const BASE: &str = "https://api.curseforge.com";

#[derive(Deserialize)]
struct Envelope<T> {
    data: T,
}

#[derive(Clone)]
pub struct Api {
    key: String,
    http: reqwest::Client,
}

pub fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("ForeverAddOnManager")
        .connect_timeout(std::time::Duration::from_secs(20))
        .build()
        .expect("http client")
}

impl Api {
    pub fn new(key: String) -> Self {
        Api { key, http: http_client() }
    }

    async fn send<T: DeserializeOwned>(&self, req: reqwest::RequestBuilder) -> Result<T, String> {
        if self.key.is_empty() {
            return Err("No CurseForge API key set. Get one at console.curseforge.com and add it in Settings.".into());
        }
        let resp = req
            .header("x-api-key", &self.key)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| if e.is_connect() || e.is_timeout() {
                "Couldn't connect to CurseForge. Check your internet connection, or allow Forever AddOn Manager in your firewall.".to_string()
            } else {
                format!("Couldn't reach CurseForge: {e}")
            })?;
        let status = resp.status();
        let body = resp.text().await.map_err(|e| e.to_string())?;
        if !status.is_success() {
            let snippet: String = body.chars().take(200).collect();
            return Err(format!("CurseForge refused the request (HTTP {}){}", status.as_u16(),
                               if snippet.is_empty() { String::new() } else { format!(": {snippet}") }));
        }
        serde_json::from_str::<Envelope<T>>(&body).map(|e| e.data).map_err(|e| format!("Unexpected reply from CurseForge: {e}"))
    }

    fn get(&self, path: &str) -> reqwest::RequestBuilder {
        self.http.get(format!("{BASE}{path}"))
    }
    fn post(&self, path: &str, body: serde_json::Value) -> reqwest::RequestBuilder {
        self.http.post(format!("{BASE}{path}")).json(&body)
    }

    pub async fn version_types(&self) -> Result<Vec<CfVersionType>, String> {
        self.send(self.get(&format!("/v1/games/{WOW_GAME_ID}/version-types"))).await
    }

    /// Featured / popular / recently updated addons; pass seen ids to get the next batch.
    /// (Personal API keys can't use /mods/search.)
    pub async fn featured(&self, gvt: i64, excluding: &[i64]) -> Result<CfFeatured, String> {
        self.send(self.post("/v1/mods/featured",
            json!({ "gameId": WOW_GAME_ID, "excludedModIds": excluding, "gameVersionTypeId": gvt }))).await
    }

    pub async fn mods(&self, ids: &[i64]) -> Result<Vec<CfMod>, String> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        self.send(self.post("/v1/mods", json!({ "modIds": ids }))).await
    }

    pub async fn files(&self, mod_id: i64, gvt: i64) -> Result<Vec<CfFile>, String> {
        self.send(self.get(&format!("/v1/mods/{mod_id}/files?gameVersionTypeId={gvt}&pageSize=50"))).await
    }

    pub async fn file(&self, mod_id: i64, file_id: i64) -> Result<CfFile, String> {
        self.send(self.get(&format!("/v1/mods/{mod_id}/files/{file_id}"))).await
    }

    pub async fn match_fingerprints(&self, fps: &[u32]) -> Result<CfFingerprintResult, String> {
        if fps.is_empty() {
            return Ok(CfFingerprintResult::default());
        }
        self.send(self.post(&format!("/v1/fingerprints/{WOW_GAME_ID}"), json!({ "fingerprints": fps }))).await
    }

    /// Newest file for this flavour at or below the allowed release type.
    pub async fn best_file(&self, m: &CfMod, gvt: i64, release_type: i64) -> Result<Option<CfFile>, String> {
        let indexed = m.latest_files_indexes.iter().flatten()
            .filter(|i| i.game_version_type_id == Some(gvt) && i.release_type <= release_type)
            .max_by_key(|i| i.file_id);
        if let Some(idx) = indexed {
            if let Some(f) = m.latest_files.iter().flatten().find(|f| f.id == idx.file_id) {
                return Ok(Some(f.clone()));
            }
            return self.file(m.id, idx.file_id).await.map(Some);
        }
        Ok(self.files(m.id, gvt).await?.into_iter().filter(|f| f.release_type <= release_type).max_by_key(|f| f.id))
    }

    /// Checks the key without revealing it.
    pub async fn diagnose_key(&self) -> String {
        let mut notes = vec![format!("Key length: {} characters.", self.key.len())];
        if self.key.is_empty() {
            return "No key saved.".into();
        }
        if !(self.key.starts_with("$2a$") || self.key.starts_with("$2b$") || self.key.starts_with("$2y$")) {
            notes.push("It doesn't look like a CurseForge key (those start with \"$2a$10$\" and are about 60 characters).".into());
        }
        match self.get(&format!("/v1/games/{WOW_GAME_ID}")).header("x-api-key", &self.key).send().await {
            Ok(r) if r.status().is_success() => notes.push("✅ CurseForge accepted the key.".into()),
            Ok(r) => notes.push(format!("❌ CurseForge answered HTTP {}", r.status().as_u16())),
            Err(e) => notes.push(format!("❌ Network error: {e}")),
        }
        notes.join("\n")
    }
}

/// Streams a download to `dest` (addon zips can be hundreds of MB).
pub async fn download(http: &reqwest::Client, url: &str, dest: &Path) -> Result<(), String> {
    let resp = http.get(url).send().await.map_err(|e| format!("Download failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Download failed (HTTP {})", resp.status().as_u16()));
    }
    let mut file = tokio::fs::File::create(dest).await.map_err(|e| e.to_string())?;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        file.write_all(&chunk.map_err(|e| format!("Download interrupted: {e}"))?).await.map_err(|e| e.to_string())?;
    }
    file.flush().await.map_err(|e| e.to_string())
}
