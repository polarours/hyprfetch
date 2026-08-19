use anyhow::Result;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

use crate::animation::Spinner;
use crate::cli::CliArgs;
use crate::config::{self, ModuleConfig};
use crate::logo;
use crate::model::ModuleOutput;
use crate::modules::{ModuleContext, build_module};
use crate::plugins::run_plugin;
use crate::providers::Providers;
use crate::render;

#[derive(Debug, Default)]
pub struct App;

impl App {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&self, cli: CliArgs) -> Result<String> {
        let loaded = config::load(&cli)?;
        let mut runtime = config::merge(&cli, loaded.config);
        let providers = Providers::new();
        let animated = runtime.animated;
        let json = runtime.json;
        let diagnostics = runtime.diagnostics;

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

        let outputs = if animated && !json && !diagnostics {
            Spinner::new("Collecting system information").run(collect)
        } else {
            collect()
        };

        render::render(&outputs, &runtime)
    }

    pub fn run_watch(&self, cli: CliArgs) -> Result<()> {
        let interval = Duration::from_secs(cli.watch_interval);
        loop {
            let output = self.run_cli(cli.clone())?;
            print!("{output}");
            io::stdout().flush().ok();
            thread::sleep(interval);
        }
    }

    fn run_cli(&self, cli: CliArgs) -> Result<String> {
        self.run(cli)
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
