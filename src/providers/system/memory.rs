use std::fs;

use serde::Serialize;

use super::ProviderError;
use super::SystemProvider;

#[derive(Debug, Clone, Serialize)]
pub struct MemoryInfo {
    pub total_kib: u64,
    pub available_kib: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SwapInfo {
    pub total_kib: u64,
    pub free_kib: u64,
}

fn parse_meminfo(raw: &str) -> Result<MemoryInfo, ProviderError> {
    let mut total_kib = None;
    let mut available_kib = None;
    let mut free_kib = None;

    for line in raw.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let kib = value
            .split_whitespace()
            .next()
            .and_then(|value| value.parse::<u64>().ok());
        match key.trim() {
            "MemTotal" => total_kib = kib,
            "MemAvailable" => available_kib = kib,
            "MemFree" => free_kib = kib,
            _ => {}
        }
    }

    let total_kib = total_kib.ok_or_else(|| ProviderError::parse("missing MemTotal"))?;
    let available_kib = available_kib.or(free_kib).ok_or_else(|| {
        ProviderError::parse("missing MemAvailable and MemFree from /proc/meminfo")
    })?;

    Ok(MemoryInfo {
        total_kib,
        available_kib,
    })
}

fn parse_swapinfo(raw: &str) -> Result<SwapInfo, ProviderError> {
    let mut total_kib = None;
    let mut free_kib = None;

    for line in raw.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let kib = value
            .split_whitespace()
            .next()
            .and_then(|value| value.parse::<u64>().ok());
        match key.trim() {
            "SwapTotal" => total_kib = kib,
            "SwapFree" => free_kib = kib,
            _ => {}
        }
    }

    Ok(SwapInfo {
        total_kib: total_kib.ok_or_else(|| ProviderError::parse("missing SwapTotal"))?,
        free_kib: free_kib.ok_or_else(|| ProviderError::parse("missing SwapFree"))?,
    })
}

impl SystemProvider {
    pub fn memory_info(&self) -> Result<MemoryInfo, ProviderError> {
        let raw = fs::read_to_string("/proc/meminfo").map_err(ProviderError::io)?;
        parse_meminfo(&raw)
    }

    pub fn swap_info(&self) -> Result<SwapInfo, ProviderError> {
        let raw = fs::read_to_string("/proc/meminfo").map_err(ProviderError::io)?;
        parse_swapinfo(&raw)
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_meminfo() {
        let raw = r#"
MemTotal:       32773524 kB
MemFree:        10926812 kB
MemAvailable:   19534000 kB
"#;
        let memory = parse_meminfo(raw).unwrap();
        assert_eq!(memory.total_kib, 32_773_524);
        assert_eq!(memory.available_kib, 19_534_000);
    }

    #[test]
    fn parses_swapinfo() {
        let raw = r#"
SwapTotal:       8060924 kB
SwapFree:        7000000 kB
"#;
        let swap = parse_swapinfo(raw).unwrap();
        assert_eq!(swap.total_kib, 8_060_924);
        assert_eq!(swap.free_kib, 7_000_000);
    }
}
