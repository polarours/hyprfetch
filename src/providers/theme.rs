use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::Serialize;

use crate::providers::system::ProviderError;

#[derive(Debug, Clone, Default, Serialize)]
pub struct ThemeInfo {
    pub gtk_theme: Option<String>,
    pub icon_theme: Option<String>,
    pub cursor_theme: Option<String>,
    pub font: Option<String>,
}

impl ThemeInfo {
    pub fn merge_missing_from(&mut self, fallback: ThemeInfo) {
        if self.gtk_theme.is_none() {
            self.gtk_theme = fallback.gtk_theme;
        }
        if self.icon_theme.is_none() {
            self.icon_theme = fallback.icon_theme;
        }
        if self.cursor_theme.is_none() {
            self.cursor_theme = fallback.cursor_theme;
        }
        if self.font.is_none() {
            self.font = fallback.font;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.gtk_theme.is_none()
            && self.icon_theme.is_none()
            && self.cursor_theme.is_none()
            && self.font.is_none()
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct EcosystemInfo {
    pub wallpaper: Option<(String, Option<String>)>,
    pub bar: Option<(String, Option<String>)>,
    pub launcher: Option<(String, Option<String>)>,
    pub notifications: Option<(String, Option<String>)>,
    pub clipboard: Option<(String, Option<String>)>,
    pub screenshot: Option<(String, Option<String>)>,
    pub idle: Option<(String, Option<String>)>,
    pub lock: Option<(String, Option<String>)>,
}

impl EcosystemInfo {
    pub fn is_empty(&self) -> bool {
        self.wallpaper.is_none()
            && self.bar.is_none()
            && self.launcher.is_none()
            && self.notifications.is_none()
            && self.clipboard.is_none()
            && self.screenshot.is_none()
            && self.idle.is_none()
            && self.lock.is_none()
    }

    pub fn display_label(&self, field: &str) -> Option<String> {
        let pair = match field {
            "wallpaper" => &self.wallpaper,
            "bar" => &self.bar,
            "launcher" => &self.launcher,
            "notifications" => &self.notifications,
            "clipboard" => &self.clipboard,
            "screenshot" => &self.screenshot,
            "idle" => &self.idle,
            "lock" => &self.lock,
            _ => return None,
        };
        pair.as_ref().map(|(label, version)| {
            if let Some(v) = version {
                format!("{label} ({v})")
            } else {
                label.clone()
            }
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct ThemeProvider {
    process_names: OnceLock<Result<HashSet<String>, ProviderError>>,
}

impl ThemeProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn theme_info(&self) -> Result<ThemeInfo, ProviderError> {
        let mut gtk4 = self
            .load_settings_file(config_path("gtk-4.0/settings.ini"))
            .unwrap_or_default();
        let gtk3 = self
            .load_settings_file(config_path("gtk-3.0/settings.ini"))
            .unwrap_or_default();
        gtk4.merge_missing_from(gtk3);
        if gtk4.is_empty() {
            return Err(ProviderError::not_found(
                "no GTK theme settings found in gtk-3.0 or gtk-4.0 settings.ini",
            ));
        }
        Ok(gtk4)
    }

    pub fn wallpaper_backend(&self) -> Result<String, ProviderError> {
        let ecosystem = self.ecosystem_info()?;
        ecosystem
            .wallpaper
            .map(|(label, _)| label)
            .ok_or_else(|| ProviderError::not_found("no known wallpaper backend detected"))
    }

    pub fn ecosystem_info(&self) -> Result<EcosystemInfo, ProviderError> {
        let process_names = self.process_names.get_or_init(read_process_names).clone()?;
        let ecosystem = detect_ecosystem_info(&process_names);
        if ecosystem.is_empty() {
            return Err(ProviderError::not_found(
                "no known Hyprland ecosystem tools detected",
            ));
        }
        Ok(ecosystem)
    }

    fn load_settings_file(&self, path: PathBuf) -> Result<ThemeInfo, ProviderError> {
        let raw = fs::read_to_string(path).map_err(ProviderError::io)?;
        Ok(parse_gtk_settings(&raw))
    }
}

fn config_path(suffix: &str) -> PathBuf {
    let base = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| Path::new(&home).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join(suffix)
}

fn parse_gtk_settings(raw: &str) -> ThemeInfo {
    let mut info = ThemeInfo::default();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty()
            || line.starts_with('#')
            || line.starts_with(';')
            || line.starts_with('[')
        {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().to_string();
        match key.trim() {
            "gtk-theme-name" => info.gtk_theme = Some(value),
            "gtk-icon-theme-name" => info.icon_theme = Some(value),
            "gtk-cursor-theme-name" => info.cursor_theme = Some(value),
            "gtk-font-name" => info.font = Some(normalize_font_name(&value)),
            _ => {}
        }
    }
    info
}

fn normalize_font_name(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn read_process_names() -> Result<HashSet<String>, ProviderError> {
    let mut names = HashSet::new();
    for entry in fs::read_dir("/proc").map_err(ProviderError::io)? {
        let entry = entry.map_err(ProviderError::io)?;
        let Some(file_name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        if !file_name
            .chars()
            .all(|character| character.is_ascii_digit())
        {
            continue;
        }
        let comm_path = entry.path().join("comm");
        if let Ok(name) = fs::read_to_string(comm_path) {
            names.insert(name.trim().to_ascii_lowercase());
        }
    }
    Ok(names)
}

fn detect_ecosystem_info(process_names: &HashSet<String>) -> EcosystemInfo {
    EcosystemInfo {
        wallpaper: detect_versioned_backend(
            process_names,
            &[
                ("hyprpaper", "hyprpaper"),
                ("swww-daemon", "swww"),
                ("swaybg", "swaybg"),
                ("mpvpaper", "mpvpaper"),
                ("waypaper", "waypaper"),
            ],
        ),
        bar: detect_versioned_backend(
            process_names,
            &[("waybar", "waybar"), ("ags", "ags"), ("eww", "eww")],
        ),
        launcher: detect_versioned_backend(
            process_names,
            &[
                ("fuzzel", "fuzzel"),
                ("rofi", "rofi"),
                ("wofi", "wofi"),
                ("walker", "walker"),
                ("tofi", "tofi"),
            ],
        ),
        notifications: detect_versioned_backend(
            process_names,
            &[("mako", "mako"), ("swaync", "swaync"), ("dunst", "dunst")],
        ),
        clipboard: detect_versioned_backend(
            process_names,
            &[
                ("cliphist", "cliphist"),
                ("wl-copy", "wl-clipboard"),
                ("wl-paste", "wl-clipboard"),
            ],
        ),
        screenshot: detect_versioned_backend(
            process_names,
            &[
                ("grim", "grim"),
                ("slurp", "slurp"),
                ("hyprshot", "hyprshot"),
                ("flameshot", "flameshot"),
            ],
        ),
        idle: detect_versioned_backend(
            process_names,
            &[("hypridle", "hypridle"), ("swayidle", "swayidle")],
        ),
        lock: detect_versioned_backend(
            process_names,
            &[
                ("hyprlock", "hyprlock"),
                ("swaylock", "swaylock"),
                ("gtklock", "gtklock"),
            ],
        ),
    }
}

fn detect_versioned_backend(
    process_names: &HashSet<String>,
    candidates: &[(&str, &str)],
) -> Option<(String, Option<String>)> {
    for (process, label) in candidates {
        if process_names.contains(*process) {
            let version = detect_version(process);
            return Some((label.to_string(), version));
        }
    }
    None
}

fn detect_version(process: &str) -> Option<String> {
    let output = std::process::Command::new(process)
        .args(["--version"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(rest) = trimmed
            .strip_prefix(process)
            .filter(|r| !r.is_empty())
        {
            return Some(rest.trim().to_string());
        }
        if let Some(rest) = trimmed
            .strip_prefix(&format!("{process} "))
        {
            return Some(rest.to_string());
        }
        if let Some(rest) = trimmed
            .strip_prefix(&format!("{process} version "))
        {
            return rest.split_whitespace().next().map(str::to_string);
        }
        return Some(trimmed.to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{detect_ecosystem_info, parse_gtk_settings};

    #[test]
    fn parses_gtk_settings() {
        let raw = r#"
[Settings]
gtk-theme-name=catppuccin-mocha-blue-standard+default
gtk-icon-theme-name=Papirus-Dark
gtk-cursor-theme-name=Bibata-Modern-Ice
gtk-font-name=JetBrainsMono Nerd Font 11
"#;
        let info = parse_gtk_settings(raw);
        assert_eq!(
            info.gtk_theme.as_deref(),
            Some("catppuccin-mocha-blue-standard+default")
        );
        assert_eq!(info.icon_theme.as_deref(), Some("Papirus-Dark"));
        assert_eq!(info.cursor_theme.as_deref(), Some("Bibata-Modern-Ice"));
        assert_eq!(info.font.as_deref(), Some("JetBrainsMono Nerd Font 11"));
    }

    #[test]
    fn normalizes_gtk_font_spacing() {
        let raw = r#"
[Settings]
gtk-font-name=Noto Sans,  10
"#;
        let info = parse_gtk_settings(raw);
        assert_eq!(info.font.as_deref(), Some("Noto Sans, 10"));
    }

    #[test]
    fn detects_hyprland_ecosystem_processes() {
        let processes = HashSet::from([
            "hyprpaper".to_string(),
            "waybar".to_string(),
            "fuzzel".to_string(),
            "mako".to_string(),
            "hypridle".to_string(),
            "hyprlock".to_string(),
        ]);

        let ecosystem = detect_ecosystem_info(&processes);
        assert_eq!(ecosystem.wallpaper.as_ref().map(|(l, _)| l.as_str()), Some("hyprpaper"));
        assert_eq!(ecosystem.bar.as_ref().map(|(l, _)| l.as_str()), Some("waybar"));
        assert_eq!(ecosystem.launcher.as_ref().map(|(l, _)| l.as_str()), Some("fuzzel"));
        assert_eq!(ecosystem.notifications.as_ref().map(|(l, _)| l.as_str()), Some("mako"));
        assert_eq!(ecosystem.idle.as_ref().map(|(l, _)| l.as_str()), Some("hypridle"));
        assert_eq!(ecosystem.lock.as_ref().map(|(l, _)| l.as_str()), Some("hyprlock"));
    }
}
