use serde_json::json;

use crate::config::ModuleConfig;
use crate::model::{ModuleId, ModuleOutput, OutputRow};
use crate::modules::{Module, ModuleContext};
use crate::util::humanize_kib;

pub struct CpuModule;

pub struct GpuModule;

pub struct MemoryModule;

pub struct SwapModule;

impl Module for CpuModule {
    fn id(&self) -> ModuleId {
        ModuleId::Cpu
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        let model = ctx.providers.system.cpu_model();
        let usage = ctx.providers.system.cpu_usage();
        match (model, usage) {
            (Ok(cpu), Ok(usage)) => {
                let row = format!(
                    "{cpu}  {percent:.1}% ({n} threads)",
                    percent = usage.usage_percent,
                    n = usage.num_logical_cpus
                );
                ModuleOutput::ok(
                    self.id(),
                    vec![OutputRow::new("CPU", row)],
                    json!({ "model": cpu, "usage_percent": usage.usage_percent, "num_logical_cpus": usage.num_logical_cpus }),
                )
            }
            (Ok(cpu), Err(_)) => ModuleOutput::ok(
                self.id(),
                vec![OutputRow::new("CPU", &cpu)],
                json!({ "model": cpu }),
            ),
            (Err(error), _) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

impl Module for GpuModule {
    fn id(&self) -> ModuleId {
        ModuleId::Gpu
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.gpu_info() {
            Ok(gpus) => {
                let joined = gpus
                    .iter()
                    .map(|gpu| gpu.display_name())
                    .collect::<Vec<_>>()
                    .join(", ");
                ModuleOutput::ok(
                    self.id(),
                    vec![OutputRow::new("GPU", joined.clone())],
                    json!({ "gpus": gpus }),
                )
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

impl Module for MemoryModule {
    fn id(&self) -> ModuleId {
        ModuleId::Memory
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.memory_info() {
            Ok(memory) => {
                let used_kib = memory.total_kib.saturating_sub(memory.available_kib);
                let rendered = format!(
                    "{} / {} ({:.0}%)",
                    humanize_kib(used_kib),
                    humanize_kib(memory.total_kib),
                    memory_usage_percent(used_kib, memory.total_kib)
                );
                ModuleOutput::ok(
                    self.id(),
                    vec![OutputRow::new("Memory", rendered)],
                    json!({
                        "used_kib": used_kib,
                        "available_kib": memory.available_kib,
                        "total_kib": memory.total_kib
                    }),
                )
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

impl Module for SwapModule {
    fn id(&self) -> ModuleId {
        ModuleId::Swap
    }

    fn run(&self, ctx: &ModuleContext<'_>, _config: &ModuleConfig) -> ModuleOutput {
        match ctx.providers.system.swap_info() {
            Ok(swap) => {
                let used_kib = swap.total_kib.saturating_sub(swap.free_kib);
                let rendered = format!(
                    "{} / {} ({:.0}%)",
                    humanize_kib(used_kib),
                    humanize_kib(swap.total_kib),
                    memory_usage_percent(used_kib, swap.total_kib)
                );
                ModuleOutput::ok(
                    self.id(),
                    vec![OutputRow::new("Swap", rendered)],
                    json!({
                        "used_kib": used_kib,
                        "free_kib": swap.free_kib,
                        "total_kib": swap.total_kib
                    }),
                )
            }
            Err(error) => ModuleOutput::unavailable(self.id(), error.to_string()),
        }
    }
}

fn memory_usage_percent(used_kib: u64, total_kib: u64) -> f64 {
    if total_kib == 0 {
        0.0
    } else {
        used_kib as f64 / total_kib as f64 * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_memory_usage_percent() {
        assert_eq!(memory_usage_percent(512, 1024), 50.0);
        assert_eq!(memory_usage_percent(512, 0), 0.0);
    }
}
