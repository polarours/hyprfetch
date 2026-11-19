use std::env;
use std::io::{self, IsTerminal};

use crate::logo::{self, LogoTone};
use crate::model::{ColorMode, ModuleStatus, PaletteName};
use crate::theme;
use owo_colors::OwoColorize;

pub(super) fn colors_enabled(mode: ColorMode) -> bool {
    match mode {
        ColorMode::Always => true,
        ColorMode::Never => false,
        ColorMode::Auto => io::stdout().is_terminal() && env::var_os("NO_COLOR").is_none(),
    }
}

/// Accent index for a key label. Keys are grouped by topic so a screenshot
/// shows related facts in the same color; unknown keys use the muted tone.
fn key_accent(key: &str) -> Option<usize> {
    // render_key appends a colon in the header layout ("OS:"), so match on
    // the bare label. Without trimming, every key fell through to the muted
    // tone and the whole output looked gray.
    let key = key.strip_suffix(':').unwrap_or(key);
    match key {
        "OS" | "Host" | "Kernel" => Some(0),
        "Uptime" | "Shell" | "Terminal" => Some(1),
        "CPU" | "GPU" | "Memory" | "Disk" => Some(2),
        "Battery" | "Locale" => Some(3),
        "Network" | "Temperature" | "Audio" => Some(4),
        "Hyprland" | "Workspace" | "Clients" | "Focused" => Some(5),
        "Session" | "Window" | "Workspaces" | "Monitors" => Some(6),
        "Theme" | "Ecosystem" | "Input" | "Colors" => Some(7),
        _ => None,
    }
}

pub(super) fn style_key(value: &str, palette: PaletteName, colors_enabled: bool) -> String {
    if !colors_enabled {
        return value.to_string();
    }
    let colors = palette.colors();
    if colors.mono {
        return value.bold().to_string();
    }
    let color = key_accent(value)
        .map(|index| colors.accents[index])
        .unwrap_or(colors.muted);
    format!("{}{}\x1b[0m", theme::sgr(color), value)
}

pub(super) fn style_value(
    value: &str,
    status: ModuleStatus,
    palette: PaletteName,
    colors_enabled: bool,
) -> String {
    if !colors_enabled {
        return value.to_string();
    }
    let colors = palette.colors();
    if colors.mono {
        return match status {
            ModuleStatus::Ok => value.to_string(),
            _ => value.dimmed().to_string(),
        };
    }
    let color = match status {
        ModuleStatus::Unavailable => colors.muted.to_string(),
        ModuleStatus::Partial => colors.accents[1].to_string(),
        // Good values use a bright near-white tone (derived from the theme's
        // foreground) so they read like fastfetch's values in a screenshot.
        ModuleStatus::Ok => theme::value_color(colors.foreground, colors.background),
    };
    format!("{}{}\x1b[0m", theme::sgr(&color), value)
}

pub(super) fn style_title(value: &str, palette: PaletteName, colors_enabled: bool) -> String {
    if !colors_enabled {
        return value.to_string();
    }
    let colors = palette.colors();
    if colors.mono {
        return value.bold().to_string();
    }
    format!("{}{}\x1b[0m", theme::sgr(colors.accents[0]), value)
}

pub(super) fn style_rule(value: &str, palette: PaletteName, colors_enabled: bool) -> String {
    if !colors_enabled {
        return value.to_string();
    }
    let colors = palette.colors();
    if colors.mono {
        return value.dimmed().to_string();
    }
    format!("{}{}\x1b[0m", theme::sgr(colors.muted), value)
}

/// Width in display columns of a usage meter bar (10 cells plus a leading space).
pub(super) const METER_WIDTH: usize = 11;
const METER_CELLS: usize = 10;

/// Render a compact usage bar: filled block glyphs colored green → gold → red
/// as usage rises, with the unfilled cells in the muted tone. fastfetch draws
/// these next to CPU/GPU/memory values; the color makes load readable at a
/// glance in a screenshot. Without colors the bar is still drawn in plain text.
pub(super) fn style_meter(percent: f64, palette: PaletteName, colors_enabled: bool) -> String {
    let percent = percent.clamp(0.0, 100.0);
    let filled = (percent / 100.0 * METER_CELLS as f64).round() as usize;
    let filled = filled.min(METER_CELLS);
    let empty = METER_CELLS - filled;

    let mut bar = String::from(" ");
    if !colors_enabled {
        bar.push_str(&"#".repeat(filled));
        bar.push_str(&".".repeat(empty));
        return bar;
    }

    let colors = palette.colors();
    // green under 60%, gold under 85%, red at or above 85%.
    let fill_color = if percent < 60.0 {
        colors.accents[2]
    } else if percent < 85.0 {
        colors.accents[1]
    } else {
        colors.accents[7]
    };
    bar.push_str(&format!(
        "{}{}\x1b[0m{}{}\x1b[0m",
        theme::sgr(fill_color),
        "█".repeat(filled),
        theme::sgr(colors.muted),
        "░".repeat(empty)
    ));
    bar
}

pub(super) fn color_bar(bright: bool, max_width: Option<usize>, palette: PaletteName) -> String {
    let colors = palette.colors();
    // The dim bar opens with the muted tone, then walks the accents; the
    // bright bar opens with the blue accent and walks the rest.
    let mut palette_colors: Vec<&str> = if bright {
        colors.accents.to_vec()
    } else {
        std::iter::once(colors.muted)
            .chain(colors.accents[..7].iter().copied())
            .collect()
    };
    if let Some(width) = max_width {
        let keep = (width / 4).min(palette_colors.len());
        palette_colors.truncate(keep);
    }

    let bar = palette_colors
        .into_iter()
        .map(|color| format!("{}███\x1b[0m", theme::sgr(color)))
        .collect::<Vec<_>>()
        .join(" ");
    format!(" {bar}")
}

pub(super) fn style_logo(
    value: &str,
    definition: logo::LogoDefinition,
    palette: PaletteName,
    colors_enabled: bool,
    line_index: usize,
    line_count: usize,
) -> String {
    if !colors_enabled {
        return value.to_string();
    }
    if palette.colors().mono {
        return value.bold().to_string();
    }

    let tone = logo::tone_for_line(definition, palette, line_index, line_count);
    let colors = palette.colors();
    let color = match tone {
        LogoTone::Blue => colors.accents[0],
        LogoTone::Cyan => colors.accents[4],
        LogoTone::Green => colors.accents[2],
        LogoTone::Indigo => colors.accents[3],
        LogoTone::Magenta => colors.accents[7],
        LogoTone::Red => colors.accents[6],
        LogoTone::Yellow => colors.accents[1],
        LogoTone::White => colors.foreground,
    };
    format!("{}{}\x1b[0m", theme::sgr(color), value)
}

#[cfg(test)]
mod tests {
    use unicode_width::UnicodeWidthStr;

    use super::{METER_WIDTH, color_bar, style_meter};
    use crate::model::PaletteName;

    /// Visible width of a string after removing SGR escape sequences.
    fn visible_width(value: &str) -> usize {
        let mut plain = String::new();
        let mut chars = value.chars();
        while let Some(character) = chars.next() {
            if character == '\x1b' {
                for tail in chars.by_ref() {
                    if tail == 'm' {
                        break;
                    }
                }
            } else {
                plain.push(character);
            }
        }
        UnicodeWidthStr::width(plain.as_str())
    }

    #[test]
    fn color_bar_fits_the_available_width() {
        // A full bar is 32 columns. When the terminal offers less, the bar
        // must drop cells instead of overflowing the terminal.
        assert_eq!(
            visible_width(&color_bar(false, None, PaletteName::Hypr)),
            32
        );
        for budget in [4, 8, 12, 20, 31] {
            let bar = color_bar(false, Some(budget), PaletteName::Hypr);
            assert!(
                visible_width(&bar) <= budget,
                "bar {bar:?} is wider than {budget}"
            );
        }
    }

    #[test]
    fn header_keys_with_colon_get_the_accent_color() {
        // render_key produces "OS:" in the header layout. The key must still
        // resolve to its accent, not fall through to the muted tone.
        use super::style_key;
        let plain = style_key("OS", PaletteName::Hypr, true);
        let with_colon = style_key("OS:", PaletteName::Hypr, true);
        // Both are wrapped in the same accent color; only the label text differs.
        let color_prefix = |styled: &str| styled.split("OS").next().unwrap().to_string();
        assert_eq!(
            color_prefix(&with_colon),
            color_prefix(&plain),
            "a trailing colon must not change the key color"
        );
        let muted = style_key("SomeUnknownKey", PaletteName::Hypr, true);
        assert_ne!(
            color_prefix(&with_colon),
            muted.split("SomeUnknownKey").next().unwrap(),
            "known keys must not use the muted tone"
        );
    }

    #[test]
    fn meter_fill_scales_with_percent() {
        let low = style_meter(0.0, PaletteName::Hypr, true);
        let half = style_meter(50.0, PaletteName::Hypr, true);
        let full = style_meter(100.0, PaletteName::Hypr, true);
        assert_eq!(low.matches('█').count(), 0);
        assert_eq!(half.matches('█').count(), 5);
        assert_eq!(full.matches('█').count(), 10);
        assert_eq!(full.matches('░').count(), 0);
    }

    #[test]
    fn meter_uses_green_gold_red_by_threshold() {
        let green = style_meter(30.0, PaletteName::Hypr, true);
        let gold = style_meter(70.0, PaletteName::Hypr, true);
        let red = style_meter(95.0, PaletteName::Hypr, true);
        // hypr accents: green index 2, gold index 1, red/pink index 7.
        let sgr = |color: &str| {
            let hex = color.trim_start_matches('#');
            let channel = |at: usize| u8::from_str_radix(&hex[at..at + 2], 16).unwrap();
            format!("\x1b[38;2;{};{};{}m", channel(0), channel(2), channel(4))
        };
        assert!(green.contains(&sgr("#9ece6a")));
        assert!(gold.contains(&sgr("#e0af68")));
        assert!(red.contains(&sgr("#f7768e")));
    }

    #[test]
    fn meter_plain_mode_and_width() {
        let plain = style_meter(50.0, PaletteName::Hypr, false);
        assert_eq!(plain, " #####.....", "plain meter uses ASCII glyphs");
        let colored = style_meter(50.0, PaletteName::Hypr, true);
        assert_eq!(
            visible_width(&colored),
            METER_WIDTH,
            "meter must reserve a fixed display width"
        );
    }

    #[test]
    fn color_bar_uses_the_selected_theme() {
        let hypr = color_bar(true, None, PaletteName::Hypr);
        let gruvbox = color_bar(true, None, PaletteName::Gruvbox);
        assert_ne!(hypr, gruvbox, "themes must produce different color bars");
        // both are still 32 columns wide
        assert_eq!(visible_width(&hypr), 32);
        assert_eq!(visible_width(&gruvbox), 32);
    }
}
