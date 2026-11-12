//! Finding and running the pinned Godot 4.7.2 standard editor.
//!
//! Discovery order:
//! 1. The executable path configured in Nesticles.
//! 2. `NESTICLES_GODOT`, when that variable is set.
//! 3. A `godot` or `godot4` on `PATH` whose version is 4.7.2 standard.
//! 4. The user-local binary installed for this pin.
//!
//! Nothing here downloads Godot, and a missing editor is an error. Callers
//! must not substitute the browser exporter.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub const PINNED_VERSION: &str = "4.7.2";
pub const RELEASE_PAGE: &str = "https://github.com/godotengine/godot/releases/tag/4.7.2-stable";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GodotTool {
    pub executable: PathBuf,
    pub version: String,
    pub source: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GodotOutput {
    pub status: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// The user-local editor path for this pin. The binary is not part of the repo.
pub fn managed_executable() -> PathBuf {
    data_home().join("nesticles/godot/4.7.2/Godot_v4.7.2-stable_linux.x86_64")
}

pub fn linux_template_dir() -> PathBuf {
    data_home().join("godot/export_templates/4.7.2.stable")
}

pub fn discover(configured: Option<&Path>) -> Result<GodotTool, String> {
    if let Some(path) = configured.filter(|path| !path.as_os_str().is_empty()) {
        return accept(path, "settings");
    }
    if let Some(env) = std::env::var_os("NESTICLES_GODOT").filter(|value| !value.is_empty()) {
        return accept(Path::new(&env), "NESTICLES_GODOT");
    }
    let mut wrong = Vec::new();
    for candidate in path_candidates() {
        match version_of(&candidate) {
            Ok(line) if is_pinned(&line) => {
                return Ok(GodotTool {
                    executable: candidate,
                    version: line,
                    source: "PATH".into(),
                });
            }
            Ok(line) => wrong.push(format!("{} reports {line}", candidate.display())),
            Err(_) => {}
        }
    }
    let managed = managed_executable();
    if managed.is_file() {
        return accept(&managed, "managed");
    }
    Err(missing_message(&wrong))
}

/// Import resources, then return. A script error is a failure.
pub fn import_project(executable: &Path, project_dir: &Path) -> Result<(), String> {
    let imported = run_command(
        executable,
        &[
            "--headless".into(),
            "--path".into(),
            path_arg(project_dir),
            "--import".into(),
        ],
        Duration::from_secs(120),
    )?;
    if imported.status != Some(0) || script_error(&imported) {
        return Err(format!(
            "Godot import failed ({})\n{}{}",
            status_label(imported.status),
            imported.stdout,
            imported.stderr
        ));
    }
    Ok(())
}

/// Start the generated project without waiting. The process is the caller's.
///
/// Standard output and error are written to `godot-preview.log` in the project
/// directory. The call does not wait for the game to exit.
pub fn launch_preview(executable: &Path, project_dir: &Path) -> Result<Child, String> {
    let log_path = project_dir.join("godot-preview.log");
    let log = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_path)
        .map_err(|err| format!("Could not write {}: {err}", log_path.display()))?;
    let err_log = log
        .try_clone()
        .map_err(|err| format!("Could not write {}: {err}", log_path.display()))?;
    Command::new(executable)
        .arg("--audio-driver")
        .arg("Dummy")
        .arg("--position")
        .arg("64,48")
        .arg("--path")
        .arg(project_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(err_log))
        .spawn()
        .map_err(|err| format!("Could not start {}: {err}", executable.display()))
}

/// Import the project, then replay the fixed-point traces against `tick_play`.
pub fn run_parity(executable: &Path, project_dir: &Path) -> Result<GodotOutput, String> {
    import_project(executable, project_dir)?;
    run_command(
        executable,
        &[
            "--headless".into(),
            "--audio-driver".into(),
            "Dummy".into(),
            "--path".into(),
            path_arg(project_dir),
            "--quit-after".into(),
            "120".into(),
            "--".into(),
            "--nesticles-parity".into(),
        ],
        Duration::from_secs(60),
    )
}

/// Import the project, then run the headless gameplay checks.
pub fn run_proof(executable: &Path, project_dir: &Path) -> Result<GodotOutput, String> {
    import_project(executable, project_dir)?;
    run_command(
        executable,
        &[
            "--headless".into(),
            "--audio-driver".into(),
            "Dummy".into(),
            "--path".into(),
            path_arg(project_dir),
            "--quit-after".into(),
            "600".into(),
            "--".into(),
            "--nesticles-proof".into(),
        ],
        Duration::from_secs(60),
    )
}

/// Export a Linux binary. A missing template is an error, not a successful export.
pub fn export_native(
    executable: &Path,
    project_dir: &Path,
    output: &Path,
) -> Result<GodotOutput, String> {
    let template = linux_release_template();
    if template.is_none() {
        return Err(format!(
            "Godot {PINNED_VERSION} Linux export templates were not found in {}. A native export was not produced, and HTML was not used as a substitute.",
            linux_template_dir().display()
        ));
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Could not create {}: {err}", parent.display()))?;
    }
    let exported = run_command(
        executable,
        &[
            "--headless".into(),
            "--path".into(),
            path_arg(project_dir),
            "--export-release".into(),
            "Linux".into(),
            path_arg(output),
        ],
        Duration::from_secs(180),
    )?;
    if exported.status != Some(0) || script_error(&exported) {
        return Err(format!(
            "Godot export failed ({})\n{}{}",
            status_label(exported.status),
            exported.stdout,
            exported.stderr
        ));
    }
    if !output.is_file() {
        return Err(format!(
            "Godot export finished without writing {}",
            output.display()
        ));
    }
    Ok(exported)
}

/// Run an exported binary and return its output. This is the standalone game.
pub fn run_exported(executable: &Path) -> Result<GodotOutput, String> {
    if !executable.is_file() {
        return Err(format!(
            "Exported game {} is not a file, so it was not launched.",
            executable.display()
        ));
    }
    let mut command = Command::new(executable);
    command.arg("--nesticles-proof");
    if let Some(dir) = executable.parent() {
        command.current_dir(dir);
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = command.spawn().map_err(|err| {
        format!(
            "Could not launch the exported game {}: {err}",
            executable.display()
        )
    })?;
    wait_child(child, Duration::from_secs(60))
}

fn linux_release_template() -> Option<PathBuf> {
    let dir = linux_template_dir();
    for name in ["linux_release.x86_64", "linux_release.x86_64.stp"] {
        let path = dir.join(name);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

fn accept(path: &Path, source: &str) -> Result<GodotTool, String> {
    if !path.is_file() {
        return Err(format!(
            "{}\nThe configured executable {} is not a file.",
            missing_message(&[]),
            path.display()
        ));
    }
    let line = version_of(path)?;
    if !is_pinned(&line) {
        return Err(format!(
            "Found {} reporting {line}. Nesticles is pinned to Godot {PINNED_VERSION} standard, not .NET. Point the Godot field, NESTICLES_GODOT, or PATH at that build. The official release is {RELEASE_PAGE}. The browser exporter was not used.",
            path.display()
        ));
    }
    Ok(GodotTool {
        executable: path.to_path_buf(),
        version: line,
        source: source.into(),
    })
}

fn version_of(executable: &Path) -> Result<String, String> {
    let output = Command::new(executable)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|err| format!("Could not run {}: {err}", executable.display()))?;
    let mut text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        text = String::from_utf8_lossy(&output.stderr).trim().to_string();
    }
    let line = text.lines().next().unwrap_or("").trim().to_string();
    if !output.status.success() {
        return Err(format!(
            "{} --version exited {}: {line}",
            executable.display(),
            output.status
        ));
    }
    if line.is_empty() {
        return Err(format!(
            "{} --version printed no version",
            executable.display()
        ));
    }
    Ok(line)
}

fn is_pinned(line: &str) -> bool {
    let token = line.split_whitespace().next().unwrap_or("");
    let parts: Vec<&str> = token.split('.').collect();
    let lowered = line.to_ascii_lowercase();
    parts.len() >= 3
        && parts[0] == "4"
        && parts[1] == "7"
        && parts[2] == "2"
        && !lowered.contains("mono")
        && !line.contains(".NET")
}

fn path_candidates() -> Vec<PathBuf> {
    let Some(path) = std::env::var_os("PATH") else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for dir in std::env::split_paths(&path) {
        for name in ["godot", "godot4"] {
            let candidate = dir.join(name);
            if candidate.is_file() {
                found.push(candidate);
            }
        }
    }
    found
}

fn data_home() -> PathBuf {
    if let Some(xdg) = std::env::var_os("XDG_DATA_HOME").filter(|value| !value.is_empty()) {
        return PathBuf::from(xdg);
    }
    if let Some(home) = std::env::var_os("HOME").filter(|value| !value.is_empty()) {
        return PathBuf::from(home).join(".local/share");
    }
    PathBuf::from(".local/share")
}

fn missing_message(wrong: &[String]) -> String {
    let mut message = format!(
        "Godot {PINNED_VERSION} standard (not .NET) was not found. Nesticles did not launch a preview and did not use the browser exporter. Set the Godot field on the Play tab, set NESTICLES_GODOT, put a matching godot on PATH, or place the official Linux binary at {}. The official release is {RELEASE_PAGE}.",
        managed_executable().display()
    );
    if !wrong.is_empty() {
        message.push_str(" Other Godot binaries were ignored: ");
        message.push_str(&wrong.join("; "));
        message.push('.');
    }
    message
}

fn path_arg(path: &Path) -> String {
    path.display().to_string()
}

fn script_error(output: &GodotOutput) -> bool {
    let combined = format!("{}\n{}", output.stdout, output.stderr);
    combined.contains("SCRIPT ERROR")
        || combined.contains("Parse Error")
        || combined.contains("Failed to load script")
}

fn status_label(status: Option<i32>) -> String {
    match status {
        Some(code) => code.to_string(),
        None => "no exit code".into(),
    }
}

fn run_command(
    executable: &Path,
    args: &[String],
    timeout: Duration,
) -> Result<GodotOutput, String> {
    let mut command = Command::new(executable);
    command.args(args);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = command
        .spawn()
        .map_err(|err| format!("Could not start {}: {err}", executable.display()))?;
    wait_child(child, timeout)
}

fn wait_child(mut child: Child, timeout: Duration) -> Result<GodotOutput, String> {
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let out_handle = thread::spawn(move || read_pipe(stdout));
    let err_handle = thread::spawn(move || read_pipe(stderr));
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() > timeout => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = out_handle.join();
                let _ = err_handle.join();
                return Err(format!(
                    "Godot timed out after {} seconds",
                    timeout.as_secs()
                ));
            }
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(err) => return Err(format!("Could not wait for Godot: {err}")),
        }
    };
    let stdout = out_handle.join().unwrap_or_default();
    let stderr = err_handle.join().unwrap_or_default();
    Ok(GodotOutput {
        status: status.code(),
        stdout,
        stderr,
    })
}

fn read_pipe(pipe: Option<impl Read>) -> String {
    let Some(mut pipe) = pipe else {
        return String::new();
    };
    let mut buffer = Vec::new();
    let _ = pipe.read_to_end(&mut buffer);
    String::from_utf8_lossy(&buffer).into_owned()
}
