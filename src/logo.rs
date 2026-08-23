use std::env;

use crate::model::{LogoPreset, PaletteName};
use crate::providers::system::OsInfo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogoTone {
    Blue,
    Cyan,
    Green,
    Indigo,
    Magenta,
    Red,
    White,
    Yellow,
}

#[derive(Debug, Clone, Copy)]
pub struct LogoDefinition {
    pub preset: LogoPreset,
    pub aliases: &'static [&'static str],
    pub art: &'static str,
    pub primary: LogoTone,
    pub accent: LogoTone,
    pub wordmark: LogoTone,
}

impl LogoDefinition {
    pub fn lines(self) -> Vec<String> {
        self.art.lines().map(strip_fastfetch_markers).collect()
    }

    fn matches(self, candidate: &str) -> bool {
        let candidate = normalize(candidate);
        self.aliases
            .iter()
            .copied()
            .any(|alias| normalize(alias) == candidate)
    }
}

const HYPR_ALIASES: &[&str] = &["hypr", "hyprfetch"];
const LINUX_ALIASES: &[&str] = &["linux", "gnu/linux", "gnu linux", "unknown"];
const WINDOWS_ALIASES: &[&str] = &["windows", "windows nt", "microsoft", "win32", "win64"];
const MACOS_ALIASES: &[&str] = &["macos", "mac os", "osx", "mac os x", "darwin", "mac"];
const ARCH_ALIASES: &[&str] = &[
    "arch",
    "arch linux",
    "archlinux",
    "arcolinux",
    "archbang",
    "archcraft",
    "archlabs",
    "blackarch",
    "blendos",
    "hyperbola",
    "rebornos",
    "steamos",
    "chimeraos",
    "biglinux",
];
const GARUDA_ALIASES: &[&str] = &["garuda", "garuda linux"];
const NOBARA_ALIASES: &[&str] = &["nobara", "nobara project"];
const ARTIX_ALIASES: &[&str] = &["artix", "artix linux"];
const ASAHI_ALIASES: &[&str] = &["asahi", "asahi linux", "asahi remix"];
const CACHYOS_ALIASES: &[&str] = &["cachyos"];
const ENDEAVOUROS_ALIASES: &[&str] = &["endeavouros", "endeavour", "eos"];
const FEDORA_ALIASES: &[&str] = &[
    "fedora",
    "nobara",
    "ultramarine",
    "rhel",
    "redhat",
    "red hat enterprise linux",
    "rocky",
    "almalinux",
    "centos",
    "oraclelinux",
    "bazzite",
    "bluefin",
    "aurora",
    "silverblue",
    "kinoite",
    "sericea",
    "universal blue",
];
const UBUNTU_ALIASES: &[&str] = &[
    "ubuntu",
    "kubuntu",
    "xubuntu",
    "lubuntu",
    "ubuntu mate",
    "ubuntu budgie",
    "ubuntu cinnamon",
    "ubuntu unity",
    "ubuntu studio",
    "ubuntu sway",
    "ubuntu touch",
    "ubuntu kylin",
    "linuxmint",
    "linux mint",
    "elementary",
    "elementaryos",
    "zorin",
    "neon",
    "tuxedo",
    "linuxlite",
    "vanillaos",
    "vanilla",
    "rhinolinux",
    "peppermint",
    "lliurex",
    "pop",
    "popos",
    "pop os",
];
const DEBIAN_ALIASES: &[&str] = &[
    "debian",
    "devuan",
    "kali",
    "parrot",
    "mx",
    "antix",
    "raspbian",
    "raspberry pi os",
    "dietpi",
    "proxmox",
    "pve",
    "truenas scale",
    "deepin",
    "q4os",
    "lilidog",
    "emmabuntus",
    "locos",
];
const NIXOS_ALIASES: &[&str] = &["nixos"];
const OPENSUSE_ALIASES: &[&str] = &[
    "opensuse",
    "open suse",
    "opensuse tumbleweed",
    "opensuse leap",
    "tumbleweed",
    "leap",
    "slowroll",
    "aeon",
    "kairos",
];
const VOID_ALIASES: &[&str] = &["void"];
const ALPINE_ALIASES: &[&str] = &["alpine", "postmarketos", "postmarket os"];
const POP_ALIASES: &[&str] = &["pop", "popos", "pop os"];
const GENTOO_ALIASES: &[&str] = &["gentoo", "funtoo", "calculate", "redcore", "pentoo"];
const MANJARO_ALIASES: &[&str] = &["manjaro"];
const SLACKWARE_ALIASES: &[&str] = &["slackware"];
const MAGEIA_ALIASES: &[&str] = &["mageia"];

pub fn resolve(requested: LogoPreset, os_info: Option<&OsInfo>) -> LogoPreset {
    if !matches!(requested, LogoPreset::Auto) {
        return requested;
    }

    if let Some(os_info) = os_info {
        let mut candidates = Vec::new();
        if let Some(id) = &os_info.id {
            candidates.push(id.as_str());
        }
        candidates.push(os_info.name.as_str());
        for candidate in &os_info.id_like {
            candidates.push(candidate.as_str());
        }

        for candidate in candidates {
            if let Some(definition) = definitions()
                .iter()
                .copied()
                .find(|definition| definition.matches(candidate))
            {
                return definition.preset;
            }
        }
    }

    platform_fallback()
}

pub fn definition(preset: LogoPreset) -> LogoDefinition {
    match preset {
        LogoPreset::Auto => definition(platform_fallback()),
        LogoPreset::None => LogoDefinition {
            preset,
            aliases: &["none"],
            art: "",
            primary: LogoTone::White,
            accent: LogoTone::White,
            wordmark: LogoTone::White,
        },
        LogoPreset::Minimal => LogoDefinition {
            preset,
            aliases: &["minimal"],
            art: "hyprfetch",
            primary: LogoTone::Cyan,
            accent: LogoTone::Cyan,
            wordmark: LogoTone::Cyan,
        },
        _ => definitions()
            .iter()
            .copied()
            .find(|definition| definition.preset == preset)
            .unwrap_or_else(|| definition(platform_fallback())),
    }
}

pub fn definitions() -> &'static [LogoDefinition] {
    &[
        LogoDefinition {
            preset: LogoPreset::Hypr,
            aliases: HYPR_ALIASES,
            art: include_str!("../assets/logos/linux.txt"),
            primary: LogoTone::Blue,
            accent: LogoTone::Cyan,
            wordmark: LogoTone::White,
        },
        LogoDefinition {
            preset: LogoPreset::Linux,
            aliases: LINUX_ALIASES,
            art: include_str!("../assets/logos/linux.txt"),
            primary: LogoTone::Blue,
            accent: LogoTone::Cyan,
            wordmark: LogoTone::White,
        },
        LogoDefinition {
            preset: LogoPreset::Windows,
            aliases: WINDOWS_ALIASES,
            art: include_str!("../assets/logos/windows.txt"),
            primary: LogoTone::Blue,
            accent: LogoTone::Cyan,
            wordmark: LogoTone::White,
        },
        LogoDefinition {
            preset: LogoPreset::Macos,
            aliases: MACOS_ALIASES,
            art: include_str!("../assets/logos/macos.txt"),
            primary: LogoTone::White,
            accent: LogoTone::Cyan,
            wordmark: LogoTone::White,
        },
        LogoDefinition {
            preset: LogoPreset::Arch,
            aliases: ARCH_ALIASES,
            art: include_str!("../assets/logos/arch.txt"),
            primary: LogoTone::Blue,
            accent: LogoTone::Cyan,
            wordmark: LogoTone::White,
        },
        LogoDefinition {
            preset: LogoPreset::Garuda,
            aliases: GARUDA_ALIASES,
            art: include_str!("../assets/logos/garuda.txt"),
            primary: LogoTone::Red,
            accent: LogoTone::Yellow,
            wordmark: LogoTone::White,
        },
        LogoDefinition {
            preset: LogoPreset::Nobara,
            aliases: NOBARA_ALIASES,
            art: include_str!("../assets/logos/nobara.txt"),
            primary: LogoTone::Blue,
            accent: LogoTone::Cyan,
            wordmark: LogoTone::White,
        },
        LogoDefinition {
            preset: LogoPreset::Artix,
            aliases: ARTIX_ALIASES,
            art: include_str!("../assets/logos/artix.txt"),
            primary: LogoTone::Cyan,
            accent: LogoTone::Magenta,
            wordmark: LogoTone::White,
        },
        LogoDefinition {
            preset: LogoPreset::Asahi,
            aliases: ASAHI_ALIASES,
            art: include_str!("../assets/logos/asahi.txt"),
            primary: LogoTone::Green,
            accent: LogoTone::Cyan,
            wordmark: LogoTone::White,
        },
        LogoDefinition {
            preset: LogoPreset::Cachyos,
            aliases: CACHYOS_ALIASES,
            art: include_str!("../assets/logos/cachyos.txt"),
            primary: LogoTone::Cyan,
            accent: LogoTone::Magenta,
            wordmark: LogoTone::White,
        },
        LogoDefinition {
            preset: LogoPreset::Endeavouros,
            aliases: ENDEAVOUROS_ALIASES,
            art: include_str!("../assets/logos/endeavouros.txt"),
            primary: LogoTone::Indigo,
            accent: LogoTone::Magenta,
            wordmark: LogoTone::White,
        },
        LogoDefinition {
            preset: LogoPreset::Fedora,
            aliases: FEDORA_ALIASES,
            art: include_str!("../assets/logos/fedora.txt"),
            primary: LogoTone::Blue,
            accent: LogoTone::Cyan,
            wordmark: LogoTone::White,
        },
        LogoDefinition {
            preset: LogoPreset::Ubuntu,
            aliases: UBUNTU_ALIASES,
            art: include_str!("../assets/logos/ubuntu.txt"),
            primary: LogoTone::Yellow,
            accent: LogoTone::Red,
            wordmark: LogoTone::White,
        },
        LogoDefinition {
            preset: LogoPreset::Debian,
            aliases: DEBIAN_ALIASES,
            art: include_str!("../assets/logos/debian.txt"),
            primary: LogoTone::Magenta,
            accent: LogoTone::Red,
            wordmark: LogoTone::White,
        },
        LogoDefinition {
            preset: LogoPreset::Nixos,
            aliases: NIXOS_ALIASES,
            art: include_str!("../assets/logos/nixos.txt"),
            primary: LogoTone::Blue,
            accent: LogoTone::Cyan,
            wordmark: LogoTone::White,
        },
        LogoDefinition {
            preset: LogoPreset::Opensuse,
            aliases: OPENSUSE_ALIASES,
            art: include_str!("../assets/logos/opensuse.txt"),
            primary: LogoTone::Green,
            accent: LogoTone::Yellow,
            wordmark: LogoTone::White,
        },
        LogoDefinition {
            preset: LogoPreset::Void,
            aliases: VOID_ALIASES,
            art: include_str!("../assets/logos/void.txt"),
            primary: LogoTone::Green,
            accent: LogoTone::Cyan,
            wordmark: LogoTone::White,
        },
        LogoDefinition {
            preset: LogoPreset::Alpine,
            aliases: ALPINE_ALIASES,
            art: include_str!("../assets/logos/alpine.txt"),
            primary: LogoTone::Blue,
            accent: LogoTone::Cyan,
            wordmark: LogoTone::White,
        },
        LogoDefinition {
            preset: LogoPreset::Pop,
            aliases: POP_ALIASES,
            art: include_str!("../assets/logos/pop.txt"),
            primary: LogoTone::Yellow,
            accent: LogoTone::Cyan,
            wordmark: LogoTone::White,
        },
        LogoDefinition {
            preset: LogoPreset::Gentoo,
            aliases: GENTOO_ALIASES,
            art: include_str!("../assets/logos/gentoo.txt"),
            primary: LogoTone::Indigo,
            accent: LogoTone::Magenta,
            wordmark: LogoTone::White,
        },
        LogoDefinition {
            preset: LogoPreset::Manjaro,
            aliases: MANJARO_ALIASES,
            art: include_str!("../assets/logos/manjaro.txt"),
            primary: LogoTone::Green,
            accent: LogoTone::Cyan,
            wordmark: LogoTone::White,
        },
        LogoDefinition {
            preset: LogoPreset::Slackware,
            aliases: SLACKWARE_ALIASES,
            art: include_str!("../assets/logos/slackware.txt"),
            primary: LogoTone::Blue,
            accent: LogoTone::Cyan,
            wordmark: LogoTone::White,
        },
        LogoDefinition {
            preset: LogoPreset::Mageia,
            aliases: MAGEIA_ALIASES,
            art: include_str!("../assets/logos/mageia.txt"),
            primary: LogoTone::Cyan,
            accent: LogoTone::Blue,
            wordmark: LogoTone::White,
        },
    ]
}

pub fn tone_for_line(
    definition: LogoDefinition,
    palette: PaletteName,
    line_index: usize,
    line_count: usize,
) -> LogoTone {
    if matches!(palette, PaletteName::Mono) {
        return LogoTone::White;
    }

    if line_count <= 1 || line_index + 1 == line_count {
        return definition.wordmark;
    }

    if line_index < line_count / 2 {
        definition.primary
    } else {
        definition.accent
    }
}

fn normalize(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn strip_fastfetch_markers(line: &str) -> String {
    let mut output = String::with_capacity(line.len());
    let mut characters = line.chars().peekable();

    while let Some(character) = characters.next() {
        if character == '$' {
            match characters.peek().copied() {
                Some('$') => {
                    output.push('$');
                    characters.next();
                }
                Some(next) if next.is_ascii_alphanumeric() => {
                    characters.next();
                }
                _ => {}
            }
            continue;
        }

        output.push(character);
    }

    output
}

fn platform_fallback() -> LogoPreset {
    match env::consts::OS {
        "windows" => LogoPreset::Windows,
        "macos" => LogoPreset::Macos,
        "linux" => LogoPreset::Linux,
        _ => LogoPreset::Linux,
    }
}

#[cfg(test)]
mod tests {
    use super::{definition, resolve, strip_fastfetch_markers};
    use crate::model::LogoPreset;
    use crate::providers::system::OsInfo;

    #[test]
    fn resolves_explicit_logo_without_detection() {
        let os = OsInfo {
            name: "Ubuntu".to_string(),
            id: Some("ubuntu".to_string()),
            id_like: vec!["debian".to_string()],
        };

        assert_eq!(resolve(LogoPreset::Arch, Some(&os)), LogoPreset::Arch);
    }

    #[test]
    fn resolves_logo_from_os_id() {
        let os = OsInfo {
            name: "Arch Linux".to_string(),
            id: Some("arch".to_string()),
            id_like: vec![],
        };

        assert_eq!(resolve(LogoPreset::Auto, Some(&os)), LogoPreset::Arch);
    }

    #[test]
    fn resolves_logo_from_os_family() {
        let os = OsInfo {
            name: "Linux Mint".to_string(),
            id: Some("linuxmint".to_string()),
            id_like: vec!["ubuntu".to_string(), "debian".to_string()],
        };

        assert_eq!(resolve(LogoPreset::Auto, Some(&os)), LogoPreset::Ubuntu);
    }

    #[test]
    fn resolves_logo_from_name_alias() {
        let os = OsInfo {
            name: "Raspberry Pi OS".to_string(),
            id: Some("raspbian".to_string()),
            id_like: vec!["debian".to_string()],
        };

        assert_eq!(resolve(LogoPreset::Auto, Some(&os)), LogoPreset::Debian);
    }

    #[test]
    fn resolves_arch_derivative_logo() {
        let os = OsInfo {
            name: "ArcoLinux".to_string(),
            id: Some("arcolinux".to_string()),
            id_like: vec!["arch".to_string()],
        };

        assert_eq!(resolve(LogoPreset::Auto, Some(&os)), LogoPreset::Arch);
    }

    #[test]
    fn resolves_fedora_derivative_logo() {
        let os = OsInfo {
            name: "Bazzite".to_string(),
            id: Some("bazzite".to_string()),
            id_like: vec!["fedora".to_string()],
        };

        assert_eq!(resolve(LogoPreset::Auto, Some(&os)), LogoPreset::Fedora);
    }

    #[test]
    fn falls_back_to_platform_logo() {
        let os = OsInfo {
            name: "Unknown Distribution".to_string(),
            id: Some("mystery".to_string()),
            id_like: vec!["something".to_string()],
        };

        assert_eq!(resolve(LogoPreset::Auto, Some(&os)), LogoPreset::Linux);
    }

    #[test]
    fn builtin_logo_art_is_present() {
        assert!(!definition(LogoPreset::Arch).lines().is_empty());
        assert!(!definition(LogoPreset::Windows).lines().is_empty());
        assert!(!definition(LogoPreset::Macos).lines().is_empty());
    }

    #[test]
    fn strips_fastfetch_color_markers() {
        assert_eq!(strip_fastfetch_markers("$2#####$1#####"), "##########");
        assert_eq!(strip_fastfetch_markers("$1hello $$ world"), "hello $ world");
    }
}
