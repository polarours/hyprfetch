use crate::config::RuntimeConfig;
use crate::logo::{self};
use crate::model::{ModuleOutput, OutputRow, RenderStyle};
use crate::util::truncate_visible;
use unicode_width::UnicodeWidthStr;

use super::style::color_bar;
use super::style::colors_enabled;
use super::style::style_key;
use super::style::style_logo;
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
) -> String {
    let colors_enabled = colors_enabled(runtime.appearance.color);
    let is_minimal = runtime.appearance.style == RenderStyle::Minimal;
    let use_logo = !runtime.is_plain() && runtime.appearance.style == RenderStyle::Split;
    let separator_width = UnicodeWidthStr::width(runtime.appearance.separator.as_str());

    if use_logo {
        let rows = flatten_rows(outputs);
        if rows.is_empty() {
            return String::new();
        }

        let key_width = max_key_width(&rows);
        let logo_definition = logo::definition(runtime.resolved_logo);
        let raw_logo = logo_definition.lines();
        if !raw_logo.is_empty() {
            let split_padding = runtime.appearance.padding + 1;

            // the split layout writes the key with a colon behind it
            let split_base_width = |logo: &[String]| {
                logo_width_of(logo) + split_padding + key_width + 1 + separator_width
            };

            let full_base_width = split_base_width(&raw_logo);
            if has_split_space(terminal_width, full_base_width) {
                return render_split(
                    rows,
                    raw_logo,
                    runtime,
                    colors_enabled,
                    logo_definition,
                    key_width,
                    split_value_width(terminal_width, full_base_width),
                );
            }

            // The full drawing is too wide, so try the narrower one before
            // giving the logo up altogether.
            let compact_logo = logo_definition.compact_lines();
            if !compact_logo.is_empty() {
                let compact_base_width = split_base_width(&compact_logo);
                if has_split_space(terminal_width, compact_base_width) {
                    return render_split(
                        rows,
                        compact_logo,
                        runtime,
                        colors_enabled,
                        logo_definition,
                        key_width,
                        split_value_width(terminal_width, compact_base_width),
                    );
                }
            }
        }
    }

    let filtered_outputs = prioritize_outputs_for_width(outputs, terminal_width);
    let rows = flatten_rows(&filtered_outputs);
    if rows.is_empty() {
        return String::new();
    }

    let key_width = max_key_width(&rows);

    if is_minimal {
        if has_column_space(terminal_width, key_width, separator_width) {
            return render_minimal(rows, runtime, colors_enabled, terminal_width, key_width);
        }

        // The key and a value do not fit next to each other, so the rows are
        // written one under the other.
        return render_dense(rows, runtime, colors_enabled, terminal_width);
    }

    let key_suffix_width = usize::from(!runtime.is_plain());
    if has_column_space(terminal_width, key_width, separator_width) {
        return render_column(
            rows,
            runtime,
            colors_enabled,
            key_width,
            column_value_width(terminal_width, key_width, key_suffix_width, separator_width),
        );
    }

    render_dense(rows, runtime, colors_enabled, terminal_width)
}

pub(super) fn render_split(
    rows: Vec<RenderRow>,
    raw_logo: Vec<String>,
    runtime: &RuntimeConfig,
    colors_enabled: bool,
    logo_definition: logo::LogoDefinition,
    key_width: usize,
    value_width: Option<usize>,
) -> String {
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

    output
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
        let rule = "─".repeat(title_width);
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
        let value = truncate_row_value(&row.value, options.value_width);
        let styled_value = style_value(
            &value,
            row.status,
            runtime.appearance.palette,
            colors_enabled,
        );
        lines.push(format!(
            "{}{}{}",
            styled_key, runtime.appearance.separator, styled_value
        ));
    }

    if options.include_header && runtime.appearance.color_bars && colors_enabled {
        lines.push(String::new());
        lines.push(color_bar(false));
        lines.push(color_bar(true));
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
        let value_width = terminal_width.and_then(|width| {
            width.checked_sub(key_width + UnicodeWidthStr::width(DENSE_SEPARATOR))
        });
        let value = truncate_row_value(&row.value, value_width);
        let styled_key = style_key(&key, runtime.appearance.palette, colors_enabled);
        let styled_value = style_value(
            &value,
            row.status,
            runtime.appearance.palette,
            colors_enabled,
        );

        if value.is_empty() {
            output.push_str(&format!("{styled_key}\n"));
        } else {
            output.push_str(&format!("{styled_key}{DENSE_SEPARATOR}{styled_value}\n"));
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
        for OutputRow { key, value } in &output.rows {
            rows.push(RenderRow {
                key: key.clone(),
                value: value.clone(),
                status: output.status,
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
        })
        .collect()
}

fn render_key(key: &str, options: InfoRenderOptions) -> String {
    if options.include_header {
        format!("{key}:")
    } else {
        format!("{:>width$}", key, width = options.key_width)
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

fn max_key_width(rows: &[RenderRow]) -> usize {
    rows.iter()
        .map(|row| UnicodeWidthStr::width(row.key.as_str()))
        .max()
        .unwrap_or(0)
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
            modules: vec![ModuleConfig::from(ModuleId::Os)],
            plugins: Vec::new(),
            json: false,
            diagnostics: false,
            animated: false,
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

        let rendered = render_text_with_width(&outputs, &runtime, None);
        assert_eq!(rendered, "OS  Arch Linux\n");
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
            modules: vec![ModuleConfig::from(ModuleId::Os)],
            plugins: Vec::new(),
            json: false,
            diagnostics: false,
            animated: false,
            watch: false,
            watch_interval: 1,
        };
        let outputs = vec![ModuleOutput::ok(
            ModuleId::Os,
            vec![OutputRow::new("OS", "Arch Linux")],
            json!({"name": "Arch Linux"}),
        )];

        let rendered = render_text_with_width(&outputs, &runtime, Some(40));
        assert!(rendered.ends_with("OS:  Arch Linux\n"));
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
            modules: vec![ModuleConfig::from(ModuleId::Theme)],
            plugins: Vec::new(),
            json: false,
            diagnostics: false,
            animated: false,
            watch: false,
            watch_interval: 1,
        };
        let outputs = vec![ModuleOutput::ok(
            ModuleId::Theme,
            vec![OutputRow::new("Theme", "gtk:Adwaita icons:breeze-dark")],
            json!({}),
        )];

        let rendered = render_text_with_width(&outputs, &runtime, Some(18));
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
            modules: vec![ModuleConfig::from(ModuleId::Theme)],
            plugins: Vec::new(),
            json: false,
            diagnostics: false,
            animated: false,
            watch: false,
            watch_interval: 1,
        };
        let outputs = vec![ModuleOutput::ok(
            ModuleId::Theme,
            vec![OutputRow::new("Theme", "gtk:Adwaita icons:breeze-dark")],
            json!({}),
        )];

        let rendered = render_text_with_width(&outputs, &runtime, Some(20));
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
            modules: vec![ModuleConfig::from(ModuleId::Os)],
            plugins: Vec::new(),
            json: false,
            diagnostics: false,
            animated: false,
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
            let rendered = render_text_with_width(&outputs, &runtime, Some(width));
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
            modules: vec![ModuleConfig::from(ModuleId::Os)],
            plugins: Vec::new(),
            json: false,
            diagnostics: false,
            animated: false,
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
            let rendered = render_text_with_width(&outputs, &runtime, Some(width));
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
            modules: vec![ModuleConfig::from(ModuleId::Os)],
            plugins: Vec::new(),
            json: false,
            diagnostics: false,
            animated: false,
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
        let rendered = render_text_with_width(&outputs, &runtime, Some(width));
        assert!(
            rendered.contains(compact.first().expect("the compact logo has a line")),
            "the logo is missing:\n{rendered}"
        );
        assert!(
            !rendered.contains(full.first().expect("the logo has a line")),
            "the full drawing is still used:\n{rendered}"
        );
    }
}
