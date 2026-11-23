use serde_json::json;

use crate::config::ModuleConfig;
use crate::model::{ModuleId, ModuleOutput, OutputRow};
use crate::modules::{Module, ModuleContext};
use crate::util::humanize_uptime;

pub struct OsModule;

pub struct HostModule;

pub struct KernelModule;

pub struct PackagesModule;

pub struct UptimeModule;

pub struct ShellModule;

pub struct TerminalModule;

impl Module for OsModule {
    fn id(&self) -> ModuleId {
        ModuleId::Os
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.os_info() {
            Ok(info) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new("OS", info.name.clone())],
                json!({ "name": info.name }),
            ),
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

impl Module for HostModule {
    fn id(&self) -> ModuleId {
        ModuleId::Host
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.host_info() {
            Ok(host) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new("Host", host.display_name())],
                json!(host),
            ),
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

impl Module for KernelModule {
    fn id(&self) -> ModuleId {
        ModuleId::Kernel
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.kernel_version() {
            Ok(kernel) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new("Kernel", kernel.clone())],
                json!({ "version": kernel }),
            ),
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

impl Module for PackagesModule {
    fn id(&self) -> ModuleId {
        ModuleId::Packages
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.package_info() {
            Ok(packages) => {
                let rendered = packages
                    .iter()
                    .map(|info| format!("{} ({})", info.count, info.manager))
                    .collect::<Vec<_>>()
                    .join(", ");
                ModuleOutput::ok(
                    self.id(),
                    vec![OutputRow::new("Packages", rendered)],
                    json!(packages),
                )
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

impl Module for UptimeModule {
    fn id(&self) -> ModuleId {
        ModuleId::Uptime
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.uptime_seconds() {
            Ok(seconds) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new("Uptime", humanize_uptime(seconds))],
                json!({ "seconds": seconds }),
            ),
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

impl Module for ShellModule {
    fn id(&self) -> ModuleId {
        ModuleId::Shell
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.shell_name() {
            Some(shell) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new("Shell", shell.clone())],
                json!({ "name": shell }),
            ),
            None => ModuleOutput::unavailable(self.id(), "shell not detected"),
        }
    }
}

impl Module for TerminalModule {
    fn id(&self) -> ModuleId {
        ModuleId::Terminal
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.terminal_name() {
            Some(terminal) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new("Terminal", terminal.clone())],
                json!({ "name": terminal }),
            ),
            None => ModuleOutput::unavailable(self.id(), "terminal not detected"),
        }
    }
}
