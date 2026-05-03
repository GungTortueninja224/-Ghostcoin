use crate::config;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process;

pub struct InstanceLock {
    path: PathBuf,
}

impl InstanceLock {
    pub fn acquire() -> Result<Self, String> {
        config::ensure_data_dir().map_err(|e| format!("failed to prepare data dir: {}", e))?;
        let path = config::instance_lock_file();

        match try_create_lock(&path) {
            Ok(()) => Ok(Self { path }),
            Err(first_err) => {
                if is_stale_lock(&path) {
                    let _ = fs::remove_file(&path);
                    match try_create_lock(&path) {
                        Ok(()) => Ok(Self { path }),
                        Err(_) => Err(format!(
                            "another GhostCoin instance is already using {}\nclose the other process and try again.",
                            path.display()
                        )),
                    }
                } else {
                    Err(first_err)
                }
            }
        }
    }
}

impl Drop for InstanceLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn try_create_lock(path: &Path) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| lock_error_message(path))?;

    let payload = format!(
        "pid={}\nexe={}\n",
        process::id(),
        std::env::current_exe()
            .ok()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    );
    file.write_all(payload.as_bytes())
        .map_err(|e| format!("failed to write lock file {}: {}", path.display(), e))
}

fn lock_error_message(path: &Path) -> String {
    let pid_hint = fs::read_to_string(path)
        .ok()
        .and_then(|content| {
            content
                .lines()
                .find_map(|line| line.strip_prefix("pid=").map(str::to_string))
        })
        .unwrap_or_else(|| "unknown".to_string());

    format!(
        "another GhostCoin instance appears to be running (pid {}).\nlock file: {}\nstop the other process before starting a new local node.",
        pid_hint,
        path.display()
    )
}

fn is_stale_lock(path: &Path) -> bool {
    let pid = fs::read_to_string(path)
        .ok()
        .and_then(|content| {
            content
                .lines()
                .find_map(|line| line.strip_prefix("pid="))
                .and_then(|raw| raw.parse::<u32>().ok())
        });

    match pid {
        Some(pid) => !process_is_running(pid),
        None => false,
    }
}

#[cfg(windows)]
fn process_is_running(pid: u32) -> bool {
    std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {}", pid), "/NH"])
        .output()
        .ok()
        .map(|output| {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.contains(&pid.to_string()) && !stdout.contains("No tasks are running")
        })
        .unwrap_or(true)
}

#[cfg(not(windows))]
fn process_is_running(pid: u32) -> bool {
    Path::new(&format!("/proc/{}", pid)).exists()
}
