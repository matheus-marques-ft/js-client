use log::{error, info};
use std::env;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::RecvTimeoutError;
use std::thread;
use std::time::Duration;
use tauri::path::BaseDirectory;
use tauri::AppHandle;
use tauri::Manager;

// Map platform/architecture to the subdirectory holding the binary
fn platform_subdir() -> Option<&'static str> {
    let os = env::consts::OS; // "linux" | "macos" | "windows"
    let arch = env::consts::ARCH; // "x86_64" | "aarch64" | "arm" | ...

    match os {
        "linux" => match arch {
            "x86_64" => Some("linux-amd64"),
            "arm" | "aarch64" => Some("linux-arm64"),
            _ => None,
        },
        "macos" => match arch {
            "x86_64" => Some("darwin-amd64"),
            "arm" | "aarch64" => Some("darwin-arm64"),
            _ => None,
        },
        "windows" => Some("windows"),
        _ => None,
    }
}

// Append .exe on Windows; unchanged on other platforms
fn executable_name() -> &'static str {
    if env::consts::OS == "windows" {
        "JumpServerClient.exe"
    } else {
        "JumpServerClient"
    }
}

// Generate candidate executable paths:
// - Dev mode: bin/<subdir>/JumpServerClient[.exe] relative to the project root
// - Production mode: alongside the packaged executable, or in macOS's Resources directory
fn append_resource_candidates(app: &AppHandle, candidates: &mut Vec<PathBuf>) {
    let Some(subdir) = platform_subdir() else {
        return;
    };
    let exe_name = executable_name();

    for rel in [
        format!("resources/bin/{}/{}", subdir, exe_name),
        format!("resources/bin/{}", exe_name),
        format!("bin/{}/{}", subdir, exe_name),
        format!("bin/{}", exe_name),
    ] {
        if let Ok(path) = app.path().resolve(&rel, BaseDirectory::Resource) {
            candidates.push(path);
        }
    }
}

fn candidate_paths(app: &AppHandle, is_dev: bool) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let subdir = match platform_subdir() {
        Some(s) => s,
        None => return candidates,
    };

    let exe_name = executable_name();

    if is_dev {
        // Dev mode: try the ./, ../, and ../../ relative locations, to work from different working directories
        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let bases = [cwd.clone(), cwd.join(".."), cwd.join("../..")];
        for base in bases {
            candidates.push(
                base.join("resources")
                    .join("bin")
                    .join(subdir)
                    .join(exe_name),
            );
            candidates.push(base.join("resources").join("bin").join(exe_name));
        }
    } else {
        append_resource_candidates(app, &mut candidates);

        if let Ok(current_exe) = env::current_exe() {
            if let Some(base) = current_exe.parent() {
                candidates.push(
                    base.join("resources")
                        .join("bin")
                        .join(subdir)
                        .join(exe_name),
                );
                candidates.push(base.join("resources").join("bin").join(exe_name));
                // macOS packaging: the run directory is under App.app/Contents/MacOS/
                // Resources are usually in App.app/Contents/Resources/
                if cfg!(target_os = "macos") {
                    if let Some(contents) = base.parent() {
                        let resources = contents.join("Resources").join("resources").join("bin");
                        candidates.push(resources.join(subdir).join(exe_name));
                        candidates.push(resources.join(exe_name));
                    }
                }
            }
        }
    }

    candidates
}

fn resolve_executable(app: &AppHandle, is_dev: bool) -> Option<PathBuf> {
    for p in candidate_paths(app, is_dev) {
        if p.exists() && p.is_file() {
            return Some(p);
        }
    }
    None
}

fn canonicalize_if_exists(path: &PathBuf) -> Option<PathBuf> {
    path.canonicalize().ok()
}

#[tauri::command]
/// Launch the local JumpServerClient executable, passing the URL parameter
/// Frontend: invoke('pull_up', { url })
pub fn pull_up(app: AppHandle, url: String) -> Result<(), String> {
    if url.trim().is_empty() {
        let err_msg = "pull_up called with empty url";
        error!("{}", err_msg);
        return Err(err_msg.to_string());
    }

    // Corresponds to JS: is.dev && !process.env.IS_TEST
    let is_test = env::var("IS_TEST")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let is_dev = cfg!(debug_assertions) && !is_test;

    let Some(exe_path) = resolve_executable(&app, is_dev) else {
        let err_msg = format!(
            "JumpServerClient executable not found. Searched: {:?}",
            candidate_paths(&app, is_dev)
        );
        error!("{}", err_msg);
        return Err(err_msg);
    };

    if let Ok(current_exe) = env::current_exe() {
        if canonicalize_if_exists(&current_exe) == canonicalize_if_exists(&exe_path) {
            let err_msg = format!(
                "Refusing to relaunch current desktop binary recursively: {:?}",
                exe_path
            );
            error!("{}", err_msg);
            return Err(err_msg);
        }
    }

    info!("Launching client: {:?} {}", exe_path, url);

    // Use a pipe to capture stderr, to detect error output from the child process
    let mut child = Command::new(&exe_path)
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            let err_msg = format!("Failed to launch client: {}", e);
            error!("{}", err_msg);
            err_msg
        })?;

    // Get the stderr reader
    let stderr = child.stderr.take().ok_or_else(|| {
        let err_msg = "Failed to capture stderr from client process";
        error!("{}", err_msg);
        err_msg.to_string()
    })?;

    // Use a channel to communicate between the background thread and the main thread
    let (tx, rx) = std::sync::mpsc::channel::<String>();

    // Read stderr on a background thread, checking for error output
    thread::spawn(move || {
        let reader = BufReader::new(stderr);

        for line in reader.lines() {
            match line {
                Ok(line) => {
                    let lower = line.to_lowercase();
                    // Check whether this is an error line (Go client errors usually start with "Error:")
                    if lower.contains("error:") {
                        error!("Client stderr: {}", line);
                        // Send the error signal immediately
                        let _ = tx.send(line.clone());
                    } else if !line.trim().is_empty() {
                        // Log all non-empty output, which may be a warning or error
                        error!("Client stderr: {}", line);
                        // Also collect it if it contains common error keywords
                        if lower.contains("not found")
                            || lower.contains("not configured")
                            || lower.contains("failed")
                            || lower.contains("application configured or found")
                        {
                            let _ = tx.send(line);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to read stderr line: {}", e);
                    break;
                }
            }
        }
    });

    // Wait and loop-check for error output, up to 2 seconds
    for _ in 0..20 {
        // Check for an error message
        if let Ok(error_msg) = rx.try_recv() {
            let err_msg = format!("Client error: {}", error_msg);
            error!("{}", err_msg);
            return Err(err_msg);
        }

        // Check whether the process has already exited (possibly due to an error)
        if let Ok(Some(status)) = child.try_wait() {
            // Process has exited: wait a short moment to pick up the stderr reader's last line (avoids missing it if the process exits quickly)
            match rx.recv_timeout(Duration::from_millis(300)) {
                Ok(error_msg) => {
                    let err_msg = format!("Client error: {}", error_msg);
                    error!("{}", err_msg);
                    return Err(err_msg);
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {}
            }

            if !status.success() {
                let err_msg = format!("Client process exited with status: {:?}", status);
                error!("{}", err_msg);
                return Err(err_msg);
            }

            // The process exited successfully; this is normal (some clients may exit immediately)
            return Ok(());
        }

        thread::sleep(Duration::from_millis(100));
    }

    // Check once more for an error message (in case it arrived just after the loop ended)
    if let Ok(error_msg) = rx.try_recv() {
        let err_msg = format!("Client error: {}", error_msg);
        error!("{}", err_msg);
        return Err(err_msg);
    }

    // If the process is still running, that's normal — let it keep running in the background
    // Any subsequent errors are sent to the frontend via an event

    Ok(())
}
