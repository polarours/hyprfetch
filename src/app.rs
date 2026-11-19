use anyhow::Result;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

use crate::animation::{self, Spinner};
use crate::cli::CliArgs;
use crate::config::{self, ModuleConfig};
use crate::logo;
use crate::model::ModuleOutput;
use crate::modules::{ModuleContext, build_module};
use crate::plugins::run_plugin;
use crate::providers::Providers;
use crate::render;
use crate::util;

#[derive(Debug, Default)]
pub struct App;

impl App {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&self, cli: CliArgs) -> Result<String> {
        Ok(self.render_with_reveal(cli)?.0)
    }

    /// Render the fetch output and decide how to present it: static, a plain
    /// line-by-line reveal, or the two-phase logo-first reveal.
    fn render_with_reveal(&self, cli: CliArgs) -> Result<(String, animation::Reveal)> {
        let loaded = config::load(&cli)?;
        let mut runtime = config::merge(&cli, loaded.config);
        let providers = Providers::new();
        let json = runtime.json;
        let diagnostics = runtime.diagnostics;
        // `static` turns every motion off, spinner included.
        let animated = runtime.animation != animation::RevealStyle::Static;
        let reveal = if animation::should_reveal(
            animated,
            json,
            diagnostics,
            runtime.is_plain(),
            util::stdout_is_tty(),
        ) {
            animation::Reveal::Lines
        } else {
            animation::Reveal::Static
        };
        let show_spinner = should_show_spinner(
            animated,
            json,
            diagnostics,
            runtime.is_plain(),
            runtime.appearance.color == crate::model::ColorMode::Never,
            util::stdout_is_tty(),
        );
        let spinner_palette = runtime.appearance.palette;
        let spinner_colors = runtime.appearance.color == crate::model::ColorMode::Always
            || (runtime.appearance.color == crate::model::ColorMode::Auto && util::stdout_is_tty());

        let mut collect = || {
            providers.hyprland.prefetch();
            let os_info = providers.system.os_info().ok();
            runtime.resolved_logo = logo::resolve(runtime.logo, os_info.as_ref());
            let context = ModuleContext {
                runtime: &runtime,
                providers: &providers,
            };

            let mut outputs = runtime
                .modules
                .iter()
                .map(|module_config| {
                    let output = build_module(module_config.name).run(&context, module_config);
                    apply_module_display_options(output, module_config)
                })
                .collect::<Vec<_>>();
            outputs.extend(runtime.plugins.iter().map(run_plugin));
            outputs
        };

        let outputs = if show_spinner {
            Spinner::with_palette(
                "Collecting system information",
                spinner_palette,
                spinner_colors,
            )
            .run(collect)
        } else {
            collect()
        };

        let (output, logo_width) = render::render(&outputs, &runtime)?;
        // The chosen style upgrades the reveal now that the logo width is
        // known: a layout without a logo column falls back to plain lines,
        // and a gated run stays static whatever the style says.
        let reveal = match (runtime.animation, logo_width) {
            (animation::RevealStyle::Split, Some(width)) if reveal == animation::Reveal::Lines => {
                animation::Reveal::TwoPhase { width }
            }
            (animation::RevealStyle::Wave, Some(width)) if reveal == animation::Reveal::Lines => {
                animation::Reveal::Wave { width }
            }
            (_, _) => reveal,
        };
        Ok((output, reveal))
    }

    /// The single-shot run: render, then show the output — animated when the
    /// terminal and the mode allow it, printed at once otherwise.
    pub fn run_once(&self, cli: CliArgs) -> Result<()> {
        let (output, reveal) = self.render_with_reveal(cli)?;
        animation::present(&output, &reveal);
        io::stdout().flush().ok();
        Ok(())
    }

    pub fn run_watch(&self, cli: CliArgs) -> Result<()> {
        let interval = Duration::from_secs(watch_interval_seconds(cli.watch_interval));
        let redraw = util::stdout_is_tty();
        let mut first = true;

        loop {
            if redraw && !first {
                // Move the cursor home and clear the rest of the screen so each
                // refresh replaces the previous one instead of scrolling.
                print!("\x1b[H\x1b[J");
            }

            let (output, reveal) = self.render_with_reveal(cli.clone())?;
            // Only the first frame animates: a watch that flickers row by row
            // on every refresh would be exhausting instead of elegant.
            let reveal = if first {
                reveal
            } else {
                animation::Reveal::Static
            };
            animation::present(&output, &reveal);
            first = false;
            io::stdout().flush().ok();
            thread::sleep(interval);
        }
    }
}

/// A zero-second watch interval would make `run_watch` spin at full CPU,
/// reloading config and every module in a tight loop. Clamp it to one second.
fn watch_interval_seconds(requested: u64) -> u64 {
    requested.max(1)
}

fn should_show_spinner(
    animated: bool,
    json: bool,
    diagnostics: bool,
    plain: bool,
    color_never: bool,
    is_tty: bool,
) -> bool {
    animated && !json && !diagnostics && !plain && !color_never && is_tty
}

#[cfg(test)]
mod spinner_tests {
    use super::{should_show_spinner, watch_interval_seconds};

    #[test]
    fn watch_interval_never_spins_at_zero() {
        assert_eq!(watch_interval_seconds(0), 1);
        assert_eq!(watch_interval_seconds(5), 5);
    }

    #[test]
    fn spinner_needs_a_terminal() {
        assert!(should_show_spinner(true, false, false, false, false, true));
        assert!(!should_show_spinner(
            true, false, false, false, false, false
        ));
    }

    #[test]
    fn spinner_respects_plain_and_machine_modes() {
        assert!(!should_show_spinner(true, false, false, true, false, true));
        assert!(!should_show_spinner(true, true, false, false, false, true));
        assert!(!should_show_spinner(true, false, true, false, false, true));
        assert!(!should_show_spinner(true, false, false, false, true, true));
    }
}

fn apply_module_display_options(
    mut output: ModuleOutput,
    module_config: &ModuleConfig,
) -> ModuleOutput {
    let labels = module_config.option_string_map("labels");
    if !labels.is_empty() {
        for row in &mut output.rows {
            if let Some(label) = labels.get(&row.key) {
                row.key.clone_from(label);
            }
        }
    }

    if output.rows.len() == 1
        && let Some(label) = module_config.option_string("label")
    {
        output.rows[0].key = label;
    }

    output
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::json;
    use toml::Value;

    use super::apply_module_display_options;
    use crate::config::ModuleConfig;
    use crate::model::{ModuleId, ModuleOutput, OutputRow};

    #[test]
    fn applies_single_row_label_override() {
        let mut options = BTreeMap::new();
        options.insert("label".to_string(), Value::String("Distro".to_string()));
        let config = ModuleConfig {
            name: ModuleId::Os,
            enabled: true,
            options,
        };
        let output = ModuleOutput::ok(
            ModuleId::Os,
            vec![OutputRow::new("OS", "Arch Linux")],
            json!({}),
        );

        let output = apply_module_display_options(output, &config);
        assert_eq!(output.rows[0].key, "Distro");
    }

    #[test]
    fn applies_multi_row_label_map_override() {
        let mut labels = toml::map::Map::new();
        labels.insert("Workspace".to_string(), Value::String("WS".to_string()));
        labels.insert("Clients".to_string(), Value::String("Wins".to_string()));

        let mut options = BTreeMap::new();
        options.insert("labels".to_string(), Value::Table(labels));
        let config = ModuleConfig {
            name: ModuleId::Hyprland,
            enabled: true,
            options,
        };
        let output = ModuleOutput::ok(
            ModuleId::Hyprland,
            vec![
                OutputRow::new("Hyprland", "0.48.1"),
                OutputRow::new("Workspace", "1"),
                OutputRow::new("Clients", "4"),
            ],
            json!({}),
        );

        let output = apply_module_display_options(output, &config);
        assert_eq!(output.rows[0].key, "Hyprland");
        assert_eq!(output.rows[1].key, "WS");
        assert_eq!(output.rows[2].key, "Wins");
    }
}
