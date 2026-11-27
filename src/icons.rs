//! Nerd Font glyphs for `--icons`.
//!
//! The mapping is keyed by the rendered row label (`OS`, `CPU`, `Workspace`…)
//! and returns a private-use-area icon. Icons are opt-in because a terminal
//! without a Nerd Font renders them as tofu; the default layout keeps the
//! plain `[LABEL]` text.

/// The icon for a row key, if one is mapped.
///
/// Only widely-covered Font Awesome codepoints are used so the glyphs exist in
/// every Nerd Font build (v2 and v3).
pub fn icon_for(key: &str) -> Option<char> {
    // Rows such as `Disk (/)` or `Battery (L23M4PK5)` carry a detail in
    // parentheses; match on the stable prefix.
    let base = key.split('(').next().unwrap_or(key).trim();
    let glyph = match base {
        // system
        "OS" => '\u{f17c}',          // fa-linux
        "Host" => '\u{f108}',        // fa-desktop
        "Kernel" => '\u{f121}',      // fa-code (kernel build)
        "Uptime" => '\u{f017}',      // fa-clock-o
        "Packages" => '\u{f1b2}',    // fa-cube
        "Shell" => '\u{f120}',       // fa-terminal
        "Terminal" => '\u{f120}',    // fa-terminal
        "CPU" => '\u{f2db}',         // fa-microchip
        "GPU" => '\u{f085}',         // fa-cogs
        "Memory" => '\u{f538}',      // fa-memory
        "Swap" => '\u{f0a0}',        // fa-hdd-o
        "Disk" => '\u{f0a0}',        // fa-hdd-o
        "Battery" => '\u{f240}',     // fa-battery-full
        "Locale" => '\u{f0ac}',      // fa-globe
        "Network" => '\u{f1eb}',     // fa-wifi
        "Temperature" => '\u{f2c9}', // fa-thermometer-half
        "Audio" => '\u{f028}',       // fa-volume-up
        "Media" => '\u{f001}',       // fa-music
        "Datetime" => '\u{f073}',    // fa-calendar
        "Load" => '\u{f0e4}',        // fa-tachometer
        "Processes" => '\u{f0c9}',   // fa-list-ul
        "Users" => '\u{f007}',       // fa-user
        "Local IP" => '\u{f225}',    // fa-server
        "BIOS" => '\u{f2db}',        // fa-microchip
        "Board" => '\u{f0ad}',       // fa-wrench
        "Init" => '\u{f013}',        // fa-cog
        "Editor" => '\u{f044}',      // fa-pencil
        "Brightness" => '\u{f185}',  // fa-sun-o
        "Chassis" => '\u{f109}',     // fa-laptop
        "Power" => '\u{f1e6}',       // fa-plug
        "Bluetooth" => '\u{f293}',   // fa-bluetooth-b
        "Disk I/O" => '\u{f0a0}',    // fa-hdd-o
        "Net I/O" => '\u{f0e8}',     // fa-exchange
        "OpenGL" => '\u{f1b2}',      // fa-cube
        "Weather" => '\u{f0c2}',     // fa-cloud
        "Public IP" => '\u{f0ac}',   // fa-globe
        "Display" => '\u{f108}',     // fa-desktop
        "Colorscheme" => '\u{f53f}', // fa-paint-brush
        // rice / hyprland
        "Hyprland" => '\u{f108}',   // fa-desktop
        "Hyprctl" => '\u{f120}',    // fa-terminal
        "Workspace" => '\u{f009}',  // fa-th-large
        "Workspaces" => '\u{f009}', // fa-th-large
        "Clients" => '\u{f0c0}',    // fa-users
        "Focused" => '\u{f140}',    // fa-bullseye
        "Session" => '\u{f0c0}',    // fa-users
        "Window" => '\u{f2d0}',     // fa-window-maximize
        "Monitors" => '\u{f108}',   // fa-desktop
        "Theme" => '\u{f1fc}',      // fa-paint-brush
        "Wallpaper" => '\u{f03e}',  // fa-image
        "Ecosystem" => '\u{f1b3}',  // fa-cubes
        "Input" => '\u{f11c}',      // fa-keyboard-o
        "Binds" => '\u{f11c}',      // fa-keyboard-o
        _ => return None,
    };
    Some(glyph)
}

#[cfg(test)]
mod tests {
    use super::icon_for;

    #[test]
    fn common_rows_have_icons() {
        for key in [
            "OS",
            "Host",
            "Kernel",
            "Uptime",
            "Packages",
            "Shell",
            "Terminal",
            "CPU",
            "GPU",
            "Memory",
            "Swap",
            "Disk",
            "Battery",
            "Chassis",
            "Power",
            "Bluetooth",
            "Disk I/O",
            "Net I/O",
            "OpenGL",
            "Network",
            "Temperature",
            "Audio",
            "Media",
            "Datetime",
            "Hyprland",
            "Workspace",
            "Window",
            "Monitors",
            "Theme",
            "Wallpaper",
            "Ecosystem",
            "Input",
        ] {
            assert!(icon_for(key).is_some(), "no icon mapped for {key}");
        }
    }

    #[test]
    fn icons_come_from_the_private_use_area() {
        // A real Nerd Font glyph lives in the PUA; an ASCII fallback would mean
        // a silently broken mapping.
        for key in ["OS", "CPU", "Memory", "Workspace"] {
            let icon = icon_for(key).unwrap();
            assert!(
                ('\u{e000}'..='\u{f8ff}').contains(&icon),
                "{key} icon {icon:?} is outside the Nerd Font PUA"
            );
        }
    }

    #[test]
    fn parenthesised_keys_match_their_prefix() {
        assert!(icon_for("Disk (/)").is_some());
        assert!(icon_for("Battery (L23M4PK5)").is_some());
    }

    #[test]
    fn unknown_keys_have_no_icon() {
        assert!(icon_for("Nonsense").is_none());
    }
}
