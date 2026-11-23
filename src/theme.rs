//! Theme catalogue and color roles.
//!
//! Every theme defines eight accent colors plus foreground, background and a
//! muted tone. The renderer maps output roles (keys, values, title, rule,
//! color bar, logo) onto those colors, so adding a theme is a single catalog
//! entry. The catalog also generates `PaletteName`, keeping the selectable
//! names, the config enum and the color definitions in sync.

use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// The colors a theme provides to the renderer.
#[derive(Debug, Clone, Copy)]
pub struct ThemeColors {
    pub name: &'static str,
    pub foreground: &'static str,
    pub background: &'static str,
    pub muted: &'static str,
    /// blue, gold, green, purple, cyan, lavender, orange, pink
    pub accents: [&'static str; 8],
    pub mono: bool,
}

/// ANSI SGR sequence for a 24-bit foreground color given as `#RRGGBB`.
pub fn sgr(hex: &str) -> String {
    let hex = hex.trim_start_matches('#');
    let channel = |from: usize| -> u8 {
        hex.get(from..from + 2)
            .and_then(|pair| u8::from_str_radix(pair, 16).ok())
            .unwrap_or(255)
    };
    format!("\x1b[38;2;{};{};{}m", channel(0), channel(2), channel(4))
}

/// Decode `#RRGGBB` into `(r, g, b)` channels.
fn rgb(hex: &str) -> (f64, f64, f64) {
    let hex = hex.trim_start_matches('#');
    let channel = |from: usize| -> f64 {
        hex.get(from..from + 2)
            .and_then(|pair| u8::from_str_radix(pair, 16).ok())
            .unwrap_or(255) as f64
    };
    (channel(0), channel(2), channel(4))
}

/// The color used for values: a bright, near-white tone that reads like the
/// terminal's default foreground in a screenshot.
///
/// fastfetch draws values in near-white regardless of theme; a theme's plain
/// `foreground` can be a muted tint (the default `hypr` theme is a blue-gray
/// `#c0caf5`), which looks dim next to the bright key colors. Mix the
/// foreground 55% toward white on dark themes, or toward black on light themes
/// (detected from the background's luminance) so light themes stay readable.
/// Deriving it keeps every existing and future theme working without a second
/// hand-maintained color per entry.
pub fn value_color(foreground: &str, background: &str) -> String {
    let (fr, fg, fb) = rgb(foreground);
    let (br, bg, bb) = rgb(background);
    let background_luma = 0.2126 * br + 0.7152 * bg + 0.0722 * bb;
    let light_theme = background_luma > 128.0;
    let target = if light_theme { 0.0 } else { 255.0 };
    let mix = |channel: f64| (channel * 0.45 + target * 0.55).round() as u8;
    format!("#{:02x}{:02x}{:02x}", mix(fr), mix(fg), mix(fb))
}

macro_rules! theme_catalog {
    ($(
        $variant:ident => $name:literal,
        fg: $fg:literal, bg: $bg:literal, muted: $muted:literal,
        accents: [$($accent:literal),+],
        mono: $mono:literal;
    )*) => {
        /// A color theme selectable with `--theme` or `palette` in the config.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
        #[serde(rename_all = "kebab-case")]
        pub enum PaletteName {
            #[default]
            $($variant,)*
        }

        impl PaletteName {
            pub const fn all() -> &'static [Self] {
                &[$(Self::$variant,)*]
            }

            pub fn colors(self) -> ThemeColors {
                match self {
                    $(Self::$variant => ThemeColors {
                        name: $name,
                        foreground: $fg,
                        background: $bg,
                        muted: $muted,
                        accents: [$($accent),+],
                        mono: $mono,
                    },)*
                }
            }
        }

        impl Display for PaletteName {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                let value = match self {
                    $(Self::$variant => $name,)*
                };
                f.write_str(value)
            }
        }

        impl FromStr for PaletteName {
            type Err = String;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                match value {
                    $($name => Ok(Self::$variant),)*
                    _ => Err(format!("invalid theme: {value}")),
                }
            }
        }
    };
}

theme_catalog! {
    // 80s/90s arcade CRT, matched to Cppuccino's arcade theme:
    // Ice-blue coin-op: light blue #8ecbff, electric aqua #7ef5e0 and ice
    // white #e6f0ff on a deep navy #0a1020. Identity keys take the light blue,
    // meters the glowing aqua, and the session block a muted periwinkle /
    // steel blue so it recedes instead of shouting.
    Arcade => "arcade",
        fg: "#e6f0ff", bg: "#0a1020", muted: "#6e8199",
        accents: ["#8ecbff", "#7ef5e0", "#7ef5e0", "#a8d8ff", "#a8d8ff", "#b8b0ff", "#8fa8c8", "#8ecbff"],
        mono: false;
    Hypr => "hypr",
        fg: "#c0caf5", bg: "#1a1b26", muted: "#565f89",
        accents: ["#7aa2f7", "#e0af68", "#9ece6a", "#bb9af7", "#7dcfff", "#c0a7d6", "#ff9e64", "#f7768e"],
        mono: false;
    Mono => "mono",
        fg: "#ffffff", bg: "#000000", muted: "#808080",
        accents: ["#ffffff", "#ffffff", "#ffffff", "#ffffff", "#ffffff", "#ffffff", "#ffffff", "#ffffff"],
        mono: true;
    Catppuccin => "catppuccin",
        fg: "#cdd6f4", bg: "#1e1e2e", muted: "#585b70",
        accents: ["#89b4fa", "#f9e2af", "#a6e3a1", "#cba6f7", "#94e2d5", "#b4befe", "#fab387", "#f38ba8"],
        mono: false;
    Dracula => "dracula",
        fg: "#f8f8f2", bg: "#282a36", muted: "#6272a4",
        accents: ["#8be9fd", "#f1fa8c", "#50fa7b", "#bd93f9", "#8be9fd", "#bd93f9", "#ff5555", "#ff79c6"],
        mono: false;
    Gruvbox => "gruvbox",
        fg: "#ebdbb2", bg: "#282828", muted: "#a89984",
        accents: ["#83a598", "#fabd2f", "#b8bb26", "#d3869b", "#8ec07c", "#d3869b", "#fe8019", "#fb4934"],
        mono: false;
    GruvboxLight => "gruvbox-light",
        fg: "#3c3836", bg: "#fbf1c7", muted: "#7c6f64",
        accents: ["#076678", "#b57614", "#79740e", "#8f3f71", "#427b58", "#8f3f71", "#af3a03", "#9d0006"],
        mono: false;
    Nord => "nord",
        fg: "#d8dee9", bg: "#2e3440", muted: "#4c566a",
        accents: ["#81a1c1", "#ebcb8b", "#a3be8c", "#b48ead", "#88c0d0", "#b48ead", "#d08770", "#bf616a"],
        mono: false;
    OneDark => "one-dark",
        fg: "#abb2bf", bg: "#282c34", muted: "#5c6370",
        accents: ["#61afef", "#e5c07b", "#98c379", "#c678dd", "#56b6c2", "#c678dd", "#d19a66", "#e06c75"],
        mono: false;
    SolarizedDark => "solarized-dark",
        fg: "#839496", bg: "#002b36", muted: "#586e75",
        accents: ["#268bd2", "#b58900", "#859900", "#6c71c4", "#2aa198", "#6c71c4", "#cb4b16", "#d33682"],
        mono: false;
    SolarizedLight => "solarized-light",
        fg: "#657b83", bg: "#fdf6e3", muted: "#93a1a1",
        accents: ["#268bd2", "#b58900", "#859900", "#6c71c4", "#2aa198", "#6c71c4", "#cb4b16", "#d33682"],
        mono: false;
    RosePine => "rose-pine",
        fg: "#e0def4", bg: "#191724", muted: "#6e6a86",
        accents: ["#31748f", "#f6c177", "#9ccfd8", "#c4a7e7", "#9ccfd8", "#c4a7e7", "#f6c177", "#eb6f92"],
        mono: false;
    Ayu => "ayu",
        fg: "#b3b1ad", bg: "#0a0e14", muted: "#5c6773",
        accents: ["#59a5ff", "#e7c547", "#aad94c", "#c792ea", "#77e7e7", "#c792ea", "#ffb454", "#f07178"],
        mono: false;
    Everforest => "everforest",
        fg: "#d3c6aa", bg: "#2d353b", muted: "#7a8478",
        accents: ["#7fbbb3", "#dbbc7f", "#a7c080", "#d699b6", "#83c092", "#d699b6", "#e69875", "#e67e80"],
        mono: false;
    Kanagawa => "kanagawa",
        fg: "#dcd7ba", bg: "#1f1f28", muted: "#727169",
        accents: ["#7e9cd8", "#e0c589", "#98bb6c", "#957fb8", "#7fb4ca", "#938aa9", "#ff9e3b", "#e46876"],
        mono: false;
    Monokai => "monokai",
        fg: "#f8f8f2", bg: "#272822", muted: "#75715e",
        accents: ["#66d9ef", "#f4bf75", "#a6e22e", "#ae81ff", "#66d9ef", "#ae81ff", "#fd971f", "#f92672"],
        mono: false;
    Material => "material",
        fg: "#eeffff", bg: "#212121", muted: "#6c7a7a",
        accents: ["#82aaff", "#ffcb6b", "#c3e88d", "#c792ea", "#89ddff", "#c792ea", "#f78c6c", "#ff5370"],
        mono: false;
    Github => "github",
        fg: "#c9d1d9", bg: "#0d1117", muted: "#8b949e",
        accents: ["#58a6ff", "#d29922", "#3fb950", "#bc8cff", "#39c5cf", "#bc8cff", "#f0883e", "#f85149"],
        mono: false;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_theme_defines_eight_accents() {
        for palette in PaletteName::all() {
            assert_eq!(
                palette.colors().accents.len(),
                8,
                "{} must define eight accents",
                palette
            );
        }
    }

    #[test]
    fn theme_names_round_trip() {
        for palette in PaletteName::all() {
            let parsed = PaletteName::from_str(&palette.to_string())
                .unwrap_or_else(|error| panic!("{} did not parse: {error}", palette));
            assert_eq!(parsed, *palette);
        }
    }

    #[test]
    fn arcade_is_the_default_theme() {
        assert_eq!(PaletteName::default(), PaletteName::Arcade);
    }

    /// The foreground/background of every theme, verified against the
    /// projects' own sources (2026-11-19):
    ///
    /// - hypr matches Tokyo Night Night (`folke/tokyonight.nvim`, fg from
    ///   `colors/storm.lua`, bg from the `colors/night.lua` override);
    /// - catppuccin matches Mocha (`catppuccin/palette`, colors text/base);
    /// - rose-pine matches `rose-pine/palette` (roles text/base);
    /// - dracula matches the official README palette table;
    /// - one-dark matches Atom's HSL definitions (converted exactly);
    /// - solarized-dark matches Ethan Schoonover's X resources (base03/base0);
    /// - nord matches `nordtheme/nord` (nord0/nord4);
    /// - everforest matches the medium-dark palette (`sainnhe/everforest`,
    ///   bg0/fg);
    /// - kanagawa uses the official `sumiInk3` tone, the background every
    ///   terminal port of Kanagawa uses;
    /// - ayu uses the classic official palette every ayu port is built on
    ///   (`ayu-colors` later redesigned its dark theme with a new surface
    ///   system);
    /// - github uses the Dark Classic palette (GitHub's newer default dark
    ///   ships a different, darker tone set);
    /// - material matches the `darker` style of `kaicataldo/material.vim`
    ///   (fg `#eeffff`, bg `#212121`);
    /// - gruvbox, gruvbox-light, solarized-light and monokai match their
    ///   widely published standards.
    #[test]
    fn foregrounds_and_backgrounds_match_their_official_palettes() {
        let ground_truth = [
            (PaletteName::Hypr, "#c0caf5", "#1a1b26"),
            (PaletteName::Catppuccin, "#cdd6f4", "#1e1e2e"),
            (PaletteName::Dracula, "#f8f8f2", "#282a36"),
            (PaletteName::Gruvbox, "#ebdbb2", "#282828"),
            (PaletteName::GruvboxLight, "#3c3836", "#fbf1c7"),
            (PaletteName::Nord, "#d8dee9", "#2e3440"),
            (PaletteName::OneDark, "#abb2bf", "#282c34"),
            (PaletteName::SolarizedDark, "#839496", "#002b36"),
            (PaletteName::SolarizedLight, "#657b83", "#fdf6e3"),
            (PaletteName::RosePine, "#e0def4", "#191724"),
            (PaletteName::Ayu, "#b3b1ad", "#0a0e14"),
            (PaletteName::Everforest, "#d3c6aa", "#2d353b"),
            (PaletteName::Kanagawa, "#dcd7ba", "#1f1f28"),
            (PaletteName::Monokai, "#f8f8f2", "#272822"),
            (PaletteName::Material, "#eeffff", "#212121"),
            // Arcade coin-op: ice white on deep navy with light-blue accents.
            (PaletteName::Arcade, "#e6f0ff", "#0a1020"),
            (PaletteName::Github, "#c9d1d9", "#0d1117"),
        ];
        for (palette, foreground, background) in ground_truth {
            let colors = palette.colors();
            assert_eq!(
                colors.foreground, foreground,
                "{palette} foreground drifted from the official palette"
            );
            assert_eq!(
                colors.background, background,
                "{palette} background drifted from the official palette"
            );
        }
    }

    #[test]
    fn value_colors_are_valid_hex() {
        for palette in PaletteName::all() {
            let colors = palette.colors();
            let value = value_color(colors.foreground, colors.background);
            assert_eq!(
                value.len(),
                7,
                "{} has a malformed value color {value}",
                palette
            );
            assert!(
                value[1..].chars().all(|c| c.is_ascii_hexdigit()),
                "{} has a non-hex value color {value}",
                palette
            );
        }
    }

    #[test]
    fn dark_theme_values_brighten_toward_white() {
        // The default hypr foreground is a muted blue-gray; values must become
        // a bright near-white so they read like fastfetch in a screenshot.
        let value = value_color("#c0caf5", "#1a1b26");
        assert_eq!(value, "#e3e7fb");
        let (r, g, b) = rgb(&value);
        assert!(
            r > 200.0 && g > 200.0 && b > 200.0,
            "value {value} is not bright"
        );
    }

    #[test]
    fn light_theme_values_stay_dark() {
        // On a light background the value must darken for readability.
        let (r, g, b) = rgb(&value_color("#3c3836", "#fbf1c7"));
        assert!(
            r < 100.0 && g < 100.0 && b < 100.0,
            "light-theme value is not dark"
        );
    }

    #[test]
    fn hex_colors_are_valid() {
        for palette in PaletteName::all() {
            let colors = palette.colors();
            for color in [colors.foreground, colors.background, colors.muted]
                .into_iter()
                .chain(colors.accents)
            {
                let hex = color.trim_start_matches('#');
                assert_eq!(hex.len(), 6, "{} has a malformed color {color}", palette);
                assert!(
                    hex.chars().all(|c| c.is_ascii_hexdigit()),
                    "{} has a non-hex color {color}",
                    palette
                );
            }
        }
    }
}
