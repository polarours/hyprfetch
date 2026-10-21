use std::fs;
use std::time::{Duration, Instant};

use serde::Serialize;

use super::ProviderError;
use super::SystemProvider;

#[derive(Debug, Clone, Serialize)]
pub struct CpuUsageInfo {
    pub usage_percent: f64,
    pub num_logical_cpus: usize,
}

/// Minimum CPU sampling window.
///
/// `/proc/stat` counters advance in kernel tick steps, so a shorter window
/// yields a noisy average.
const MIN_CPU_SAMPLE_WINDOW: Duration = Duration::from_millis(50);

fn parse_cpu_model(raw: &str) -> Result<String, ProviderError> {
    for line in raw.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        if matches!(key.trim(), "model name" | "Hardware" | "Processor") {
            return Ok(value.trim().to_string());
        }
    }
    Err(ProviderError::parse("missing cpu model"))
}

#[derive(Debug, Clone, Default)]
pub(super) struct CpuStat {
    user: u64,
    nice: u64,
    system: u64,
    idle: u64,
    iowait: u64,
    irq: u64,
    softirq: u64,
    steal: u64,
}

/// A `/proc/stat` snapshot taken when the provider was created.
#[derive(Debug, Clone)]
pub(super) struct CpuBaseline {
    taken_at: Instant,
    stat: CpuStat,
}

pub(super) fn read_cpu_baseline() -> Option<CpuBaseline> {
    Some(CpuBaseline {
        taken_at: Instant::now(),
        stat: read_cpu_stat().ok()?,
    })
}

fn read_cpu_stat() -> Result<CpuStat, ProviderError> {
    let raw = fs::read_to_string("/proc/stat").map_err(ProviderError::io)?;
    let first_line = raw
        .lines()
        .next()
        .ok_or_else(|| ProviderError::parse("/proc/stat is empty"))?;
    let mut fields = first_line.split_whitespace().skip(1);
    let mut parse_next = || -> Result<u64, ProviderError> {
        fields
            .next()
            .ok_or_else(|| ProviderError::parse("not enough fields in /proc/stat"))
            .and_then(|v| {
                v.parse::<u64>()
                    .map_err(|e| ProviderError::parse(format!("invalid cpu stat: {e}")))
            })
    };
    Ok(CpuStat {
        user: parse_next()?,
        nice: parse_next()?,
        system: parse_next()?,
        idle: parse_next()?,
        iowait: parse_next()?,
        irq: parse_next()?,
        softirq: parse_next()?,
        steal: parse_next()?,
    })
}

fn cpu_usage_percent(a: &CpuStat, b: &CpuStat) -> f64 {
    let idle_delta = b.idle + b.iowait - a.idle - a.iowait;
    let total_a: u64 = a.user + a.nice + a.system + a.idle + a.iowait + a.irq + a.softirq + a.steal;
    let total_b: u64 = b.user + b.nice + b.system + b.idle + b.iowait + b.irq + b.softirq + b.steal;
    let total_delta = total_b.saturating_sub(total_a);
    if total_delta == 0 {
        return 0.0;
    }
    (1.0 - idle_delta as f64 / total_delta as f64) * 100.0
}

fn num_logical_cpus() -> Result<usize, ProviderError> {
    let raw = fs::read_to_string("/proc/cpuinfo").map_err(ProviderError::io)?;
    let count = raw
        .lines()
        .filter(|line| line.starts_with("processor"))
        .count();
    (count > 0)
        .then_some(count)
        .ok_or_else(|| ProviderError::parse("no processor entries in /proc/cpuinfo"))
}

impl SystemProvider {
    pub fn cpu_model(&self) -> Result<String, ProviderError> {
        let raw = fs::read_to_string("/proc/cpuinfo").map_err(ProviderError::io)?;
        parse_cpu_model(&raw)
    }

    pub fn cpu_usage(&self) -> Result<CpuUsageInfo, ProviderError> {
        let baseline = self
            .cpu_baseline
            .get()
            .and_then(Option::as_ref)
            .ok_or_else(|| ProviderError::not_found("cpu baseline unavailable"))?;

        let elapsed = baseline.taken_at.elapsed();
        if elapsed < MIN_CPU_SAMPLE_WINDOW {
            std::thread::sleep(MIN_CPU_SAMPLE_WINDOW - elapsed);
        }

        let current = read_cpu_stat()?;
        let usage = cpu_usage_percent(&baseline.stat, &current);
        let num_logical_cpus = num_logical_cpus()?;
        Ok(CpuUsageInfo {
            usage_percent: usage,
            num_logical_cpus,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cpu_model() {
        let raw = r#"
processor   : 0
model name  : AMD Ryzen 7 7840HS w/ Radeon 780M Graphics
"#;
        let model = parse_cpu_model(raw).unwrap();
        assert!(model.contains("Ryzen 7"));
    }

    #[test]
    fn parses_cpu_stat_and_computes_usage() {
        let stat_a = CpuStat {
            user: 100,
            nice: 10,
            system: 50,
            idle: 1000,
            iowait: 5,
            irq: 0,
            softirq: 0,
            steal: 0,
        };
        let stat_b = CpuStat {
            user: 200,
            nice: 20,
            system: 100,
            idle: 1000,
            iowait: 10,
            irq: 0,
            softirq: 0,
            steal: 0,
        };
        // total delta: (200-100)+(20-10)+(100-50)+(1000-1000)+(10-5) = 100+10+50+0+5 = 165
        // idle delta: (1000-1000)+(10-5) = 0+5 = 5
        // usage = (165-5)/165 = 160/165 ≈ 96.97%
        let usage = cpu_usage_percent(&stat_a, &stat_b);
        assert!((usage - 96.97).abs() < 0.1);
    }

    #[test]
    fn cpu_usage_is_zero_when_no_delta() {
        let stat = CpuStat::default();
        let usage = cpu_usage_percent(&stat, &stat);
        assert_eq!(usage, 0.0);
    }

    #[test]
    fn cpu_usage_covers_at_least_the_minimum_window() {
        let provider = SystemProvider::new();
        let start = Instant::now();
        let usage = provider
            .cpu_usage()
            .expect("cpu usage should be readable on linux");
        assert!(start.elapsed() >= MIN_CPU_SAMPLE_WINDOW);
        assert!((0.0..=100.0).contains(&usage.usage_percent));
    }
}
