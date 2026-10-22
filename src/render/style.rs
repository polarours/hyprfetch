use std::env;
use std::io::{self, IsTerminal};

use crate::logo::{self, LogoTone};
use crate::model::{ColorMode, ModuleStatus, PaletteName};
use owo_colors::OwoColorize;

pub(super) fn colors_enabled(mode: ColorMode) -> bool {
    match mode {
        ColorMode::Always => true,
        ColorMode::Never => false,
        ColorMode::Auto => io::stdout().is_terminal() && env::var_os("NO_COLOR").is_none(),
    }
}

pub(super) fn style_key(value: &str, _palette: PaletteName, colors_enabled: bool) -> String {
    if !colors_enabled {
        return value.to_string();
    }

    // 使用 Tokyo Night 风格的柔和色调
    let color = match value {
        "OS" | "Host" | "Kernel" => "\x1b[38;5;110m", // #7aa2f7 柔蓝
        "Uptime" | "Shell" | "Terminal" => "\x1b[38;5;180m", // #e0af68 柔金
        "CPU" | "GPU" | "Memory" | "Disk" => "\x1b[38;5;114m", // #9ece6a 柔绿
        "Battery" | "Locale" => "\x1b[38;5;141m",     // #bb9af7 柔紫
        "Network" | "Temperature" | "Audio" => "\x1b[38;5;111m", // #7dcfff 柔青
        "Hyprland" | "Workspace" | "Clients" | "Focused" => "\x1b[38;5;140m", // #c0a7d6 柔薰衣草
        "Session" | "Window" | "Workspaces" | "Monitors" => "\x1b[38;5;181m", // #ff9e64 柔橘
        "Theme" | "Ecosystem" | "Input" | "Colors" => "\x1b[38;5;175m", // #f7768e 柔粉
        _ => "\x1b[38;5;243m",                        // #565f89 柔灰
    };

    format!("{}{}\x1b[0m", color, value)
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

    // 使用 Tokyo Night 风格的柔和色调
    match (palette, status) {
        (_, ModuleStatus::Unavailable) => format!("\x1b[38;5;243m{}\x1b[0m", value), // 柔灰
        (_, ModuleStatus::Partial) => format!("\x1b[38;5;215m{}\x1b[0m", value),     // 柔橙
        (PaletteName::Hypr, ModuleStatus::Ok) => format!("\x1b[38;5;252m{}\x1b[0m", value), // 亮白灰
        (PaletteName::Mono, ModuleStatus::Ok) => value.to_string(),
        (PaletteName::Catppuccin, ModuleStatus::Ok) => format!("\x1b[38;5;109m{}\x1b[0m", value), // 柔青
        (PaletteName::Dracula, ModuleStatus::Ok) => format!("\x1b[38;5;114m{}\x1b[0m", value), // 柔绿
    }
}

pub(super) fn style_title(value: &str, palette: PaletteName, colors_enabled: bool) -> String {
    if !colors_enabled {
        return value.to_string();
    }

    match palette {
        PaletteName::Hypr => format!("\x1b[38;5;255m{}\x1b[0m", value),
        PaletteName::Mono => value.bold().to_string(),
        PaletteName::Catppuccin => format!("\x1b[38;5;255m{}\x1b[0m", value),
        PaletteName::Dracula => format!("\x1b[38;5;255m{}\x1b[0m", value),
    }
}

pub(super) fn style_rule(value: &str, palette: PaletteName, colors_enabled: bool) -> String {
    if !colors_enabled {
        return value.to_string();
    }

    match palette {
        PaletteName::Hypr => format!("\x1b[38;5;240m{}\x1b[0m", value),
        PaletteName::Mono => value.dimmed().to_string(),
        PaletteName::Catppuccin => format!("\x1b[38;5;240m{}\x1b[0m", value),
        PaletteName::Dracula => format!("\x1b[38;5;240m{}\x1b[0m", value),
    }
}

pub(super) fn color_bar(bright: bool) -> String {
    // Tokyo Night 风格的颜色条
    let cells = if bright {
        [
            "███".truecolor(122, 162, 247).to_string(), // #7aa2f7
            "███".truecolor(187, 154, 247).to_string(), // #bb9af7
            "███".truecolor(125, 207, 255).to_string(), // #7dcfff
            "███".truecolor(158, 206, 106).to_string(), // #9ece6a
            "███".truecolor(224, 175, 104).to_string(), // #e0af68
            "███".truecolor(255, 158, 100).to_string(), // #ff9e64
            "███".truecolor(247, 118, 142).to_string(), // #f7768e
            "███".truecolor(192, 167, 214).to_string(), // #c0a7d6
        ]
    } else {
        [
            "███".truecolor(86, 95, 137).to_string(),   // #565f89
            "███".truecolor(122, 162, 247).to_string(), // #7aa2f7
            "███".truecolor(187, 154, 247).to_string(), // #bb9af7
            "███".truecolor(125, 207, 255).to_string(), // #7dcfff
            "███".truecolor(158, 206, 106).to_string(), // #9ece6a
            "███".truecolor(224, 175, 104).to_string(), // #e0af68
            "███".truecolor(255, 158, 100).to_string(), // #ff9e64
            "███".truecolor(247, 118, 142).to_string(), // #f7768e
        ]
    };

    format!(" {}", cells.join(" "))
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
