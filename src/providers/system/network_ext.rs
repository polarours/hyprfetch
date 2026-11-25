//! Opt-in network lookups: the public IP and a one-line weather report.
//!
//! Both go through `curl` with a hard three-second budget, so a missing binary,
//! a captive portal or a slow API degrades to "module unavailable" instead of
//! stalling the render. Neither is on by default — they leak a request off the
//! machine, which a fetch tool should only do when asked.

use std::time::Duration;

use super::{SystemProvider, run_with_timeout};

const BUDGET: Duration = Duration::from_millis(3000);

/// A weather line: condition text plus the temperature.
#[derive(Debug, Clone, PartialEq)]
pub struct Weather {
    pub condition: String,
    pub temperature: String,
}

impl Weather {
    pub fn display(&self) -> String {
        format!("{} {}", self.condition, self.temperature)
    }
}

impl SystemProvider {
    /// Public IPv4 address, via icanhazip.
    pub fn public_ip(&self) -> Option<String> {
        let raw = run_with_timeout(
            "curl",
            &["-4", "-s", "-f", "--max-time", "3", "https://icanhazip.com"],
            BUDGET,
        )?;
        let ip = raw.trim();
        if ip.is_empty() {
            return None;
        }
        // Guard against a captive portal returning HTML.
        if !ip
            .chars()
            .all(|c| c.is_ascii_hexdigit() || c == '.' || c == ':')
        {
            return None;
        }
        Some(ip.to_string())
    }

    /// Current weather from wttr.in, which geolocates by the caller's address.
    pub fn weather(&self) -> Option<Weather> {
        let raw = run_with_timeout(
            "curl",
            &["-s", "-f", "--max-time", "3", "wttr.in/?format=%C|%t"],
            BUDGET,
        )?;
        parse_weather(&raw)
    }
}

/// Split the `%C|%t` payload; separated for testability.
fn parse_weather(raw: &str) -> Option<Weather> {
    let (condition, temperature) = raw.trim().split_once('|')?;
    let condition = condition.trim();
    let temperature = temperature.trim();
    if condition.is_empty() || temperature.is_empty() {
        return None;
    }
    Some(Weather {
        condition: condition.to_string(),
        temperature: temperature.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::parse_weather;

    #[test]
    fn parses_a_wttr_line() {
        let weather = parse_weather("Partly cloudy|+12°C\n").unwrap();
        assert_eq!(weather.condition, "Partly cloudy");
        assert_eq!(weather.temperature, "+12°C");
        assert_eq!(weather.display(), "Partly cloudy +12°C");
    }

    #[test]
    fn empty_halves_are_rejected() {
        assert!(parse_weather("|+12°C").is_none());
        assert!(parse_weather("Sunny|").is_none());
        assert!(parse_weather("no separator").is_none());
    }
}
