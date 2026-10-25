//! System information providers.
//!
//! Every submodule owns one part of the system state: the operating system
//! and the host, the processor, memory and swap, storage and power, the
//! graphics hardware and the session.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::thread;
use std::time::{Duration, Instant};

use thiserror::Error;

mod cpu;
mod graphics;
mod media;
mod memory;
mod os;
mod session;
mod storage;

pub use cpu::CpuUsageInfo;
pub use graphics::GpuInfo;
pub use media::MediaPlayer;
pub use memory::{MemoryInfo, SwapInfo};
pub use os::{HostInfo, OsInfo, PackageInfo};
pub use session::{AudioInfo, ColorschemeInfo, DatetimeInfo, NetworkInfo, TemperatureInfo};
pub use storage::{BatteryInfo, DiskInfo};

#[derive(Debug, Clone, Error)]
pub enum ProviderError {
    #[error("io error: {0}")]
    Io(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("command error: {0}")]
    Command(String),
    #[error("not found: {0}")]
    NotFound(String),
}

impl ProviderError {
    pub fn io(error: impl ToString) -> Self {
        Self::Io(error.to_string())
    }

    pub fn parse(error: impl ToString) -> Self {
        Self::Parse(error.to_string())
    }

    pub fn command(error: impl ToString) -> Self {
        Self::Command(error.to_string())
    }

    pub fn not_found(error: impl ToString) -> Self {
        Self::NotFound(error.to_string())
    }
}

/// Reads information about the running system.
#[derive(Debug, Clone)]
pub struct SystemProvider {
    cpu_baseline: OnceLock<Option<cpu::CpuBaseline>>,
}

impl Default for SystemProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemProvider {
    pub fn new() -> Self {
        let cpu_baseline = OnceLock::new();
        let _ = cpu_baseline.set(cpu::read_cpu_baseline());
        Self { cpu_baseline }
    }
}

/// Runs a command and gives up on it after `timeout`.
///
/// A driver that does not answer must not be able to hang the whole run.
pub(super) fn run_with_timeout(program: &str, args: &[&str], timeout: Duration) -> Option<String> {
    let mut child = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                let output = child.wait_with_output().ok()?;
                if !output.status.success() {
                    return None;
                }
                return Some(String::from_utf8_lossy(&output.stdout).to_string());
            }
            Ok(None) => {}
            Err(_) => return None,
        }

        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }

        thread::sleep(Duration::from_millis(10));
    }
}

/// Reads a file and trims the whitespace around its content.
pub(super) fn read_trimmed(path: PathBuf) -> Result<String, ProviderError> {
    let raw = fs::read_to_string(&path).map_err(ProviderError::io)?;
    Ok(raw.trim().to_string())
}
