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
    // render_key appends a colon in the header layouts ("OS:") and pads with
    // leading spaces in the right-aligned minimal layout (" OS"). Without
    // trimming both, the label falls through to the muted tone and the whole
    // row looks gray.
    let key = key.trim_start();
    let key = key.strip_suffix(':').unwrap_or(key);
    match key {
        "OS" | "Host" | "Kernel" => Some(0),
        "Uptime" | "Shell" | "Terminal" => Some(1),
        "CPU" | "GPU" | "Memory" | "Disk" => Some(2),
        "Battery" | "Locale" => Some(3),
        "Network" | "Temperature" | "Audio" => Some(4),
        "Hyprland" | "Workspace" | "Clients" | "Focused" => Some(5),
        "Session" | "Window" | "Workspaces" | "Monitors" => Some(6),
        "Theme" | "Wallpaper" | "Ecosystem" | "Input" | "Colors" => Some(7),
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

/// Colour the load and size figures inside a value (a `53%`, an `8.1 GiB`, a
/// `72°C`) so numbers read like game readouts instead of blending into prose.
/// Everything else keeps the base tone.
fn highlight_figures(value: &str, accent: &str, base: &str) -> String {
    const UNITS: &[&str] = &["GiB", "MiB", "KiB", "GB", "MB", "KB", "TB", "°C"];
    let chars: Vec<char> = value.chars().collect();
    let mut out = String::new();
    let mut plain = String::new();
    let flush_plain = |out: &mut String, plain: &mut String| {
        if !plain.is_empty() {
            out.push_str(&format!("{}{}\x1b[0m", base, plain));
            plain.clear();
        }
    };

    let mut i = 0;
    while i < chars.len() {
        if !chars[i].is_ascii_digit() {
            plain.push(chars[i]);
            i += 1;
            continue;
        }
        // Read a number, keeping any decimal point.
        let start = i;
        while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
            i += 1;
        }
        let mut end = i;
        while end > start && chars[end - 1] == '.' {
            end -= 1;
        }

        let percent = end < chars.len() && chars[end] == '%';
        let unit = if percent {
            false
        } else {
            let mut j = end;
            while j < chars.len() && chars[j] == ' ' {
                j += 1;
            }
            UNITS.iter().any(|unit| {
                let len = unit.chars().count();
                j + len <= chars.len() && chars[j..j + len].iter().copied().eq(unit.chars())
            })
        };

        if percent || unit {
            flush_plain(&mut out, &mut plain);
            let mut figure: String = chars[start..end].iter().collect();
            if percent {
                // Keep the sign with the number so "53%" reads as one figure.
                figure.push('%');
                i = end + 1;
            } else {
                i = end;
            }
            out.push_str(&format!("{}{}\x1b[0m", accent, figure));
        } else {
            plain.extend(chars[start..i].iter());
        }
    }
    flush_plain(&mut out, &mut plain);
    out
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
        // foreground) so they read like fastfetch's values in a screenshot,
        // with the actual figures picked out in the theme accent.
        ModuleStatus::Ok => {
            // value_color returns a hex string; the renderer needs SGR escapes
            // so the literal hex never leaks into the output.
            let base = theme::sgr(&theme::value_color(colors.foreground, colors.background));
            let accent = theme::sgr(colors.accents[0]);
            return highlight_figures(value, &accent, &base);
        }
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
pub(super) const METER_WIDTH: usize = 15;
const METER_CELLS: usize = 14;

/// Render a compact usage bar: filled block glyphs colored green → gold → red
/// as usage rises, with the unfilled cells in the muted tone. fastfetch draws
/// these next to CPU/GPU/memory values; the color makes load readable at a
/// glance in a screenshot. Without colors the bar is still drawn in plain text.
pub(super) fn style_meter(
    percent: f64,
    palette: PaletteName,
    colors_enabled: bool,
    retro: bool,
) -> String {
    let percent = percent.clamp(0.0, 100.0);
    let filled = (percent / 100.0 * METER_CELLS as f64).round() as usize;
    let filled = filled.min(METER_CELLS);
    let empty = METER_CELLS - filled;

    let mut bar = String::from(" ");
    if !colors_enabled {
        if retro {
            // Same segmented cells without colour: lit versus hollow blocks
            // still carry the ratio.
            bar.push_str(&"\u{25ae}".repeat(filled));
            bar.push_str(&"\u{25af}".repeat(empty));
        } else {
            bar.push_str(&"#".repeat(filled));
            bar.push_str(&".".repeat(empty));
        }
        return bar;
    }

    if retro {
        // Arcade LED energy cells: lit cells glow in the identity blue and the
        // spent cells stay dim, like a segmented level meter on a coin-op.
        let colors = palette.colors();
        bar.push_str(&format!(
            "{}{}\x1b[0m{}{}\x1b[0m",
            theme::sgr(colors.accents[0]),
            "\u{25ae}".repeat(filled),
            theme::sgr(colors.muted),
            "\u{25af}".repeat(empty)
        ));
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
        if retro { "▰" } else { "█" }.repeat(filled),
        theme::sgr(colors.muted),
        if retro { "▱" } else { "░" }.repeat(empty)
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

    use super::{METER_WIDTH, color_bar, highlight_figures, key_accent, style_key, style_meter};
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
    fn padded_keys_keep_their_accent_color() {
        // The right-aligned minimal layout pads short keys with leading
        // spaces (" OS"); the padding must not hide the label from its
        // accent color.
        use super::style_key;
        let plain = style_key("OS", PaletteName::Hypr, true);
        let padded = style_key(" OS", PaletteName::Hypr, true);
        let color_of = |styled: &str| styled.split('m').next().unwrap_or_default().to_string();
        assert_eq!(color_of(&padded), color_of(&plain));
        let muted = style_key(" SomeUnknownKey", PaletteName::Hypr, true);
        assert_ne!(color_of(&padded), color_of(&muted));
    }

    #[test]
    fn theme_rows_share_one_accent_group() {
        // These four rows come from the theme/ecosystem modules and are
        // grouped visually in the screenshot; Wallpaper was missing from the
        // mapping and fell through to the muted gray.
        for key in ["Theme", "Wallpaper", "Ecosystem", "Input"] {
            assert_eq!(
                key_accent(key),
                Some(7),
                "{key} must share the theme accent"
            );
            assert_eq!(key_accent(&format!("{key}:")), Some(7));
        }
        // The style path resolves the same index for every grouped key, in
        // every palette (all palettes carry eight accents).
        for &palette in crate::theme::PaletteName::all() {
            let styled = style_key("Wallpaper", palette, true);
            assert!(
                styled.starts_with("\x1b["),
                "wallpaper must be colored in {palette:?}"
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
        let low = style_meter(0.0, PaletteName::Hypr, true, false);
        let half = style_meter(50.0, PaletteName::Hypr, true, false);
        let full = style_meter(100.0, PaletteName::Hypr, true, false);
        assert_eq!(low.matches('█').count(), 0);
        assert_eq!(half.matches('█').count(), 7);
        assert_eq!(full.matches('█').count(), 14);
        assert_eq!(full.matches('░').count(), 0);
    }

    #[test]
    fn meter_uses_green_gold_red_by_threshold() {
        let green = style_meter(30.0, PaletteName::Hypr, true, false);
        let gold = style_meter(70.0, PaletteName::Hypr, true, false);
        let red = style_meter(95.0, PaletteName::Hypr, true, false);
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
        let plain = style_meter(50.0, PaletteName::Hypr, false, false);
        assert_eq!(plain, " #######.......", "plain meter uses ASCII glyphs");
        let colored = style_meter(50.0, PaletteName::Hypr, true, false);
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

    #[test]
    fn retro_meter_is_an_arcade_led_meter() {
        // Lit cells for the load, hollow cells for the remainder (14 cells).
        let lit = "\u{25ae}".repeat(14);
        let dim = "\u{25af}".repeat(14);
        assert_eq!(
            style_meter(0.0, PaletteName::Arcade, false, true),
            format!(" {dim}")
        );
        assert_eq!(
            style_meter(100.0, PaletteName::Arcade, false, true),
            format!(" {lit}")
        );
        assert_eq!(
            style_meter(50.0, PaletteName::Arcade, false, true),
            format!(" {}{}", "\u{25ae}".repeat(7), "\u{25af}".repeat(7))
        );
        // Every frame keeps the same visible width so rows stay aligned.
        for percent in [0.0, 17.0, 33.3, 50.0, 66.6, 84.2, 100.0] {
            assert_eq!(
                UnicodeWidthStr::width(
                    style_meter(percent, PaletteName::Arcade, false, true).as_str()
                ),
                METER_WIDTH,
                "led meter changed width at {percent}%"
            );
        }
    }

    #[test]
    fn figures_are_picked_out_in_the_accent() {
        let accent = "\x1b[38;2;1;2;3m";
        let base = "\x1b[38;2;4;5;6m";
        let styled = highlight_figures("8.1 GiB / 15 GiB (53%)", accent, base);
        assert!(
            styled.contains(&format!("{accent}8.1\x1b[0m")),
            "size figure: {styled}"
        );
        assert!(
            styled.contains(&format!("{accent}53%\x1b[0m")),
            "percent figure: {styled}"
        );
        // Units and prose keep the base tone.
        assert!(
            !styled.contains(&format!("{accent}GiB")),
            "unit must not be accented"
        );
        // A version string is not a measurement and stays plain.
        let plain = highlight_figures("7.3.5-arch1-1", accent, base);
        assert!(
            !plain.contains(accent),
            "version numbers must stay plain: {plain}"
        );
    }
}
