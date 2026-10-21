use anyhow::Result;
use clap::Parser;
use hyprfetch::app::App;
use hyprfetch::cli::CliArgs;
use hyprfetch::config;
use hyprfetch::logo::LogoPreset;
use hyprfetch::model::{ModuleId, PaletteName};

fn main() -> Result<()> {
    let cli = CliArgs::parse();

    if cli.print_config {
        print!("{}", config::default_config_toml());
        return Ok(());
    }

    if cli.init_config {
        let path = config::init_config(&cli)?;
        println!("created config at {}", path.display());
        return Ok(());
    }

    if cli.list_modules {
        for module in ModuleId::all() {
            println!("{module}");
        }
        return Ok(());
    }

    if cli.list_themes {
        for palette in PaletteName::all() {
            println!("{palette}");
        }
        return Ok(());
    }

    if cli.list_logos {
        for preset in LogoPreset::all() {
            println!("{preset}");
        }
        return Ok(());
    }

    if cli.watch {
        return App::new().run_watch(cli);
    }

    let output = App::new().run(cli)?;
    print!("{output}");
    Ok(())
}
