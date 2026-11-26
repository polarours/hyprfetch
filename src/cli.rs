use std::path::PathBuf;

use clap::Parser;

use crate::animation::RevealStyle;
use crate::logo::LogoPreset;
use crate::model::{ColorMode, ModuleId, PaletteName, RenderStyle};

#[derive(Debug, Clone, Parser)]
#[command(
    name = "hyprfetch",
    version,
    about = "A Hyprland-native fetch utility",
    disable_help_subcommand = true
)]
pub struct CliArgs {
    #[arg(short, long)]
    pub config: Option<PathBuf>,
    #[arg(long)]
    pub init_config: bool,
    #[arg(long)]
    pub print_config: bool,
    #[arg(long)]
    pub no_config: bool,
    #[arg(long)]
    pub plain: bool,
    /// Skip expensive probes (CPU sampling, GPU vendor tools, audio/network/media
    /// commands) for a near-instant render. Module rows stay, with fewer values.
    #[arg(long)]
    pub fast: bool,
    #[arg(long)]
    pub share: bool,
    /// How the output is animated: split (logo skeleton first), lines, wave
    /// (the logo lights up column by column) or static for no animation.
    #[arg(long)]
    pub animation: Option<RevealStyle>,
    #[arg(long)]
    pub watch: bool,
    #[arg(long, default_value = "1")]
    pub watch_interval: u64,
    #[arg(long)]
    pub style: Option<RenderStyle>,
    #[arg(long, default_value = "auto")]
    pub color: ColorMode,
    /// Load a built-in preset (all, rice or minimal) before CLI overrides.
    #[arg(long)]
    pub preset: Option<crate::config::Preset>,
    /// Load a standalone theme file ([appearance] table) that sets only the look.
    #[arg(long)]
    pub theme_file: Option<PathBuf>,
    #[arg(short = 'm', long, value_delimiter = ',')]
    pub modules: Option<Vec<ModuleId>>,
    #[arg(long)]
    pub theme: Option<PaletteName>,
    #[arg(long)]
    pub logo: Option<LogoPreset>,
    /// Load custom ASCII logo artwork from a file.
    #[arg(long)]
    pub logo_file: Option<PathBuf>,
    /// Draw a PNG as the logo with the kitty graphics protocol (kitty, ghostty,
    /// WezTerm). Implies the plain column layout instead of the arcade view.
    #[arg(long, value_name = "PATH")]
    pub logo_image: Option<PathBuf>,
    /// Maximum width of that image in terminal cells. 0 derives it from the
    /// number of information rows, so the picture fills the text block.
    #[arg(long, value_name = "COLS", default_value_t = 0)]
    pub logo_image_cols: u32,
    #[arg(long)]
    pub json: bool,
    #[arg(long)]
    pub diagnostics: bool,
    /// Explain why modules failed and how to fix them.
    #[arg(long)]
    pub doctor: bool,
    /// Render with an 80s arcade CRT look (on by default).
    #[arg(long)]
    pub retro: bool,
    /// Disable the default arcade styling.
    #[arg(long)]
    pub no_retro: bool,
    /// Force Nerd Font icons on (they are already the default).
    #[arg(long)]
    pub icons: bool,
    /// Turn the icons off: plain [LABEL] tags only, for terminals whose font
    /// lacks the Nerd Font private-use glyphs.
    #[arg(long)]
    pub no_icons: bool,
    #[arg(long)]
    pub list_logos: bool,
    #[arg(long)]
    pub list_modules: bool,
    #[arg(long)]
    pub list_themes: bool,
}
