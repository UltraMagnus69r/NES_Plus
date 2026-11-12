//! Preview WAV bytes with a system player. The waveform is always drawn in the UI.

use std::process::{Child, Command};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn preview_wav(bytes: &[u8]) -> Result<(), String> {
    let path = temp_wav(bytes)?;
    spawn_player(&path, false).map(|_| ())
}

pub fn loop_wav(bytes: &[u8]) -> Result<Child, String> {
    let path = temp_wav(bytes)?;
    spawn_player(&path, true)
}

fn temp_wav(bytes: &[u8]) -> Result<std::path::PathBuf, String> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!("nesticles-{stamp}-{}.wav", std::process::id()));
    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(path)
}

fn spawn_player(path: &std::path::Path, looping: bool) -> Result<Child, String> {
    let path_s = path.to_string_lossy().to_string();
    #[cfg(target_os = "windows")]
    {
        let _ = looping;
        return Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "(New-Object Media.SoundPlayer '{path_s}').PlaySync()"
                ),
            ])
            .spawn()
            .map_err(|e| format!("could not play wav: {e}"));
    }
    #[cfg(not(target_os = "windows"))]
    {
        if looping {
            if let Ok(child) = Command::new("ffplay")
                .args(["-nodisp", "-loop", "0", "-loglevel", "quiet", &path_s])
                .spawn()
            {
                return Ok(child);
            }
        } else if let Ok(child) = Command::new("ffplay")
            .args(["-nodisp", "-autoexit", "-loglevel", "quiet", &path_s])
            .spawn()
        {
            return Ok(child);
        }
        Command::new("aplay")
            .arg(&path_s)
            .spawn()
            .map_err(|e| format!("could not play wav ({e}). Install ffplay or aplay to preview."))
    }
}
