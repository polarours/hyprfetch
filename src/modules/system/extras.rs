//! Load average, process count, logged-in users and the local IP.
//!
//! All four are cheap local reads, so they can live in the default set without
//! slowing the render down.

use serde_json::json;

use crate::config::ModuleConfig;
use crate::model::{ModuleId, ModuleOutput, OutputRow};
use crate::modules::{Module, ModuleContext};

pub struct LoadAverageModule;

pub struct ProcessesModule;

pub struct UsersModule;

pub struct LocalIpModule;

impl Module for LoadAverageModule {
    fn id(&self) -> ModuleId {
        ModuleId::LoadAverage
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.load_average() {
            Some(load) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new("Load", load.display())],
                json!({
                    "one": load.one,
                    "five": load.five,
                    "fifteen": load.fifteen,
                    "running": load.running,
                    "total": load.total,
                }),
            ),
            None => ModuleOutput::unavailable(self.id(), "loadavg unavailable".to_string()),
        }
    }
}

impl Module for ProcessesModule {
    fn id(&self) -> ModuleId {
        ModuleId::Processes
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.process_count() {
            Some(count) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new("Processes", count.to_string())],
                json!({ "count": count }),
            ),
            None => ModuleOutput::unavailable(self.id(), "process list unavailable".to_string()),
        }
    }
}

impl Module for UsersModule {
    fn id(&self) -> ModuleId {
        ModuleId::Users
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.logged_in_users() {
            Some(users) => {
                let value = if users.len() == 1 {
                    users[0].clone()
                } else {
                    format!("{} ({})", users.join(", "), users.len())
                };
                ModuleOutput::ok(
                    self.id(),
                    vec![OutputRow::new("Users", value)],
                    json!({ "users": users, "count": users.len() }),
                )
            }
            None => ModuleOutput::unavailable(self.id(), "no logged-in users".to_string()),
        }
    }
}

impl Module for LocalIpModule {
    fn id(&self) -> ModuleId {
        ModuleId::LocalIp
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.local_ip() {
            Some(ip) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new("Local IP", ip.clone())],
                json!({ "address": ip }),
            ),
            None => ModuleOutput::unavailable(self.id(), "no routable address".to_string()),
        }
    }
}
