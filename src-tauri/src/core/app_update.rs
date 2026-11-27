//! Looks for a newer release of this app on GitHub.

use super::models::AppUpdate;

pub const RELEASES: &str = "https://github.com/chromag316/forever-addon-manager/releases";

/// Newest release, if it's newer than `current`. Reads the tag from the redirect of
/// /releases/latest (a web page, so GitHub's 60-requests-per-hour API limit doesn't apply).
pub async fn check(current: &str) -> Result<Option<AppUpdate>, String> {
    let http = reqwest::Client::builder()
        .user_agent("ForeverAddOnManager")
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(std::time::Duration::from_secs(20))
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = http.get(format!("{RELEASES}/latest")).send().await
        .map_err(|e| format!("Couldn't check for a new app version: {e}"))?;
    let location = resp.headers().get(reqwest::header::LOCATION).and_then(|v| v.to_str().ok()).unwrap_or_default();
    let Some(tag) = tag_from_location(location) else { return Ok(None) };
    let version = tag.trim_start_matches('v').to_string();
    Ok(is_newer(&version, current).then(|| AppUpdate { url: format!("{RELEASES}/tag/{tag}"), version }))
}

/// "…/releases/tag/v1.2.0" → "v1.2.0".
pub fn tag_from_location(location: &str) -> Option<String> {
    let (_, tag) = location.rsplit_once("/releases/tag/")?;
    let tag = tag.split(['?', '#']).next()?.trim_end_matches('/');
    (!tag.is_empty() && !tag.contains('/')).then(|| tag.to_string())
}

/// Compares dotted versions numerically ("1.10.0" > "1.9.2"). A pre-release such as
/// "1.1.0-beta" counts as newer only than versions below 1.1.0.
pub fn is_newer(candidate: &str, current: &str) -> bool {
    fn parts(v: &str) -> Option<(Vec<u64>, bool)> {
        let v = v.trim().trim_start_matches('v');
        let (core, pre) = match v.split_once(['-', '+']) {
            Some((c, rest)) => (c, !rest.is_empty() && v.as_bytes()[c.len()] == b'-'),
            None => (v, false),
        };
        let mut nums: Vec<u64> = core.split('.').map(|p| p.parse().ok()).collect::<Option<_>>()?;
        while nums.len() < 3 {
            nums.push(0);
        }
        Some((nums, pre))
    }
    let (Some((a, a_pre)), Some((b, b_pre))) = (parts(candidate), parts(current)) else { return false };
    match a.cmp(&b) {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Less => false,
        std::cmp::Ordering::Equal => !a_pre && b_pre, // 1.1.0 is newer than 1.1.0-beta
    }
}
