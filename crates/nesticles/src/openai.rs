//! OpenAI calls for UnMaker Lab.
//!
//! The key is held in memory, in the process environment, or in the Secret Service.
//! It is not written to the project, a package, a knowledge file, or a log line.
//! A connection test is a real HTTPS request. An empty key does not count as connected.

use base64::Engine;
use nesticles_core::guard::redact_secrets;
use nesticles_core::pipeline::{address_blocked, allow_reference_url, classify_api, ApiStatus};
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

pub fn fetch_reference(url: &str) -> Result<Vec<u8>, String> {
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
        let response = client
            .get(&current)
            .send()
            .map_err(|_| "Network unavailable".to_string())?;
        let status = response.status();
        if status.is_redirection() {
            let next = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or("The redirect had no location.")?
                .to_string();
            let next = if next.starts_with("https://") {
                next
            } else {
                return Err("A redirect left HTTPS. The fetch was stopped.".into());
            };
            allow_reference_url(&next)?;
            current = next;
            continue;
        }
        if !status.is_success() {
            return Err(format!("The reference returned HTTP {}.", status.as_u16()));
        }
        let kind = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !kind.starts_with("image/")
            && !kind.starts_with("text/html")
            && !kind.starts_with("text/plain")
        {
            return Err("That content type is not a page or an image.".into());
        }
        let bytes = response
            .bytes()
            .map_err(|_| "The download was interrupted.".to_string())?;
        if bytes.len() > 1_000_000 {
            return Err("The reference is larger than 1 MB. It was not kept.".into());
        }
        return Ok(bytes.to_vec());
    }
    Err("Too many redirects. The fetch was stopped.".into())
}

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
    fn private_urls_are_not_fetched() {
        assert!(fetch_reference("https://127.0.0.1/tile").is_err());
        assert!(fetch_reference("http://example.com/a").is_err());
    }
}
