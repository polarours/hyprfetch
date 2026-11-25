//! System information providers.
//!
//! Every submodule owns one part of the system state: the operating system
//! and the host, the processor, memory and swap, storage and power, the
//! graphics hardware and the session.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::Duration;

use thiserror::Error;

mod cpu;
mod firmware;
mod graphics;
mod load;
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
pub use session::{AudioInfo, DatetimeInfo, NetworkInfo, TemperatureInfo};
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
    /// Skip expensive probes for a near-instant render. Values that need a
    /// vendor tool or a sampling window are omitted rather than fabricated.
    pub fast: bool,
}

impl Default for SystemProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemProvider {
    pub fn new() -> Self {
        Self::with_fast(false)
    }

    pub fn with_fast(fast: bool) -> Self {
        let cpu_baseline = OnceLock::new();
        if !fast {
            let _ = cpu_baseline.set(cpu::read_cpu_baseline());
        }
        Self { cpu_baseline, fast }
    }
}

/// Runs a command and gives up on it after `timeout`.
///
/// A driver that does not answer must not be able to hang the whole run. The
/// deadline covers the pipes as well: a background descendant that inherited
/// stdout cannot stretch the read past it.
pub(super) fn run_with_timeout(program: &str, args: &[&str], timeout: Duration) -> Option<String> {
    let child = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    match crate::util::wait_with_deadline(child, timeout) {
        Ok(crate::util::CommandWait::Finished(output)) if output.status.success() => {
            Some(String::from_utf8_lossy(&output.stdout).to_string())
        }
        _ => None,
    }
}

/// Reads a file and trims the whitespace around its content.
pub(super) fn read_trimmed(path: PathBuf) -> Result<String, ProviderError> {
    let raw = fs::read_to_string(&path).map_err(ProviderError::io)?;
    Ok(raw.trim().to_string())
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{SystemProvider, run_with_timeout};

    #[test]
    fn fast_mode_skips_the_cpu_sampling_baseline() {
        let fast = SystemProvider::with_fast(true);
        assert!(fast.cpu_baseline.get().is_none());
        // Without a baseline, cpu_usage() fails and the CPU module falls back
        // to showing the model only — the intended fast-mode degradation.
        assert!(fast.cpu_usage().is_err());

        let normal = SystemProvider::new();
        assert!(normal.cpu_baseline.get().is_some());
    }

    #[test]
    fn a_descendant_holding_the_pipe_cannot_stretch_the_wait() {
        // The child exits at once but leaves a background grandchild holding
        // the pipe. The helper used to read until EOF with no deadline and
        // hang for the grandchild's lifetime; now the deadline bounds it.
        let started = Instant::now();
        let output = run_with_timeout(
            "sh",
            &["-c", "echo done; sleep 30 &"],
            Duration::from_millis(300),
        );
        let elapsed = started.elapsed();
        assert!(
            elapsed < Duration::from_secs(2),
            "the wait stretched past its deadline: {elapsed:?}"
        );
        assert_eq!(
            output.as_deref(),
            Some("done\n"),
            "the output before the deadline must survive"
        );
    }
}
