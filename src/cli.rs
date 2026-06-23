use std::path::PathBuf;

use clap::Parser;

use crate::model::{ColorMode, LogoPreset, ModuleId, PaletteName, RenderStyle};

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
    #[arg(long)]
    pub share: bool,
    #[arg(long)]
    pub style: Option<RenderStyle>,
    #[arg(long, default_value = "auto")]
    pub color: ColorMode,
    #[arg(short = 'm', long, value_delimiter = ',')]
    pub modules: Option<Vec<ModuleId>>,
    #[arg(long)]
    pub theme: Option<PaletteName>,
    #[arg(long)]
    pub logo: Option<LogoPreset>,
    #[arg(long)]
    pub json: bool,
    #[arg(long)]
    pub diagnostics: bool,
    #[arg(long)]
    pub list_logos: bool,
    #[arg(long)]
    pub list_modules: bool,
    #[arg(long)]
    pub list_themes: bool,
}
