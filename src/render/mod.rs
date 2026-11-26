//! Rendering the collected modules.
//!
//! The layout decides how the rows are arranged, the style decides how
//! they are painted and mod.rs ties both together.

use std::env;
use std::fs;
use std::io::{self, IsTerminal};
use std::path::Path;
use unicode_width::UnicodeWidthStr;

use anyhow::Result;

use crate::config::RuntimeConfig;
use crate::logo_image;
use crate::model::{ModuleId, ModuleOutput, ModuleStatus};

mod doctor;
mod layout;
mod style;

use layout::render_text_with_width;

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
    meter: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
struct InfoRenderOptions {
    key_width: usize,
    /// Prefix each label with a Nerd Font icon (`--icons`).
    icons: bool,
    value_width: Option<usize>,
    max_line_width: Option<usize>,
    include_header: bool,
}

/// The rendered output plus, when a logo sits left of the rows, the number of
/// display columns it occupies on every line. The animation uses that prefix
/// width to reveal the logo skeleton before the session rows.
pub fn render(
    outputs: &[ModuleOutput],
    runtime: &RuntimeConfig,
) -> Result<(String, Option<usize>)> {
    if runtime.doctor {
        return Ok((doctor::render_doctor(outputs), None));
    }

    if runtime.diagnostics {
        return Ok((render_diagnostics(outputs), None));
    }

    if runtime.json {
        return render_json(outputs).map(|rendered| (rendered, None));
    }

    // An image logo replaces the whole text layout: the picture is drawn at the
    // top-left and the rows are positioned beside it with absolute cursor moves.
    // Only in a real terminal, and only where the protocol is supported.
    if !runtime.is_plain()
        && io::stdout().is_terminal()
        && let Some(path) = runtime.logo_image.as_deref()
        && let Some(rendered) = render_image_logo(outputs, runtime, Path::new(path))
    {
        return Ok((rendered, None));
    }

    Ok(render_text(outputs, runtime))
}

/// Draw a PNG as the logo and place the rows to its right.
///
/// Returns `None` whenever the terminal cannot do it or the file is unusable,
/// so the caller falls back to the text layout rather than printing raw escape
/// codes into the output.
/// Where the picture starts, given how tall it is and how tall the rows are.
///
/// Centred when there is real slack, top-aligned when the two are close: a
/// one- or two-row difference reads as a mistake, not as a composition.
fn logo_start_row(rows: u32, target_rows: u32) -> u32 {
    if rows + 2 < target_rows {
        (target_rows - rows) / 2 + 1
    } else {
        1
    }
}

fn render_image_logo(
    outputs: &[ModuleOutput],
    runtime: &RuntimeConfig,
    path: &Path,
) -> Option<String> {
    if !logo_image::supports_graphics() {
        return None;
    }
    let png = logo_image::load_png(path)?;
    let dimensions = logo_image::png_dimensions(&png)?;

    let terminal_cols = detect_terminal_width().unwrap_or(120);
    let lines = layout::render_rows_only(outputs, runtime, detect_terminal_width());
    if lines.is_empty() {
        return None;
    }

    // A zero `logo_image_cols` means "size it to the text block", but capped at
    // a third of the terminal: a square picture stretched to fill 57 rows would
    // be 114 columns wide and eat half the screen, which reads as a wallpaper
    // rather than a logo. A third keeps the picture beside the rows instead of
    // in front of them, and the shortfall shows as a shorter logo, the way
    // fastfetch composes it.
    let max_cols = if runtime.logo_image_cols == 0 {
        (terminal_cols / 3).max(10) as u32
    } else {
        runtime.logo_image_cols
    };
    let target_rows = lines.len().max(1) as u32;
    let cols = logo_image::fit_cols(target_rows, dimensions, max_cols);
    let rows = logo_image::fit_rows(cols, dimensions, target_rows);

    // Scale the pixels to the box: the protocol's c/r only moves the cursor.
    let scaled = logo_image::scale_to_box(&png, cols, rows, logo_image::cell_pixels());

    // A picture that cannot fill the block (a wide image capped by the width
    // ceiling) is centred against the rows instead of hanging from the top:
    // the eye reads the pair as one composition, not as a logo with a hole
    // underneath it. One row of slack is left at the top so nothing looks glued
    // to the first line.
    let start_row = logo_start_row(rows, target_rows);

    let text_col = cols + 2;
    let mut rendered = String::from("\x1b[H");
    if start_row > 1 {
        rendered.push_str(&format!("\x1b[{start_row};1H"));
    }
    rendered.push_str(&logo_image::apc(&scaled, cols, rows));
    for (index, line) in lines.iter().enumerate() {
        rendered.push_str(&format!("\x1b[{};{}H{}", index + 1, text_col, line));
    }
    Some(rendered)
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

pub fn render_text(outputs: &[ModuleOutput], runtime: &RuntimeConfig) -> (String, Option<usize>) {
    render_text_with_width(outputs, runtime, detect_terminal_width())
}

fn output_priority(id: &str) -> usize {
    let Ok(id) = id.parse::<ModuleId>() else {
        return 1;
    };

    match id {
        ModuleId::Os
        | ModuleId::Host
        | ModuleId::Bios
        | ModuleId::Board
        | ModuleId::Chassis
        | ModuleId::Kernel
        | ModuleId::InitSystem
        | ModuleId::Uptime
        | ModuleId::Packages
        | ModuleId::Memory
        | ModuleId::Hyprland
        | ModuleId::HyprHealth
        | ModuleId::HyprInstances
        | ModuleId::HyprActiveWindow
        | ModuleId::HyprCursor
        | ModuleId::HyprWorkspaceHistory
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
        | ModuleId::Media
        | ModuleId::Datetime
        | ModuleId::Colorscheme
        | ModuleId::LoadAverage
        | ModuleId::Processes
        | ModuleId::Users
        | ModuleId::LocalIp
        | ModuleId::PublicIp
        | ModuleId::Weather
        | ModuleId::Display
        | ModuleId::Editor
        | ModuleId::Brightness
        | ModuleId::PowerAdapter
        | ModuleId::Bluetooth
        | ModuleId::DiskIo
        | ModuleId::NetIo
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

/// The number of rows the terminal shows, when it can be asked.
///
/// The animated reveal steps the cursor back up over the printed output, so
/// it needs to know the output fits on screen; a query that fails leaves the
/// caller on the safe single-pass reveal.
pub fn detect_terminal_rows() -> Option<usize> {
    query_terminal_rows().or_else(|| {
        env::var("LINES")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|rows| *rows > 0)
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

#[cfg(unix)]
fn query_terminal_rows() -> Option<usize> {
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
    (result == 0 && size.ws_row > 0).then_some(size.ws_row as usize)
}

#[cfg(not(unix))]
fn query_terminal_width() -> Option<usize> {
    None
}

#[cfg(not(unix))]
fn query_terminal_rows() -> Option<usize> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::model::{ModuleId, ModuleOutput, OutputRow};
    use serde_json::json;

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

    #[test]
    fn a_short_picture_is_centred_against_the_rows() {
        // 20 rows of picture inside 50 rows of text: 15 rows of slack, so it
        // starts at row 16.
        assert_eq!(logo_start_row(20, 50), 16);
        assert_eq!(logo_start_row(30, 54), 13);
    }

    #[test]
    fn a_close_fit_stays_at_the_top() {
        assert_eq!(logo_start_row(50, 50), 1);
        assert_eq!(logo_start_row(48, 50), 1);
        assert_eq!(logo_start_row(49, 50), 1);
    }

    #[test]
    fn a_taller_picture_never_shifts_down() {
        assert_eq!(logo_start_row(60, 50), 1);
    }
}
