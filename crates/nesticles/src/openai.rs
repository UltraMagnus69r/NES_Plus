//! OpenAI calls for UnMaker Lab.
//!
//! The key is held in memory, in the process environment, or in the Secret Service.
//! It is not written to the project, a package, a knowledge file, or a log line.
//! A connection test is a real HTTPS request. An empty key does not count as connected.

use base64::Engine;
use nesticles_core::guard::redact_secrets;
use nesticles_core::pipeline::{address_blocked, allow_reference_url, classify_api, ApiStatus};
use nesticles_core::reference::{http_status_error, IMAGE_BYTE_LIMIT, PAGE_BYTE_LIMIT};
use serde_json::{json, Value};
use std::io::Write;
use std::net::ToSocketAddrs;
use std::process::{Command, Stdio};
use std::time::Duration;

pub const DEFAULT_VISION: &str = "gpt-4.1-mini";
pub const DEFAULT_IMAGE: &str = "gpt-image-1";
pub const DEFAULT_BEHAVIOR: &str = "gpt-4.1-mini";

pub fn redact(text: &str, key: &str) -> String {
    redact_secrets(text, &[key])
}

pub fn key_from_env() -> Option<String> {
    std::env::var("NESTICLES_OPENAI_API_KEY")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn load_keyring() -> Option<String> {
    let output = Command::new("timeout")
        .args(["2", "secret-tool", "lookup", "service", "nesticles", "account", "openai"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

/// Store the key in the Secret Service. Failure leaves the key unsaved.
pub fn store_keyring(key: &str) -> Result<(), String> {
    if key.trim().is_empty() {
        return Err("There is no key to save.".into());
    }
    let mut child = Command::new("secret-tool")
        .args([
            "store",
            "--label",
            "Nesticles OpenAI",
            "service",
            "nesticles",
            "account",
            "openai",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| {
            "Secret Service is not available. The key was not written to a file.".to_string()
        })?;
    if let Some(stdin) = child.stdin.as_mut() {
        let _ = stdin.write_all(key.as_bytes());
    }
    let output = child
        .wait_with_output()
        .map_err(|_| "Secret Service did not answer. The key was not written.".to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err("Secret Service refused the key. It was not written to a file.".into())
    }
}

pub fn clear_keyring() -> Result<(), String> {
    let status = Command::new("secret-tool")
        .args(["clear", "service", "nesticles", "account", "openai"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|_| "Secret Service is not available.".to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("The key could not be removed from Secret Service.".into())
    }
}

pub fn test_connection(key: &str, model: &str) -> ApiStatus {
    if key.trim().is_empty() {
        return ApiStatus::InvalidCredentials;
    }
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
    {
        Ok(client) => client,
        Err(_) => return ApiStatus::Unavailable,
    };
    match client
        .get("https://api.openai.com/v1/models")
        .bearer_auth(key)
        .send()
    {
        Ok(response) => {
            let status = response.status().as_u16();
            let body = response.text().unwrap_or_default();
            let body = redact(&body, key);
            let class = classify_api(Some(status), &body, false, false);
            if class == ApiStatus::Connected && !model.trim().is_empty() && !body.contains(model) {
                ApiStatus::ModelUnavailable
            } else {
                class
            }
        }
        Err(err) => {
            if err.is_timeout() {
                ApiStatus::Timeout
            } else {
                ApiStatus::Network
            }
        }
    }
}

pub fn vision_request(model: &str, png: &[u8]) -> Value {
    let encoded = base64::engine::general_purpose::STANDARD.encode(png);
    json!({
        "model": model,
        "response_format": {"type": "json_object"},
        "messages": [{
            "role": "user",
            "content": [
                {"type": "text", "text": "Return JSON with name, category, and tiles (array of integers). Do not include scripts."},
                {"type": "image_url", "image_url": {"url": format!("data:image/png;base64,{encoded}")}}
            ]
        }]
    })
}

pub fn parse_vision_response(body: &str) -> Result<Value, String> {
    let value: Value =
        serde_json::from_str(body).map_err(|_| "Malformed provider response".to_string())?;
    let content = value
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .ok_or("Malformed provider response")?;
    serde_json::from_str(content).map_err(|_| "Malformed provider response".into())
}

pub fn image_request(model: &str, prompt: &str) -> Value {
    json!({
        "model": model,
        "prompt": prompt,
        "size": "1024x1024",
        "n": 1
    })
}

pub fn parse_image_response(body: &str) -> Result<Vec<u8>, String> {
    let value: Value =
        serde_json::from_str(body).map_err(|_| "Malformed provider response".to_string())?;
    let encoded = value
        .pointer("/data/0/b64_json")
        .and_then(|v| v.as_str())
        .ok_or("Malformed provider response")?;
    base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| "Malformed provider response".into())
}

/// Fetch a reference page or image. HTML is returned as bytes and is not executed.
pub fn generate_image(key: &str, model: &str, prompt: &str) -> Result<Vec<u8>, String> {
    if key.trim().is_empty() {
        return Err(ApiStatus::InvalidCredentials.label().into());
    }
    if nesticles_core::image_prompt_blocked(prompt).is_some() {
        return Err("That prompt is refused because it names a commercial character. Describe an original design.".into());
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|_| ApiStatus::Unavailable.label().to_string())?;
    let response = client
        .post("https://api.openai.com/v1/images/generations")
        .bearer_auth(key)
        .json(&image_request(model, prompt))
        .send();
    let response = match response {
        Ok(response) => response,
        Err(err) => {
            return Err(if err.is_timeout() {
                ApiStatus::Timeout.label().into()
            } else {
                ApiStatus::Network.label().into()
            });
        }
    };
    let status = response.status().as_u16();
    let body = redact(&response.text().unwrap_or_default(), key);
    let class = classify_api(Some(status), &body, false, false);
    if class != ApiStatus::Connected {
        return Err(class.label().into());
    }
    parse_image_response(&body)
}

pub fn analyze_image(key: &str, model: &str, png: &[u8]) -> Result<Value, String> {
    if key.trim().is_empty() {
        return Err(ApiStatus::InvalidCredentials.label().into());
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(45))
        .build()
        .map_err(|_| ApiStatus::Unavailable.label().to_string())?;
    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(key)
        .json(&vision_request(model, png))
        .send();
    let response = match response {
        Ok(response) => response,
        Err(err) => {
            return Err(if err.is_timeout() {
                ApiStatus::Timeout.label().into()
            } else {
                ApiStatus::Network.label().into()
            });
        }
    };
    let status = response.status().as_u16();
    let body = redact(&response.text().unwrap_or_default(), key);
    let class = classify_api(Some(status), &body, false, false);
    if class != ApiStatus::Connected {
        return Err(class.label().into());
    }
    parse_vision_response(&body)
}

#[allow(dead_code)]
pub fn fetch_reference(url: &str) -> Result<Vec<u8>, String> {
    fetch_limited(url, PAGE_BYTE_LIMIT, false, &std::sync::atomic::AtomicBool::new(false))
}

pub fn fetch_reference_image(
    url: &str,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<Vec<u8>, String> {
    fetch_limited(url, IMAGE_BYTE_LIMIT, true, cancel)
}

pub fn fetch_reference_page(
    url: &str,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<Vec<u8>, String> {
    fetch_limited(url, PAGE_BYTE_LIMIT, false, cancel)
}

fn fetch_limited(
    url: &str,
    limit: usize,
    image_only: bool,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<Vec<u8>, String> {
    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
        return Err("The fetch was cancelled.".into());
    }
    allow_reference_url(url)?;
    let host = reference_host(url)?;
    let addrs = (host.as_str(), 443)
        .to_socket_addrs()
        .map_err(|_| "That host did not resolve.".to_string())?;
    let mut saw = false;
    for addr in addrs {
        saw = true;
        if address_blocked(addr.ip()) {
            return Err("That host resolved to a private address. The fetch was refused.".into());
        }
    }
    if !saw {
        return Err("That host did not resolve.".into());
    }
    let client = reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|_| "Could not start the fetch.".to_string())?;
    let mut current = url.to_string();
    for _ in 0..3 {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return Err("The fetch was cancelled.".into());
        }
        let response = client
            .get(&current)
            .send()
            .map_err(|_| "Network unavailable. Failed during Connecting.".to_string())?;
        let status = response.status();
        if status.is_redirection() {
            let next = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or("The redirect had no location.")?;
            current = follow_redirect(&current, next)?;
            continue;
        }
        if !status.is_success() {
            return Err(http_status_error("Page retrieval", status.as_u16()));
        }
        let kind = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_ascii_lowercase();
        if image_only {
            if !kind.starts_with("image/") {
                return Err(
                    "Failed during image retrieval. The response was not an image.".into(),
                );
            }
        } else if !kind.starts_with("image/")
            && !kind.starts_with("text/html")
            && !kind.starts_with("text/plain")
            && !kind.is_empty()
        {
            return Err("That content type is not a page or an image.".into());
        }
        return read_capped(response, limit, cancel);
    }
    Err("Too many redirects. The fetch was stopped.".into())
}

pub fn follow_redirect(current: &str, location: &str) -> Result<String, String> {
    let location = location.trim();
    let next = if location.starts_with("https://") {
        location.to_string()
    } else if let Some(rest) = location.strip_prefix("//") {
        format!("https://{rest}")
    } else if location.starts_with('/') {
        format!("https://{}{location}", reference_host(current)?)
    } else {
        return Err("A redirect left HTTPS. The fetch was stopped.".into());
    };
    allow_reference_url(&next)?;
    Ok(next)
}

fn read_capped(
    mut response: reqwest::blocking::Response,
    limit: usize,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<Vec<u8>, String> {
    use std::io::Read;
    if let Some(len) = response.content_length() {
        if len > limit as u64 {
            return Err("The reference is larger than the limit. It was not kept.".into());
        }
    }
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        if cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return Err("The fetch was cancelled.".into());
        }
        let n = response
            .read(&mut chunk)
            .map_err(|_| "The download was interrupted.".to_string())?;
        if n == 0 {
            break;
        }
        if buf.len() + n > limit {
            return Err("The reference is larger than the limit. It was not kept.".into());
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    Ok(buf)
}

#[allow(dead_code)]
pub fn image_urls(html: &str) -> Vec<String> {
    let mut urls = Vec::new();
    for part in html.split("src=\"").skip(1) {
        let Some(url) = part.split('"').next() else {
            continue;
        };
        if allow_reference_url(url).is_ok() && !urls.iter().any(|have: &String| have == url) {
            urls.push(url.to_string());
        }
        if urls.len() == 12 {
            break;
        }
    }
    urls
}

fn reference_host(url: &str) -> Result<String, String> {
    let rest = url.trim_start_matches("https://");
    let host_port = rest.split('/').next().unwrap_or("");
    let host = host_port.split(':').next().unwrap_or("").trim();
    if host.is_empty() {
        Err("That URL has no host.".into())
    } else {
        Ok(host.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_key_does_not_claim_a_connection() {
        assert_eq!(
            test_connection("", "gpt-4.1-mini"),
            ApiStatus::InvalidCredentials
        );
    }

    #[test]
    fn errors_do_not_keep_the_key() {
        let key = "sk-test-secret-value";
        let text = format!("provider said {key} was rejected");
        let clean = redact(&text, key);
        assert!(!clean.contains(key));
    }

    #[test]
    fn vision_and_image_payloads_parse() {
        let body = vision_request("gpt-4.1-mini", b"png");
        assert!(body.to_string().contains("tiles"));
        let raw = r#"{"choices":[{"message":{"content":"{\"name\":\"Blob\",\"tiles\":[1],\"category\":\"enemy\"}"}}]}"#;
        let parsed = parse_vision_response(raw).unwrap();
        assert_eq!(parsed["name"], "Blob");
        let image = parse_image_response(r#"{"data":[{"b64_json":"aGk="}]}"#).unwrap();
        assert_eq!(image, b"hi");
        assert!(parse_image_response("not-json").is_err());
    }

    #[test]
    fn the_old_img_scan_does_not_see_sprite_sheet_links() {
        let html = r#"<a href="https://cdn.example/enemies.png">Enemies</a><img src="https://cdn.example/logo.png">"#;
        let urls = image_urls(html);
        assert_eq!(urls, vec!["https://cdn.example/logo.png".to_string()]);
    }

    #[test]
    fn private_urls_are_not_fetched() {
        assert!(fetch_reference("https://127.0.0.1/tile").is_err());
        assert!(fetch_reference("http://example.com/a").is_err());
        assert!(fetch_reference("https://10.1.2.3/tile").is_err());
        assert!(fetch_reference("https://localhost/tile").is_err());
    }

    #[test]
    fn redirects_stay_on_public_https() {
        let next = follow_redirect("https://cdn.example/sprites/page", "/sheet.png").unwrap();
        assert_eq!(next, "https://cdn.example/sheet.png");
        let absolute =
            follow_redirect("https://cdn.example/a", "https://cdn.example/b.png").unwrap();
        assert_eq!(absolute, "https://cdn.example/b.png");
        assert!(follow_redirect("https://cdn.example/a", "http://cdn.example/b.png").is_err());
        assert!(follow_redirect("https://cdn.example/a", "https://127.0.0.1/b.png").is_err());
        assert!(follow_redirect("https://cdn.example/a", "https://192.168.0.4/b.png").is_err());
    }

    #[test]
    #[ignore = "live public page"]
    fn the_public_smb_page_yields_a_selectable_reference() {
        use std::sync::atomic::AtomicBool;
        let url = "https://www.mariouniverse.com/sprites-nes-smb/";
        let bytes = fetch_reference_page(url, &AtomicBool::new(false)).expect("page");
        let html = String::from_utf8_lossy(&bytes);
        let items = nesticles_core::reference::scan_reference_html(url, &html);
        let original = items
            .iter()
            .filter(|item| item.class == nesticles_core::reference::RefClass::Original)
            .count();
        let custom = items
            .iter()
            .filter(|item| item.class == nesticles_core::reference::RefClass::Custom)
            .count();
        assert!(original > 0, "original {original} custom {custom}");
        assert!(custom > 0, "custom art was not separated");
        assert!(items.iter().any(|item| {
            item.category == "Mario" && item.class == nesticles_core::reference::RefClass::Original
        }));
        assert!(items.iter().any(|item| item.category == "Enemies"));
        let mario = items
            .iter()
            .find(|item| {
                item.category == "Mario"
                    && item.class == nesticles_core::reference::RefClass::Original
            })
            .unwrap();
        let image = fetch_reference_image(&mario.image_url, &AtomicBool::new(false)).expect("image");
        let preview = nesticles_core::pngutil::reference_preview(&image, 64).expect("decode");
        eprintln!(
            "LIVE page_bytes={} images={} original={} custom={} preview={}x{} url={}",
            bytes.len(),
            items.len(),
            original,
            custom,
            preview.3,
            preview.4,
            mario.image_url
        );
        assert!(preview.3 > 0 && preview.4 > 0);
    }

    #[test]
    fn a_missing_key_is_still_refused_without_a_reference() {
        assert_eq!(
            test_connection("", "gpt-4.1-mini"),
            ApiStatus::InvalidCredentials
        );
    }

    #[test]
    fn r4_analyze_without_key_does_not_invent_success() {
        let err = analyze_image("", "gpt-4.1-mini", b"\x89PNG\r\n\x1a\n").unwrap_err();
        assert!(!err.contains("sk-"));
        assert_eq!(err, ApiStatus::InvalidCredentials.label());
    }

    #[test]
    fn r4_gate_helpers_match_openai_credential_policy() {
        use nesticles_core::ref_intel::{gate_ai_send, AiSendRequest};
        let blocked = gate_ai_send(&AiSendRequest {
            has_credential: false,
            user_confirmed: true,
            explicit_send: true,
            image_bytes: 32,
            notes: String::new(),
            model: DEFAULT_VISION.into(),
        });
        assert!(blocked.is_err());
        let open = gate_ai_send(&AiSendRequest {
            has_credential: true,
            user_confirmed: true,
            explicit_send: true,
            image_bytes: 32,
            notes: "compare silhouette".into(),
            model: DEFAULT_VISION.into(),
        });
        assert!(open.is_ok());
    }
}
