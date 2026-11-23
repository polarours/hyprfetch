use crate::config::RuntimeConfig;
use crate::logo::{self};
use crate::model::{ModuleOutput, OutputRow, RenderStyle};
use crate::util::truncate_visible;
use unicode_width::UnicodeWidthStr;

use super::style::METER_WIDTH;
use super::style::color_bar;
use super::style::colors_enabled;
use super::style::style_key;
use super::style::style_logo;
use super::style::style_meter;
use super::style::style_rule;
use super::style::style_title;
use super::style::style_value;
use super::{
    DENSE_PRIORITY_WIDTH, DENSE_SEPARATOR, InfoRenderOptions, MIN_COLUMN_VALUE_WIDTH,
    MIN_SPLIT_VALUE_WIDTH, RenderRow, header_title, header_width, output_label, output_priority,
};

pub(super) fn render_text_with_width(
    outputs: &[ModuleOutput],
    runtime: &RuntimeConfig,
    terminal_width: Option<usize>,
) -> (String, Option<usize>) {
    let colors_enabled = colors_enabled(runtime.appearance.color);
    let is_minimal = runtime.appearance.style == RenderStyle::Minimal;
    let use_logo = !runtime.is_plain() && runtime.appearance.style == RenderStyle::Split;
    let separator_width = UnicodeWidthStr::width(runtime.appearance.separator.as_str());

    if use_logo {
        let rows = flatten_rows(outputs);
        if rows.is_empty() {
            return (String::new(), None);
        }
        let key_width = max_key_width(&rows, true);
        // A custom logo file supplies its own artwork; use the default tone
        // set so it is colored from the active palette like built-in logos.
        let logo_definition = if runtime.custom_logo.is_some() {
            logo::definition(logo::LogoPreset::Hypr)
        } else {
            logo::definition(runtime.resolved_logo)
        };
        let raw_logo = match runtime.custom_logo.as_deref() {
            Some(art) => logo::custom_lines(art),
            None => logo_definition.lines(),
        };
        if !raw_logo.is_empty() {
            let split_padding = runtime.appearance.padding + 1;

            // the split layout writes the key with a colon behind it
            let split_base_width = |logo: &[String]| {
                logo_width_of(logo) + split_padding + key_width + 1 + separator_width
            };

            let full_base_width = split_base_width(&raw_logo);
            if has_split_space(terminal_width, full_base_width) {
                let (rendered, logo_width) = render_split(
                    rows,
                    raw_logo,
                    runtime,
                    colors_enabled,
                    logo_definition,
                    key_width,
                    split_value_width(terminal_width, full_base_width),
                );
                return (rendered, Some(logo_width + split_padding.saturating_sub(2)));
            }

            // The full drawing is too wide, so try the narrower one before
            // giving the logo up altogether.
            let compact_logo = logo_definition.compact_lines();
            if !compact_logo.is_empty() {
                let compact_base_width = split_base_width(&compact_logo);
                if has_split_space(terminal_width, compact_base_width) {
                    let (rendered, logo_width) = render_split(
                        rows,
                        compact_logo,
                        runtime,
                        colors_enabled,
                        logo_definition,
                        key_width,
                        split_value_width(terminal_width, compact_base_width),
                    );
                    return (rendered, Some(logo_width + split_padding.saturating_sub(2)));
                }
            }
        }
    }

    let filtered_outputs = prioritize_outputs_for_width(outputs, terminal_width);
    let rows = flatten_rows(&filtered_outputs);
    if rows.is_empty() {
        return (String::new(), None);
    }

    let key_width = max_key_width(&rows, !runtime.is_plain());

    if is_minimal {
        if has_column_space(terminal_width, key_width, separator_width) {
            return (
                render_minimal(rows, runtime, colors_enabled, terminal_width, key_width),
                None,
            );
        }

        // The key and a value do not fit next to each other, so the rows are
        // written one under the other.
        return (
            render_dense(rows, runtime, colors_enabled, terminal_width),
            None,
        );
    }

    let key_suffix_width = usize::from(!runtime.is_plain());
    if has_column_space(terminal_width, key_width, separator_width) {
        return (
            render_column(
                rows,
                runtime,
                colors_enabled,
                key_width,
                column_value_width(terminal_width, key_width, key_suffix_width, separator_width),
            ),
            None,
        );
    }

    (
        render_dense(rows, runtime, colors_enabled, terminal_width),
        None,
    )
}

pub(super) fn render_split(
    rows: Vec<RenderRow>,
    raw_logo: Vec<String>,
    runtime: &RuntimeConfig,
    colors_enabled: bool,
    logo_definition: logo::LogoDefinition,
    key_width: usize,
    value_width: Option<usize>,
) -> (String, usize) {
    let logo_width = raw_logo
        .iter()
        .map(|line| UnicodeWidthStr::width(line.as_str()))
        .max()
        .unwrap_or(0);
    let right_width = value_width.map(|width| {
        key_width + 1 + UnicodeWidthStr::width(runtime.appearance.separator.as_str()) + width
    });
    let right_lines = render_info_lines(
        rows,
        runtime,
        colors_enabled,
        InfoRenderOptions {
            key_width,
            value_width,
            // the title is indented by one against the keys
            max_line_width: right_width.and_then(|width| width.checked_sub(1)),
            include_header: true,
        },
    );
    let total_lines = raw_logo.len().max(right_lines.len());
    let mut output = String::new();

    for index in 0..total_lines {
        let logo_raw = raw_logo.get(index).map(String::as_str).unwrap_or("");
        let logo_padded = format!("{logo_raw:<logo_width$}");
        let logo = style_logo(
            &logo_padded,
            logo_definition,
            runtime.appearance.palette,
            colors_enabled,
            index,
            raw_logo.len(),
        );

        if let Some(line) = right_lines.get(index) {
            output.push_str(&format!(
                "{logo}{}{}\n",
                " ".repeat(runtime.appearance.padding + 1),
                line
            ));
        } else {
            output.push_str(&format!("{logo}\n"));
        }
    }

    (output, logo_width + runtime.appearance.padding + 1)
}

fn render_column(
    rows: Vec<RenderRow>,
    runtime: &RuntimeConfig,
    colors_enabled: bool,
    key_width: usize,
    value_width: Option<usize>,
) -> String {
    let right_width = value_width.map(|width| {
        let key_suffix_width = usize::from(!runtime.is_plain());
        key_width
            + key_suffix_width
            + UnicodeWidthStr::width(runtime.appearance.separator.as_str())
            + width
    });
    render_info_lines(
        rows,
        runtime,
        colors_enabled,
        InfoRenderOptions {
            key_width,
            value_width,
            // the title is indented by one against the keys
            max_line_width: right_width.and_then(|width| width.checked_sub(1)),
            include_header: !runtime.is_plain(),
        },
    )
    .into_iter()
    .map(|line| format!("{line}\n"))
    .collect()
}

fn render_info_lines(
    rows: Vec<RenderRow>,
    runtime: &RuntimeConfig,
    colors_enabled: bool,
    options: InfoRenderOptions,
) -> Vec<String> {
    let mut lines = Vec::new();

    if options.include_header
        && let Some(title) = header_title()
    {
        let title_width = header_width(&title, options.max_line_width);
        let title = truncate_visible(&title, title_width);
        // A short ASCII dash rule matches the understated title underline
        // fastfetch uses in screenshots (rather than a full-width box rule).
        let rule = if runtime.retro { "═" } else { "-" }.repeat(title_width);
        lines.push(style_title(
            &format!(" {title}"),
            runtime.appearance.palette,
            colors_enabled,
        ));
        lines.push(style_rule(
            &format!(" {rule}"),
            runtime.appearance.palette,
            colors_enabled,
        ));
    }

    for row in rows {
        let key = render_key(&row.key, options);
        let styled_key = style_key(&key, runtime.appearance.palette, colors_enabled);
        // A meter sits to the right of the value; reserve its width so the
        // value text never pushes the combined line past the available width.
        let value_budget = options
            .value_width
            .map(|width| width.saturating_sub(if row.meter.is_some() { METER_WIDTH } else { 0 }));
        let value = truncate_row_value(&row.value, value_budget);
        let styled_value = style_value(
            &value,
            row.status,
            runtime.appearance.palette,
            colors_enabled,
        );
        let meter = row
            .meter
            .map(|percent| {
                style_meter(
                    percent,
                    runtime.appearance.palette,
                    colors_enabled,
                    runtime.retro,
                )
            })
            .unwrap_or_default();
        // style_meter already starts with one space, which is the whole gap
        // between value and bar; adding another would overflow the width
        // budget that only reserves METER_WIDTH.
        let pad = 0;
        lines.push(format!(
            "{}{}{}{}{}",
            styled_key,
            runtime.appearance.separator,
            styled_value,
            " ".repeat(pad),
            meter
        ));
    }

    if options.include_header && runtime.appearance.color_bars && colors_enabled {
        lines.push(String::new());
        lines.push(color_bar(
            false,
            options.max_line_width,
            runtime.appearance.palette,
        ));
        lines.push(color_bar(
            true,
            options.max_line_width,
            runtime.appearance.palette,
        ));
    }

    lines
}

fn render_dense(
    rows: Vec<RenderRow>,
    runtime: &RuntimeConfig,
    colors_enabled: bool,
    terminal_width: Option<usize>,
) -> String {
    let mut output = String::new();

    for row in rows {
        let key_budget = terminal_width.unwrap_or(usize::MAX);
        let key = truncate_visible(&row.key, key_budget);
        let key_width = UnicodeWidthStr::width(key.as_str());
        // When the key fills the terminal, saturating_sub yields Some(0) and
        // the value is dropped. checked_sub returned None here, which meant
        // "no limit" and let the value overflow the terminal entirely.
        let meter_width = if row.meter.is_some() { METER_WIDTH } else { 0 };
        let value_width = terminal_width.map(|width| {
            width.saturating_sub(key_width + UnicodeWidthStr::width(DENSE_SEPARATOR) + meter_width)
        });
        let value = truncate_row_value(&row.value, value_width);
        let styled_key = style_key(&key, runtime.appearance.palette, colors_enabled);
        let styled_value = style_value(
            &value,
            row.status,
            runtime.appearance.palette,
            colors_enabled,
        );
        let meter = row
            .meter
            .map(|percent| {
                style_meter(
                    percent,
                    runtime.appearance.palette,
                    colors_enabled,
                    runtime.retro,
                )
            })
            .unwrap_or_default();

        if value.is_empty() {
            output.push_str(&format!("{styled_key}\n"));
        } else {
            output.push_str(&format!(
                "{styled_key}{DENSE_SEPARATOR}{styled_value}{meter}\n"
            ));
        }
    }

    output
}

fn render_minimal(
    rows: Vec<RenderRow>,
    runtime: &RuntimeConfig,
    colors_enabled: bool,
    terminal_width: Option<usize>,
    key_width: usize,
) -> String {
    let separator_width = UnicodeWidthStr::width(runtime.appearance.separator.as_str());
    let value_width =
        terminal_width.and_then(|width| width.checked_sub(key_width + separator_width));
    render_info_lines(
        rows,
        runtime,
        colors_enabled,
        InfoRenderOptions {
            key_width,
            value_width,
            max_line_width: None,
            include_header: false,
        },
    )
    .into_iter()
    .map(|line| format!("{line}\n"))
    .collect()
}

fn flatten_rows(outputs: &[ModuleOutput]) -> Vec<RenderRow> {
    let mut rows = Vec::new();
    for output in outputs {
        for OutputRow { key, value, meter } in &output.rows {
            rows.push(RenderRow {
                key: key.clone(),
                value: value.clone(),
                status: output.status,
                meter: *meter,
            });
        }
    }

    if !rows.is_empty() {
        return rows;
    }

    outputs
        .iter()
        .map(|output| RenderRow {
            key: output_label(&output.id),
            value: output
                .message
                .clone()
                .unwrap_or_else(|| "unavailable".to_string()),
            status: output.status,
            meter: None,
        })
        .collect()
}

fn render_key(key: &str, options: InfoRenderOptions) -> String {
    // format!'s width counts Unicode scalar values, not display columns, so a
    // wide key would be padded too little and misalign the values.
    let key_width = UnicodeWidthStr::width(key);
    if options.include_header {
        // Arcade button labels: `[OS]` in upper case, padded so every value
        // starts in the same column. Square brackets are plain ASCII, so the
        // width budget stays exact on every terminal (unlike `▸`/`»`, which
        // are ambiguous-width and shift the layout when drawn two columns wide).
        let label_width = key_width + 2;
        let padding = options.key_width.saturating_sub(label_width);
        format!("[{}]{}", key.to_uppercase(), " ".repeat(padding))
    } else {
        let padding = options.key_width.saturating_sub(key_width);
        format!("{}{}", " ".repeat(padding), key)
    }
}

fn prioritize_outputs_for_width(
    outputs: &[ModuleOutput],
    terminal_width: Option<usize>,
) -> Vec<ModuleOutput> {
    let Some(priority_ceiling) = dense_priority_ceiling(terminal_width) else {
        return outputs.to_vec();
    };

    let filtered = outputs
        .iter()
        .filter(|output| output_priority(&output.id) <= priority_ceiling)
        .cloned()
        .collect::<Vec<_>>();

    if filtered.is_empty() {
        outputs.to_vec()
    } else {
        filtered
    }
}

fn dense_priority_ceiling(terminal_width: Option<usize>) -> Option<usize> {
    match terminal_width {
        Some(width) if width <= 18 => Some(0),
        Some(width) if width <= DENSE_PRIORITY_WIDTH => Some(1),
        _ => None,
    }
}

fn max_key_width(rows: &[RenderRow], include_header: bool) -> usize {
    let brackets = if include_header { 2 } else { 0 };
    rows.iter()
        .map(|row| UnicodeWidthStr::width(row.key.as_str()))
        .max()
        .unwrap_or(0)
        + brackets
}

fn has_split_space(terminal_width: Option<usize>, split_base_width: usize) -> bool {
    terminal_width
        .map(|width| width >= split_base_width + MIN_SPLIT_VALUE_WIDTH)
        .unwrap_or(true)
}

fn has_column_space(
    terminal_width: Option<usize>,
    key_width: usize,
    separator_width: usize,
) -> bool {
    terminal_width
        .map(|width| width >= key_width + separator_width + MIN_COLUMN_VALUE_WIDTH)
        .unwrap_or(true)
}

/// The width of the widest line of a logo.
fn logo_width_of(lines: &[String]) -> usize {
    lines
        .iter()
        .map(|line| UnicodeWidthStr::width(line.as_str()))
        .max()
        .unwrap_or(0)
}

fn split_value_width(terminal_width: Option<usize>, split_base_width: usize) -> Option<usize> {
    terminal_width.and_then(|width| width.checked_sub(split_base_width))
}

fn column_value_width(
    terminal_width: Option<usize>,
    key_width: usize,
    key_suffix_width: usize,
    separator_width: usize,
) -> Option<usize> {
    terminal_width
        .and_then(|width| width.checked_sub(key_width + key_suffix_width + separator_width))
}

fn truncate_row_value(value: &str, value_width: Option<usize>) -> String {
    match value_width {
        Some(width) => truncate_visible(value, width),
        None => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AppearanceConfig, ModuleConfig, RuntimeConfig};
    use crate::logo::LogoPreset;
    use crate::model::{
        ColorMode, ModuleId, ModuleOutput, ModuleStatus, OutputRow, PaletteName, RenderStyle,
    };
    use serde_json::json;

    #[test]
    fn renders_plain_column_output() {
        let runtime = RuntimeConfig {
            appearance: AppearanceConfig {
                style: RenderStyle::Compact,
                palette: PaletteName::Hypr,
                color: ColorMode::Never,
                plain: true,
                color_bars: false,
                padding: 1,
                separator: "  ".to_string(),
            },
            logo: LogoPreset::None,
            resolved_logo: LogoPreset::None,
            custom_logo: None,
            modules: vec![ModuleConfig::from(ModuleId::Os)],
            plugins: Vec::new(),
            json: false,
            fast: false,
            diagnostics: false,
            doctor: false,
            retro: false,
            animation: crate::animation::RevealStyle::Static,
            watch: false,
            watch_interval: 1,
        };

        let outputs = vec![
            ModuleOutput::ok(
                ModuleId::Os,
                vec![OutputRow::new("OS", "Arch Linux")],
                json!({"name": "Arch Linux"}),
            ),
            ModuleOutput {
                id: ModuleId::Hyprland.to_string(),
                status: ModuleStatus::Unavailable,
                rows: Vec::new(),
                message: Some("socket unavailable".to_string()),
                data: serde_json::Value::Null,
            },
        ];

        let rendered = render_text_with_width(&outputs, &runtime, None).0;
        assert_eq!(rendered, "OS  Arch Linux\n");
    }

    #[test]
    fn minimal_layout_aligns_wide_character_keys() {
        let runtime = RuntimeConfig {
            appearance: AppearanceConfig {
                style: RenderStyle::Minimal,
                palette: PaletteName::Hypr,
                color: ColorMode::Never,
                plain: false,
                color_bars: false,
                padding: 1,
                separator: "  ".to_string(),
            },
            logo: LogoPreset::None,
            resolved_logo: LogoPreset::None,
            custom_logo: None,
            modules: vec![ModuleConfig::from(ModuleId::Os)],
            plugins: Vec::new(),
            json: false,
            fast: false,
            diagnostics: false,
            doctor: false,
            retro: false,
            animation: crate::animation::RevealStyle::Static,
            watch: false,
            watch_interval: 1,
        };
        let outputs = vec![ModuleOutput::ok(
            ModuleId::Os,
            vec![OutputRow::new("OS", "Arch"), OutputRow::new("工作区", "1")],
            json!({}),
        )];

        let rendered = render_text_with_width(&outputs, &runtime, None).0;
        let lines: Vec<&str> = rendered.lines().collect();
        // Both values must start at the same display column even though the
        // CJK key is three chars wide and six columns wide.
        let os_col = UnicodeWidthStr::width(lines[0].split("Arch").next().unwrap());
        let cjk_col = UnicodeWidthStr::width(lines[1].split('1').next().unwrap());
        assert_eq!(os_col, cjk_col, "misaligned values: {rendered:?}");
    }

    #[test]
    fn dense_layout_keeps_a_long_key_inside_the_terminal() {
        let runtime = RuntimeConfig {
            appearance: AppearanceConfig {
                style: RenderStyle::Compact,
                palette: PaletteName::Hypr,
                color: ColorMode::Never,
                plain: true,
                color_bars: false,
                padding: 1,
                separator: "  ".to_string(),
            },
            logo: LogoPreset::None,
            resolved_logo: LogoPreset::None,
            custom_logo: None,
            modules: vec![ModuleConfig::from(ModuleId::Os)],
            plugins: Vec::new(),
            json: false,
            fast: false,
            diagnostics: false,
            doctor: false,
            retro: false,
            animation: crate::animation::RevealStyle::Static,
            watch: false,
            watch_interval: 1,
        };
        let outputs = vec![ModuleOutput::ok(
            ModuleId::Os,
            vec![OutputRow::new("ABCDEFGHIJKLMNOP", "some value here")],
            json!({}),
        )];

        let rendered = render_text_with_width(&outputs, &runtime, Some(10)).0;
        // The key consumes the whole width, so the value must be cut to zero,
        // not left untruncated because checked_sub underflowed to None.
        for line in rendered.lines() {
            assert!(
                UnicodeWidthStr::width(line) <= 10,
                "line exceeds terminal width: {line:?}"
            );
        }
    }

    #[test]
    fn falls_back_to_column_when_split_layout_is_too_narrow() {
        let runtime = RuntimeConfig {
            appearance: AppearanceConfig {
                style: RenderStyle::Split,
                palette: PaletteName::Hypr,
                color: ColorMode::Never,
                plain: false,
                color_bars: false,
                padding: 1,
                separator: "  ".to_string(),
            },
            logo: LogoPreset::Arch,
            resolved_logo: LogoPreset::Arch,
            custom_logo: None,
            modules: vec![ModuleConfig::from(ModuleId::Os)],
            plugins: Vec::new(),
            json: false,
            fast: false,
            diagnostics: false,
            doctor: false,
            retro: false,
            animation: crate::animation::RevealStyle::Static,
            watch: false,
            watch_interval: 1,
        };
        let outputs = vec![ModuleOutput::ok(
            ModuleId::Os,
            vec![OutputRow::new("OS", "Arch Linux")],
            json!({"name": "Arch Linux"}),
        )];

        let rendered = render_text_with_width(&outputs, &runtime, Some(40)).0;
        // The exact padding depends on the column budget; assert the label and
        // value survive the fallback rather than a specific gap width.
        assert!(rendered.contains("[OS]"));
        assert!(rendered.contains("Arch Linux"));
        assert!(rendered.lines().count() >= 3);
    }

    #[test]
    fn falls_back_to_dense_layout_when_terminal_is_very_narrow() {
        let runtime = RuntimeConfig {
            appearance: AppearanceConfig {
                style: RenderStyle::Split,
                palette: PaletteName::Hypr,
                color: ColorMode::Never,
                plain: false,
                color_bars: false,
                padding: 1,
                separator: "  ".to_string(),
            },
            logo: LogoPreset::Arch,
            resolved_logo: LogoPreset::Arch,
            custom_logo: None,
            modules: vec![ModuleConfig::from(ModuleId::Theme)],
            plugins: Vec::new(),
            json: false,
            fast: false,
            diagnostics: false,
            doctor: false,
            retro: false,
            animation: crate::animation::RevealStyle::Static,
            watch: false,
            watch_interval: 1,
        };
        let outputs = vec![ModuleOutput::ok(
            ModuleId::Theme,
            vec![OutputRow::new("Theme", "gtk:Adwaita icons:breeze-dark")],
            json!({}),
        )];

        let rendered = render_text_with_width(&outputs, &runtime, Some(18)).0;
        assert_eq!(rendered, "Theme gtk:Adwaita…\n");
    }

    #[test]
    fn truncates_values_to_available_width_in_column_layout() {
        let runtime = RuntimeConfig {
            appearance: AppearanceConfig {
                style: RenderStyle::Compact,
                palette: PaletteName::Hypr,
                color: ColorMode::Never,
                plain: true,
                color_bars: false,
                padding: 1,
                separator: "  ".to_string(),
            },
            logo: LogoPreset::None,
            resolved_logo: LogoPreset::None,
            custom_logo: None,
            modules: vec![ModuleConfig::from(ModuleId::Theme)],
            plugins: Vec::new(),
            json: false,
            fast: false,
            diagnostics: false,
            doctor: false,
            retro: false,
            animation: crate::animation::RevealStyle::Static,
            watch: false,
            watch_interval: 1,
        };
        let outputs = vec![ModuleOutput::ok(
            ModuleId::Theme,
            vec![OutputRow::new("Theme", "gtk:Adwaita icons:breeze-dark")],
            json!({}),
        )];

        let rendered = render_text_with_width(&outputs, &runtime, Some(20)).0;
        // the cut moves back to the last space, so no half word is left behind
        assert_eq!(rendered, "Theme gtk:Adwaita…\n");
    }

    #[test]
    fn prioritizes_hyprland_and_core_modules_on_very_narrow_widths() {
        let outputs = vec![
            ModuleOutput::ok(
                ModuleId::Os,
                vec![OutputRow::new("OS", "Arch Linux")],
                json!({}),
            ),
            ModuleOutput::ok(
                ModuleId::Theme,
                vec![OutputRow::new("Theme", "gtk:Adwaita")],
                json!({}),
            ),
            ModuleOutput::ok(
                ModuleId::Wallpaper,
                vec![OutputRow::new("Wallpaper", "hyprpaper")],
                json!({}),
            ),
            ModuleOutput::ok(
                ModuleId::Hyprland,
                vec![OutputRow::new("Hyprland", "0.48.1")],
                json!({}),
            ),
        ];

        let prioritized = prioritize_outputs_for_width(&outputs, Some(18));
        let ids = prioritized
            .iter()
            .map(|output| output.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(ids, vec!["os", "hyprland"]);
    }

    #[test]
    fn keeps_theme_but_drops_wallpaper_on_narrow_dense_layout() {
        let outputs = vec![
            ModuleOutput::ok(
                ModuleId::Os,
                vec![OutputRow::new("OS", "Arch Linux")],
                json!({}),
            ),
            ModuleOutput::ok(
                ModuleId::Theme,
                vec![OutputRow::new("Theme", "gtk:Adwaita")],
                json!({}),
            ),
            ModuleOutput::ok(
                ModuleId::Wallpaper,
                vec![OutputRow::new("Wallpaper", "hyprpaper")],
                json!({}),
            ),
        ];

        let prioritized = prioritize_outputs_for_width(&outputs, Some(24));
        let ids = prioritized
            .iter()
            .map(|output| output.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(ids, vec!["os", "theme"]);
    }

    /// The rendered lines never reach past the terminal width.
    #[test]
    fn keeps_every_line_inside_the_terminal_width() {
        let runtime = RuntimeConfig {
            appearance: AppearanceConfig {
                style: RenderStyle::Split,
                palette: PaletteName::Hypr,
                color: ColorMode::Never,
                plain: false,
                color_bars: false,
                padding: 1,
                separator: "  ".to_string(),
            },
            logo: LogoPreset::Arch,
            resolved_logo: LogoPreset::Arch,
            custom_logo: None,
            modules: vec![ModuleConfig::from(ModuleId::Os)],
            plugins: Vec::new(),
            json: false,
            fast: false,
            diagnostics: false,
            doctor: false,
            retro: false,
            animation: crate::animation::RevealStyle::Static,
            watch: false,
            watch_interval: 1,
        };

        let outputs = vec![ModuleOutput::ok(
            ModuleId::Os,
            vec![OutputRow::new(
                "Ecosystem",
                "bar:quickshell (0.3.2) launch:walker (2.17.2) notify:swaync (0.12.6)",
            )],
            json!({}),
        )];

        for width in [40, 60, 80, 120, 200] {
            let rendered = render_text_with_width(&outputs, &runtime, Some(width)).0;
            for line in rendered.lines() {
                assert!(
                    UnicodeWidthStr::width(line) <= width,
                    "line is {} wide, the terminal is {width}: {line}",
                    UnicodeWidthStr::width(line)
                );
            }
        }
    }

    /// The minimal layout keeps its lines inside the terminal as well.
    #[test]
    fn keeps_the_minimal_layout_inside_the_terminal_width() {
        let runtime = RuntimeConfig {
            appearance: AppearanceConfig {
                style: RenderStyle::Minimal,
                palette: PaletteName::Hypr,
                color: ColorMode::Never,
                plain: false,
                color_bars: false,
                padding: 1,
                separator: "  ".to_string(),
            },
            logo: LogoPreset::None,
            resolved_logo: LogoPreset::None,
            custom_logo: None,
            modules: vec![ModuleConfig::from(ModuleId::Os)],
            plugins: Vec::new(),
            json: false,
            fast: false,
            diagnostics: false,
            doctor: false,
            retro: false,
            animation: crate::animation::RevealStyle::Static,
            watch: false,
            watch_interval: 1,
        };

        let outputs = vec![ModuleOutput::ok(
            ModuleId::Os,
            vec![OutputRow::new(
                "Ecosystem",
                "bar:quickshell (0.3.2) launch:walker (2.17.2) notify:swaync (0.12.6)",
            )],
            json!({}),
        )];

        for width in [30, 40, 55, 80, 120] {
            let rendered = render_text_with_width(&outputs, &runtime, Some(width)).0;
            for line in rendered.lines() {
                assert!(
                    UnicodeWidthStr::width(line) <= width,
                    "line is {} wide, the terminal is {width}: {line}",
                    UnicodeWidthStr::width(line)
                );
            }
        }
    }

    /// A narrow terminal keeps the logo by switching to the compact drawing.
    #[test]
    fn uses_the_compact_logo_when_the_full_one_is_too_wide() {
        let runtime = RuntimeConfig {
            appearance: AppearanceConfig {
                style: RenderStyle::Split,
                palette: PaletteName::Hypr,
                color: ColorMode::Never,
                plain: false,
                color_bars: false,
                padding: 1,
                separator: "  ".to_string(),
            },
            logo: LogoPreset::Arch,
            resolved_logo: LogoPreset::Arch,
            custom_logo: None,
            modules: vec![ModuleConfig::from(ModuleId::Os)],
            plugins: Vec::new(),
            json: false,
            fast: false,
            diagnostics: false,
            doctor: false,
            retro: false,
            animation: crate::animation::RevealStyle::Static,
            watch: false,
            watch_interval: 1,
        };

        let outputs = vec![ModuleOutput::ok(
            ModuleId::Os,
            vec![OutputRow::new("OS", "Arch Linux")],
            json!({}),
        )];

        let definition = logo::definition(LogoPreset::Arch);
        let full = definition.lines();
        let compact = definition.compact_lines();
        assert!(!compact.is_empty(), "the arch logo has a compact drawing");
        assert!(
            logo_width_of(&compact) < logo_width_of(&full),
            "the compact drawing is not narrower"
        );

        // the full drawing does not fit here, the compact one does
        let width = 60;
        let rendered = render_text_with_width(&outputs, &runtime, Some(width)).0;
        assert!(
            rendered.contains(compact.first().expect("the compact logo has a line")),
            "the logo is missing:\n{rendered}"
        );
        assert!(
            !rendered.contains(full.first().expect("the logo has a line")),
            "the full drawing is still used:\n{rendered}"
        );
    }

    #[test]
    fn the_reported_logo_width_hides_the_session_rows() {
        let runtime = RuntimeConfig {
            appearance: AppearanceConfig {
                style: RenderStyle::Split,
                palette: PaletteName::Hypr,
                color: ColorMode::Never,
                plain: false,
                color_bars: false,
                padding: 1,
                separator: "  ".to_string(),
            },
            logo: LogoPreset::Arch,
            resolved_logo: LogoPreset::Arch,
            custom_logo: None,
            modules: vec![
                ModuleConfig::from(ModuleId::Os),
                ModuleConfig::from(ModuleId::Cpu),
            ],
            plugins: Vec::new(),
            json: false,
            fast: false,
            diagnostics: false,
            doctor: false,
            retro: false,
            animation: crate::animation::RevealStyle::Static,
            watch: false,
            watch_interval: 1,
        };
        let outputs = vec![
            ModuleOutput::ok(
                ModuleId::Os,
                vec![OutputRow::new("OS", "Arch Linux")],
                json!({}),
            ),
            ModuleOutput::ok(ModuleId::Cpu, vec![OutputRow::new("CPU", "i7")], json!({})),
        ];

        let (rendered, logo_width) = render_text_with_width(&outputs, &runtime, Some(80));
        let width = logo_width.expect("a split layout reports the logo width");
        // The first info row carries `[OS]` on the right of the logo; cutting
        // at the reported width must leave only skeleton columns, so no
        // alphanumeric of the session row leaks into the first reveal phase.
        let row = rendered
            .lines()
            .find(|line| line.contains("[OS]"))
            .expect("the OS row is rendered");
        let skeleton = crate::animation::visible_prefix(row, width);
        assert!(
            !skeleton
                .trim_end()
                .chars()
                .any(|c| c.is_ascii_alphanumeric()),
            "session text leaked into the logo skeleton: {skeleton:?}"
        );
    }

    #[test]
    fn metered_rows_render_a_usage_bar() {
        let runtime = RuntimeConfig {
            appearance: AppearanceConfig {
                style: RenderStyle::Compact,
                palette: PaletteName::Hypr,
                color: ColorMode::Never,
                plain: true,
                color_bars: false,
                padding: 1,
                separator: "  ".to_string(),
            },
            logo: LogoPreset::None,
            resolved_logo: LogoPreset::None,
            custom_logo: None,
            modules: vec![ModuleConfig::from(ModuleId::Cpu)],
            plugins: Vec::new(),
            json: false,
            fast: false,
            diagnostics: false,
            doctor: false,
            retro: false,
            animation: crate::animation::RevealStyle::Static,
            watch: false,
            watch_interval: 1,
        };

        let outputs = vec![ModuleOutput::ok(
            ModuleId::Cpu,
            vec![OutputRow::metered("CPU", "i7 @ 25.4% (28 threads)", 25.4)],
            json!({}),
        )];

        let rendered = render_text_with_width(&outputs, &runtime, None).0;
        assert!(
            rendered.contains("###......."),
            "a quarter-full ASCII usage bar is expected:\n{rendered}"
        );
    }

    #[test]
    fn metered_row_does_not_change_value_alignment() {
        // reserved meter width keeps the value text from overflowing
        let budget = Some(40_usize).map(|w| w.saturating_sub(METER_WIDTH));
        assert_eq!(budget, Some(25));
    }

    #[test]
    fn header_keys_pad_to_one_width_so_values_align() {
        // Every bracket label is padded to the widest one, so the value always
        // starts in the same column.
        let options = InfoRenderOptions {
            key_width: 8,
            value_width: None,
            max_line_width: None,
            include_header: true,
        };
        assert_eq!(render_key("OS", options), "[OS]    ");
        assert_eq!(render_key("Kernel", options), "[KERNEL]");
        assert_eq!(
            UnicodeWidthStr::width(render_key("OS", options).as_str()),
            UnicodeWidthStr::width(render_key("Kernel", options).as_str())
        );
    }
}
