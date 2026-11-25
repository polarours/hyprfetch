//! The primary display: resolution and refresh rate.
//!
//! Hyprland knows both values, so when the compositor is running the module
//! reads the focused monitor from `hyprctl`. Outside Hyprland it falls back to
//! the DRM connector status and modes, which carry the resolution but not the
//! refresh rate — an honest partial row beats inventing a number.

use std::fs;
use std::path::Path;

use super::SystemProvider;

/// One connected output.
#[derive(Debug, Clone, PartialEq)]
pub struct DisplayInfo {
    pub name: String,
    pub width: u32,
    pub height: u32,
    /// `None` when the source cannot report it (DRM fallback).
    pub refresh_hz: Option<f64>,
}

impl DisplayInfo {
    pub fn display(&self) -> String {
        match self.refresh_hz {
            Some(hz) => format!(
                "{}x{} @{:.0}Hz ({})",
                self.width, self.height, hz, self.name
            ),
            None => format!("{}x{} ({})", self.width, self.height, self.name),
        }
    }
}

impl SystemProvider {
    /// The first connected output from `/sys/class/drm`, resolution only.
    pub fn drm_display(&self) -> Option<DisplayInfo> {
        let entries = fs::read_dir("/sys/class/drm").ok()?;
        let mut candidates: Vec<_> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("card") && name.contains('-'))
            })
            .collect();
        candidates.sort();

        for path in candidates {
            if read_trimmed(&path.join("status")).as_deref() != Some("connected") {
                continue;
            }
            let Some(modes) = read_trimmed(&path.join("modes")) else {
                continue;
            };
            let Some((width, height)) = parse_mode(modes.lines().next()?) else {
                continue;
            };
            let name = connector_name(&path);
            return Some(DisplayInfo {
                name,
                width,
                height,
                refresh_hz: None,
            });
        }
        None
    }
}

/// `card1-eDP-1` → `eDP-1`.
fn connector_name(path: &Path) -> String {
    let raw = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("display");
    raw.split_once('-')
        .map(|(_, rest)| rest)
        .unwrap_or(raw)
        .to_string()
}

/// `2560x1600` → `(2560, 1600)`.
fn parse_mode(mode: &str) -> Option<(u32, u32)> {
    let (width, height) = mode.trim().split_once('x')?;
    Some((width.parse().ok()?, height.parse().ok()?))
}

fn read_trimmed(path: &Path) -> Option<String> {
    let raw = fs::read_to_string(path).ok()?;
    let value = raw.trim();
    (!value.is_empty()).then(|| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::{DisplayInfo, connector_name, parse_mode};
    use std::path::Path;

    #[test]
    fn parses_drm_modes() {
        assert_eq!(parse_mode("2560x1600\n"), Some((2560, 1600)));
        assert_eq!(parse_mode("1920x1080"), Some((1920, 1080)));
        assert_eq!(parse_mode("nonsense"), None);
    }

    #[test]
    fn connector_names_drop_the_card_prefix() {
        assert_eq!(
            connector_name(Path::new("/sys/class/drm/card1-eDP-1")),
            "eDP-1"
        );
        assert_eq!(
            connector_name(Path::new("/sys/class/drm/card0-HDMI-A-1")),
            "HDMI-A-1"
        );
    }

    #[test]
    fn display_string_shows_refresh_when_known() {
        let with_hz = DisplayInfo {
            name: "eDP-1".into(),
            width: 2560,
            height: 1600,
            refresh_hz: Some(165.019),
        };
        assert_eq!(with_hz.display(), "2560x1600 @165Hz (eDP-1)");

        let without = DisplayInfo {
            refresh_hz: None,
            ..with_hz
        };
        assert_eq!(without.display(), "2560x1600 (eDP-1)");
    }

    #[test]
    fn live_drm_display_has_a_resolution() {
        let provider = super::SystemProvider::new();
        if let Some(display) = provider.drm_display() {
            assert!(display.width > 0 && display.height > 0);
        }
    }
}
