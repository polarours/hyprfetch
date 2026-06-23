use std::env;
use std::fs;
use std::io::{self, IsTerminal};

use anyhow::Result;
use owo_colors::OwoColorize;
use unicode_width::UnicodeWidthStr;

use crate::config::RuntimeConfig;
use crate::logo::{self, LogoTone};
use crate::model::{
    ColorMode, ModuleId, ModuleOutput, ModuleStatus, OutputRow, PaletteName, RenderStyle,
};
use crate::util::truncate_visible;

const MIN_SPLIT_VALUE_WIDTH: usize = 24;
const MIN_COLUMN_VALUE_WIDTH: usize = 20;
const DENSE_PRIORITY_WIDTH: usize = 24;
const DENSE_SEPARATOR: &str = " ";
const DEFAULT_HEADER_WIDTH: usize = 40;
const MAX_HEADER_WIDTH: usize = 44;
const MIN_HEADER_WIDTH: usize = 8;

#[derive(Debug, Clone)]
struct RenderRow {
    key: String,
    value: String,
    status: ModuleStatus,
    module_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RenderSection {
    System,
    Hyprland,
    Rice,
    Custom,
}

#[derive(Debug, Clone, Copy)]
struct InfoRenderOptions {
    key_width: usize,
    value_width: Option<usize>,
    max_line_width: Option<usize>,
    include_header: bool,
    color_bars_min_line: usize,
}

pub fn render(outputs: &[ModuleOutput], runtime: &RuntimeConfig) -> Result<String> {
    if runtime.diagnostics {
        return Ok(render_diagnostics(outputs));
    }

    if runtime.json {
        return render_json(outputs);
    }

    Ok(render_text(outputs, runtime))
}

pub fn render_json(outputs: &[ModuleOutput]) -> Result<String> {
    let rendered = serde_json::to_string_pretty(outputs)?;
    Ok(format!("{rendered}\n"))
}

pub fn render_diagnostics(outputs: &[ModuleOutput]) -> String {
    let mut output = String::new();
    output.push_str("hyprfetch diagnostics\n");

    let ok_count = outputs
        .iter()
        .filter(|output| output.status == ModuleStatus::Ok)
        .count();
    let partial_count = outputs
        .iter()
        .filter(|output| output.status == ModuleStatus::Partial)
        .count();
    let unavailable_count = outputs
        .iter()
        .filter(|output| output.status == ModuleStatus::Unavailable)
        .count();

    output.push_str(&format!(
        "summary: {ok_count} ok, {partial_count} partial, {unavailable_count} unavailable\n"
    ));

    for output_item in outputs {
        let status = match output_item.status {
            ModuleStatus::Ok => "ok",
            ModuleStatus::Partial => "partial",
            ModuleStatus::Unavailable => "unavailable",
        };
        output.push_str(&format!("{}: {status}", output_item.id));
        if let Some(message) = output_item.message.as_deref() {
            output.push_str(&format!(" - {message}"));
        }
        output.push('\n');
    }

    output
}

pub fn render_text(outputs: &[ModuleOutput], runtime: &RuntimeConfig) -> String {
    render_text_with_width(outputs, runtime, detect_terminal_width())
}

fn render_text_with_width(
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
            let logo_width = raw_logo
                .iter()
                .map(|line| UnicodeWidthStr::width(line.as_str()))
                .max()
                .unwrap_or(0);
            let split_padding = runtime.appearance.padding + 1;
            let split_base_width = logo_width + split_padding + key_width + separator_width;

            if has_split_space(terminal_width, split_base_width) {
                return render_split(
                    rows,
                    raw_logo,
                    runtime,
                    colors_enabled,
                    logo_definition,
                    key_width,
                    split_value_width(terminal_width, split_base_width),
                );
            }
        }
    }

    let filtered_outputs = prioritize_outputs_for_width(outputs, terminal_width);
    let rows = flatten_rows(&filtered_outputs);
    if rows.is_empty() {
        return String::new();
    }

    if is_minimal {
        return render_minimal(rows, runtime, colors_enabled);
    }

    let key_width = max_key_width(&rows);
    if has_column_space(terminal_width, key_width, separator_width) {
        return render_column(
            rows,
            runtime,
            colors_enabled,
            key_width,
            column_value_width(terminal_width, key_width, separator_width),
        );
    }

    render_dense(rows, runtime, colors_enabled, terminal_width)
}

fn render_split(
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
            max_line_width: right_width,
            include_header: true,
            color_bars_min_line: raw_logo.len(),
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
            max_line_width: right_width,
            include_header: !runtime.is_plain(),
            color_bars_min_line: 0,
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
    let mut last_section = None;

    if options.include_header
        && let Some(title) = header_title()
    {
        let title_width = header_width(&title, options.max_line_width);
        let title = truncate_visible(&title, title_width);
        let rule = "─".repeat(title_width);
        lines.push(style_title(
            &title,
            runtime.appearance.palette,
            colors_enabled,
        ));
        lines.push(style_rule(
            &rule,
            runtime.appearance.palette,
            colors_enabled,
        ));
    }

    for row in rows {
        if options.include_header {
            let section = row_section(&row.module_id);
            if last_section.is_some_and(|previous| previous != section) {
                lines.push(String::new());
            }
            last_section = Some(section);
        }

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
        while lines.len() + 3 < options.color_bars_min_line {
            lines.push(String::new());
        }
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
) -> String {
    let key_width = max_key_width(&rows);
    render_info_lines(
        rows,
        runtime,
        colors_enabled,
        InfoRenderOptions {
            key_width,
            value_width: None,
            max_line_width: None,
            include_header: false,
            color_bars_min_line: 0,
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
                module_id: output.id.clone(),
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
            module_id: output.id.clone(),
        })
        .collect()
}

fn row_section(id: &str) -> RenderSection {
    let Ok(id) = id.parse::<ModuleId>() else {
        return RenderSection::Custom;
    };

    match id {
        ModuleId::Os
        | ModuleId::Host
        | ModuleId::Kernel
        | ModuleId::Uptime
        | ModuleId::Packages
        | ModuleId::Shell
        | ModuleId::Terminal
        | ModuleId::Cpu
        | ModuleId::Gpu
        | ModuleId::Memory
        | ModuleId::Swap
        | ModuleId::Disk
        |         ModuleId::Battery
        | ModuleId::Locale
        | ModuleId::Network
        | ModuleId::Temperature
        | ModuleId::Audio
        | ModuleId::Datetime => RenderSection::System,
        ModuleId::Hyprland
        | ModuleId::HyprHealth
        | ModuleId::HyprInstances
        | ModuleId::HyprSplash
        | ModuleId::HyprActiveWindow
        | ModuleId::HyprCursor
        | ModuleId::HyprWorkspaces
        | ModuleId::HyprMonitors
        | ModuleId::HyprRice
        | ModuleId::HyprDevices
        | ModuleId::HyprBinds
        | ModuleId::HyprShortcuts
        | ModuleId::HyprLayouts
        | ModuleId::HyprLayers
        | ModuleId::HyprConfig
        | ModuleId::HyprRules => RenderSection::Hyprland,
        ModuleId::Theme | ModuleId::Wallpaper | ModuleId::HyprEcosystem => RenderSection::Rice,
    }
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

fn output_priority(id: &str) -> usize {
    let Ok(id) = id.parse::<ModuleId>() else {
        return 1;
    };

    match id {
        ModuleId::Os
        | ModuleId::Host
        | ModuleId::Kernel
        | ModuleId::Uptime
        | ModuleId::Packages
        | ModuleId::Memory
        | ModuleId::Hyprland
        | ModuleId::HyprHealth
        | ModuleId::HyprInstances
        | ModuleId::HyprActiveWindow
        | ModuleId::HyprCursor
        | ModuleId::HyprWorkspaces
        | ModuleId::HyprMonitors => 0,
        ModuleId::Shell
        | ModuleId::Terminal
        | ModuleId::Cpu
        | ModuleId::Theme
        | ModuleId::HyprRice
        | ModuleId::HyprConfig
        | ModuleId::HyprRules => 1,
        ModuleId::Gpu
        | ModuleId::Swap
        | ModuleId::Disk
        | ModuleId::Battery
        | ModuleId::Locale
        | ModuleId::Network
        | ModuleId::Temperature
        | ModuleId::Audio
        | ModuleId::Datetime
        | ModuleId::Wallpaper
        | ModuleId::HyprEcosystem
        | ModuleId::HyprSplash
        | ModuleId::HyprDevices
        | ModuleId::HyprBinds
        | ModuleId::HyprShortcuts
        | ModuleId::HyprLayouts
        | ModuleId::HyprLayers => 2,
    }
}

fn output_label(id: &str) -> String {
    id.parse::<ModuleId>()
        .map(|module| module.label().to_string())
        .unwrap_or_else(|_| id.to_string())
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

fn split_value_width(terminal_width: Option<usize>, split_base_width: usize) -> Option<usize> {
    terminal_width.and_then(|width| width.checked_sub(split_base_width))
}

fn column_value_width(
    terminal_width: Option<usize>,
    key_width: usize,
    separator_width: usize,
) -> Option<usize> {
    terminal_width.and_then(|width| width.checked_sub(key_width + separator_width))
}

fn truncate_row_value(value: &str, value_width: Option<usize>) -> String {
    match value_width {
        Some(width) => truncate_visible(value, width),
        None => value.to_string(),
    }
}

fn header_title() -> Option<String> {
    let user = env::var("USER")
        .or_else(|_| env::var("USERNAME"))
        .ok()
        .filter(|value| !value.trim().is_empty());
    let host = fs::read_to_string("/proc/sys/kernel/hostname")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            env::var("HOSTNAME")
                .ok()
                .filter(|value| !value.trim().is_empty())
        });

    match (user, host) {
        (Some(user), Some(host)) => Some(format!("{user}@{host}")),
        (Some(user), None) => Some(user),
        (None, Some(host)) => Some(host),
        (None, None) => None,
    }
}

fn header_width(title: &str, max_line_width: Option<usize>) -> usize {
    let max_width = max_line_width
        .unwrap_or(DEFAULT_HEADER_WIDTH)
        .clamp(MIN_HEADER_WIDTH, MAX_HEADER_WIDTH);
    UnicodeWidthStr::width(title).clamp(MIN_HEADER_WIDTH, max_width)
}

fn detect_terminal_width() -> Option<usize> {
    query_terminal_width().or_else(|| {
        env::var("COLUMNS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|width| *width > 0)
    })
}

#[cfg(unix)]
fn query_terminal_width() -> Option<usize> {
    use libc::{STDOUT_FILENO, TIOCGWINSZ, ioctl, winsize};

    if !io::stdout().is_terminal() {
        return None;
    }

    let mut size = winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    let result = unsafe { ioctl(STDOUT_FILENO, TIOCGWINSZ, &mut size) };
    (result == 0 && size.ws_col > 0).then_some(size.ws_col as usize)
}

#[cfg(not(unix))]
fn query_terminal_width() -> Option<usize> {
    None
}

fn colors_enabled(mode: ColorMode) -> bool {
    match mode {
        ColorMode::Always => true,
        ColorMode::Never => false,
        ColorMode::Auto => io::stdout().is_terminal() && env::var_os("NO_COLOR").is_none(),
    }
}

fn style_key(value: &str, palette: PaletteName, colors_enabled: bool) -> String {
    if !colors_enabled {
        return value.to_string();
    }

    match palette {
        PaletteName::Hypr => value.bold().bright_cyan().to_string(),
        PaletteName::Mono => value.bold().to_string(),
        PaletteName::Catppuccin => value.bold().bright_magenta().to_string(),
        PaletteName::Dracula => value.bold().bright_magenta().to_string(),
    }
}

fn style_value(
    value: &str,
    status: ModuleStatus,
    palette: PaletteName,
    colors_enabled: bool,
) -> String {
    if !colors_enabled {
        return value.to_string();
    }

    match (palette, status) {
        (_, ModuleStatus::Unavailable) => value.dimmed().to_string(),
        (_, ModuleStatus::Partial) => value.yellow().to_string(),
        (PaletteName::Hypr, ModuleStatus::Ok) => value.bright_white().to_string(),
        (PaletteName::Mono, ModuleStatus::Ok) => value.to_string(),
        (PaletteName::Catppuccin, ModuleStatus::Ok) => value.bright_green().to_string(),
        (PaletteName::Dracula, ModuleStatus::Ok) => value.bright_green().to_string(),
    }
}

fn style_title(value: &str, palette: PaletteName, colors_enabled: bool) -> String {
    if !colors_enabled {
        return value.to_string();
    }

    match palette {
        PaletteName::Hypr => value.bold().bright_white().to_string(),
        PaletteName::Mono => value.bold().to_string(),
        PaletteName::Catppuccin => value.bold().bright_white().to_string(),
        PaletteName::Dracula => value.bold().bright_white().to_string(),
    }
}

fn style_rule(value: &str, palette: PaletteName, colors_enabled: bool) -> String {
    if !colors_enabled {
        return value.to_string();
    }

    match palette {
        PaletteName::Hypr => value.bright_cyan().to_string(),
        PaletteName::Mono => value.dimmed().to_string(),
        PaletteName::Catppuccin => value.magenta().to_string(),
        PaletteName::Dracula => value.bright_red().to_string(),
    }
}

fn color_bar(bright: bool) -> String {
    let cells = if bright {
        [
            "███".bright_black().to_string(),
            "███".bright_red().to_string(),
            "███".bright_green().to_string(),
            "███".bright_yellow().to_string(),
            "███".bright_blue().to_string(),
            "███".bright_magenta().to_string(),
            "███".bright_cyan().to_string(),
            "███".bright_white().to_string(),
        ]
    } else {
        [
            "███".black().to_string(),
            "███".red().to_string(),
            "███".green().to_string(),
            "███".yellow().to_string(),
            "███".blue().to_string(),
            "███".magenta().to_string(),
            "███".cyan().to_string(),
            "███".white().to_string(),
        ]
    };

    cells.join(" ")
}

fn style_logo(
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

    let tone = logo::tone_for_line(definition, palette, line_index, line_count);
    match palette {
        PaletteName::Hypr | PaletteName::Catppuccin | PaletteName::Dracula => {
            style_tone(value, tone)
        }
        PaletteName::Mono => value.bold().to_string(),
    }
}

fn style_tone(value: &str, tone: LogoTone) -> String {
    match tone {
        LogoTone::Blue => value.bold().bright_blue().to_string(),
        LogoTone::Cyan => value.bold().bright_cyan().to_string(),
        LogoTone::Green => value.bold().bright_green().to_string(),
        LogoTone::Indigo => value.bold().bright_magenta().to_string(),
        LogoTone::Magenta => value.bold().magenta().to_string(),
        LogoTone::Red => value.bold().bright_red().to_string(),
        LogoTone::White => value.bold().bright_white().to_string(),
        LogoTone::Yellow => value.bold().bright_yellow().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        output_priority, prioritize_outputs_for_width, render_diagnostics, render_text_with_width,
    };
    use crate::config::{AppearanceConfig, ModuleConfig, RuntimeConfig};
    use crate::model::{
        ColorMode, LogoPreset, ModuleId, ModuleOutput, ModuleStatus, OutputRow, PaletteName,
        RenderStyle,
    };

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
        };
        let outputs = vec![ModuleOutput::ok(
            ModuleId::Theme,
            vec![OutputRow::new("Theme", "gtk:Adwaita icons:breeze-dark")],
            json!({}),
        )];

        let rendered = render_text_with_width(&outputs, &runtime, Some(20));
        assert_eq!(rendered, "Theme gtk:Adwaita i…\n");
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

    #[test]
    fn assigns_lower_priority_to_auxiliary_modules() {
        assert!(output_priority("hyprland") < output_priority("wallpaper"));
        assert!(output_priority("theme") < output_priority("hypr-devices"));
    }

    #[test]
    fn renders_diagnostics_with_module_statuses() {
        let outputs = vec![
            ModuleOutput::ok(
                ModuleId::Os,
                vec![OutputRow::new("OS", "Arch Linux")],
                json!({}),
            ),
            ModuleOutput::partial(
                ModuleId::Hyprland,
                vec![OutputRow::new("Hyprland", "0.48.1")],
                json!({}),
                "clients unavailable",
            ),
            ModuleOutput::unavailable(ModuleId::Wallpaper, "no wallpaper backend detected"),
        ];

        let rendered = render_diagnostics(&outputs);
        assert!(rendered.contains("summary: 1 ok, 1 partial, 1 unavailable"));
        assert!(rendered.contains("os: ok"));
        assert!(rendered.contains("hyprland: partial - clients unavailable"));
        assert!(rendered.contains("wallpaper: unavailable - no wallpaper backend detected"));
    }
}
