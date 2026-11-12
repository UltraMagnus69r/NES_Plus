//! API keys live in the OS config directory, mode 0600 where the OS allows it.
//! They are never written into a project file or an exported game.

use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyFile {
    #[serde(default = "default_openai")]
    pub assistant_provider: String,
    #[serde(default)]
    pub assistant_base_url: String,
    #[serde(default = "default_assistant_model")]
    pub assistant_model: String,
    #[serde(default)]
    pub assistant_key: String,
    #[serde(default = "default_openai")]
    pub image_provider: String,
    #[serde(default)]
    pub image_base_url: String,
    #[serde(default = "default_image_model")]
    pub image_model: String,
    #[serde(default)]
    pub image_key: String,
}

fn default_openai() -> String {
    "openai".into()
}
fn default_assistant_model() -> String {
    "gpt-4.1-mini".into()
}
fn default_image_model() -> String {
    "gpt-image-1".into()
}

impl Default for KeyFile {
    fn default() -> Self {
        Self {
            assistant_provider: default_openai(),
            assistant_base_url: String::new(),
            assistant_model: default_assistant_model(),
            assistant_key: String::new(),
            image_provider: default_openai(),
            image_base_url: String::new(),
            image_model: default_image_model(),
            image_key: String::new(),
        }
    }
}

impl KeyFile {
    pub fn suggested_assistant_model(&self) -> &'static str {
        match self.assistant_provider.as_str() {
            "anthropic" => "claude-sonnet-4-5",
            _ => "gpt-4.1-mini",
        }
    }

    pub fn path() -> Option<PathBuf> {
        directories::ProjectDirs::from("dev", "Nesticles", "Nesticles")
            .map(|dirs| dirs.config_dir().join("keys.json"))
    }

    pub fn load() -> Self {
        let Some(path) = Self::path() else {
            return Self::default();
        };
        let Ok(text) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        serde_json::from_str(&text).unwrap_or_default()
    }

    pub fn save(&self) -> Result<PathBuf, String> {
        let path = Self::path().ok_or("could not resolve the OS config directory")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        write_private(&path, text.as_bytes())?;
        Ok(path)
    }
}

pub fn write_private(path: &std::path::Path, bytes: &[u8]) -> Result<(), String> {
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|e| e.to_string())?;
    file.write_all(bytes).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn private_file_is_mode_0600() {
        let path = std::env::temp_dir().join(format!("nesticles-keys-test-{}", std::process::id()));
        write_private(&path, b"{\"assistant_key\":\"secret\"}").unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        let _ = std::fs::remove_file(&path);
        assert_eq!(mode, 0o600);
    }
}
