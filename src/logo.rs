//! Built-in logos and distribution detection.
//!
//! The catalogue is a single table: every logo needs one entry, and the
//! enum, its `Display`/`FromStr` implementations and the definition list are
//! all generated from it. That keeps the selectable names, the os-release
//! aliases and the artwork in sync.

use std::env;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::model::PaletteName;
use crate::providers::system::OsInfo;

/// A colour applied to a logo line.
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

/// A built-in logo: its selectable name, the os-release values that select
/// it, the artwork and the colours used to render it.
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

/// Declare the logo catalogue.
///
/// Every entry provides the enum variant, the selectable name, the os-release
/// aliases, the artwork and the three tones. The enum, `all`, `Display`,
/// `FromStr` and the definition list are generated from the same entries so
/// they cannot drift apart.
macro_rules! logo_catalog {
    (
        $(
            $variant:ident => $name:literal,
            [$($alias:literal),* $(,)?],
            $art:expr,
            $primary:ident, $accent:ident, $wordmark:ident
        );* $(;)?
    ) => {
        /// A built-in logo.
        ///
        /// The catalogue holds one entry per logo. Every entry names the
        /// variant, the name the preset is selected by, the os-release values
        /// that select it, the artwork and the three tones. The entries are
        /// grouped by family: the platform logos first, then the Arch, Fedora,
        /// Debian, Ubuntu, SUSE and Gentoo families, the other Linux
        /// distributions, the enterprise and appliance ones and finally the
        /// BSDs.
        ///
        /// The aliases are matched case-insensitively and without punctuation,
        /// so `linux mint` and `linuxmint` are the same alias.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
        #[serde(rename_all = "kebab-case")]
        pub enum LogoPreset {
            /// Pick the logo that matches the running distribution.
            #[default]
            Auto,
            $(
                #[serde(rename = $name)]
                $variant,
            )*
        }

        impl LogoPreset {
            /// Every selectable logo.
            pub const fn all() -> &'static [Self] {
                &[Self::Auto, $(Self::$variant,)*]
            }

            /// The catalogue name of this logo.
            pub const fn name(self) -> &'static str {
                match self {
                    Self::Auto => "auto",
                    $(Self::$variant => $name,)*
                }
            }
        }

        impl Display for LogoPreset {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.name())
            }
        }

        impl FromStr for LogoPreset {
            type Err = String;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                let normalized = normalize(value);
                if normalized == "auto" {
                    return Ok(Self::Auto);
                }
                $(
                    if normalize($name) == normalized $(|| normalize($alias) == normalized)* {
                        return Ok(Self::$variant);
                    }
                )*
                Err(format!("invalid logo preset: {value}"))
            }
        }

        /// Every built-in logo definition, in catalogue order.
        pub const fn definitions() -> &'static [LogoDefinition] {
            &[$(LogoDefinition {
                preset: LogoPreset::$variant,
                aliases: &[$name, $($alias),*],
                art: $art,
                primary: LogoTone::$primary,
                accent: LogoTone::$accent,
                wordmark: LogoTone::$wordmark,
            },)*]
        }
    };
}

// The catalogue of the built-in logos. See the `LogoPreset` type for how an
// entry is put together.
logo_catalog! {
    None => "none", [], "", White, White, White;
    Minimal => "minimal", [], "hyprfetch", Cyan, Cyan, Cyan;
    Hypr => "hypr", ["hyprfetch"], include_str!("../assets/logos/linux.txt"), Blue, Cyan, White;
    Linux => "linux", ["gnu/linux", "unknown"], include_str!("../assets/logos/linux.txt"), Blue, Cyan, White;
    Windows => "windows", ["windows nt", "microsoft", "win32", "win64"], include_str!("../assets/logos/windows.txt"), Blue, Cyan, White;
    Macos => "macos", ["osx", "mac os x", "darwin", "mac"], include_str!("../assets/logos/macos.txt"), White, Cyan, White;
    Arch => "arch", ["arch linux", "archbang", "blendos", "rebornos"], include_str!("../assets/logos/arch.txt"), Blue, Cyan, White;
    Arcolinux => "arcolinux", [], include_str!("../assets/logos/arco.txt"), Cyan, Magenta, White;
    Archcraft => "archcraft", [], include_str!("../assets/logos/archcraft.txt"), Cyan, Blue, White;
    Archlabs => "archlabs", [], include_str!("../assets/logos/archlabs.txt"), Blue, Cyan, White;
    Blackarch => "blackarch", [], include_str!("../assets/logos/blackarch.txt"), Blue, Cyan, White;
    Hyperbola => "hyperbola", [], include_str!("../assets/logos/hyperbola.txt"), Blue, Cyan, White;
    Steamos => "steamos", [], include_str!("../assets/logos/steamos.txt"), Blue, Cyan, White;
    Steamdeck => "steamdeck", [], include_str!("../assets/logos/steamdeck.txt"), Blue, Cyan, White;
    Chimera => "chimera", ["chimeraos", "chimera linux"], include_str!("../assets/logos/chimera_linux.txt"), Cyan, Blue, White;
    Biglinux => "biglinux", [], include_str!("../assets/logos/biglinux.txt"), Magenta, Blue, White;
    Manjaro => "manjaro", [], include_str!("../assets/logos/manjaro.txt"), Green, Cyan, White;
    Garuda => "garuda", ["garuda linux"], include_str!("../assets/logos/garuda.txt"), Red, Yellow, White;
    Cachyos => "cachyos", [], include_str!("../assets/logos/cachyos.txt"), Cyan, Magenta, White;
    Endeavouros => "endeavouros", ["endeavour", "eos"], include_str!("../assets/logos/endeavouros.txt"), Indigo, Magenta, White;
    Artix => "artix", ["artix linux"], include_str!("../assets/logos/artix.txt"), Cyan, Magenta, White;
    Asahi => "asahi", ["asahi linux", "asahi remix"], include_str!("../assets/logos/asahi.txt"), Green, Cyan, White;
    Fedora => "fedora", ["ultramarine", "rhel", "redhat", "red hat enterprise linux", "bazzite", "bluefin", "aurora", "universal blue"], include_str!("../assets/logos/fedora.txt"), Blue, Cyan, White;
    Rocky => "rocky", ["rocky linux"], include_str!("../assets/logos/rocky.txt"), Green, Cyan, White;
    Almalinux => "almalinux", [], include_str!("../assets/logos/almalinux.txt"), Blue, Cyan, White;
    Centos => "centos", ["centos linux", "centos stream"], include_str!("../assets/logos/centos.txt"), Magenta, Blue, White;
    Oracle => "oracle", ["ol", "oracle linux", "oracle linux server"], include_str!("../assets/logos/oracle.txt"), Red, White, White;
    Silverblue => "silverblue", ["fedora silverblue"], include_str!("../assets/logos/fedora_silverblue.txt"), Blue, Cyan, White;
    Kinoite => "kinoite", ["fedora kinoite"], include_str!("../assets/logos/fedora_kinoite.txt"), Blue, Cyan, White;
    Sericea => "sericea", ["fedora sericea"], include_str!("../assets/logos/fedora_sericea.txt"), Blue, Cyan, White;
    Nobara => "nobara", ["nobara project"], include_str!("../assets/logos/nobara.txt"), Blue, Cyan, White;
    Coreos => "coreos", [], include_str!("../assets/logos/fedora_coreos.txt"), Blue, Cyan, White;
    Debian => "debian", ["debian gnu/linux"], include_str!("../assets/logos/debian.txt"), Red, Magenta, White;
    Devuan => "devuan", [], include_str!("../assets/logos/devuan.txt"), Red, Magenta, White;
    Kali => "kali", ["kali linux", "kali gnu/linux"], include_str!("../assets/logos/kali.txt"), Blue, Cyan, White;
    Parrot => "parrot", ["parrot os", "parrot security"], include_str!("../assets/logos/parrot.txt"), Cyan, White, White;
    Raspbian => "raspbian", ["raspberry pi os"], include_str!("../assets/logos/raspbian.txt"), Red, Cyan, White;
    Mx => "mx", ["mx linux"], include_str!("../assets/logos/mx.txt"), Green, Cyan, White;
    Antix => "antix", ["antiX linux"], include_str!("../assets/logos/antix.txt"), Cyan, White, White;
    Dietpi => "dietpi", [], include_str!("../assets/logos/dietpi.txt"), Red, Cyan, White;
    Deepin => "deepin", [], include_str!("../assets/logos/deepin.txt"), Blue, Cyan, White;
    Q4os => "q4os", [], include_str!("../assets/logos/q4os.txt"), Blue, Cyan, White;
    Proxmox => "proxmox", ["pve", "proxmox ve", "proxmox virtual environment"], include_str!("../assets/logos/proxmox.txt"), Yellow, White, White;
    Truenas => "truenas", ["truenas scale"], include_str!("../assets/logos/truenas_scale.txt"), Blue, Cyan, White;
    Emmabuntus => "emmabuntus", [], include_str!("../assets/logos/emmabuntus.txt"), Cyan, White, White;
    Lilidog => "lilidog", [], include_str!("../assets/logos/lilidog.txt"), Blue, Cyan, White;
    Locos => "locos", [], include_str!("../assets/logos/locos.txt"), Blue, Cyan, White;
    Ubuntu => "ubuntu", ["ubuntu linux"], include_str!("../assets/logos/ubuntu.txt"), Yellow, Red, White;
    Pop => "pop", ["popos"], include_str!("../assets/logos/pop.txt"), Yellow, Cyan, White;
    Elementary => "elementary", ["elementaryos"], include_str!("../assets/logos/elementary.txt"), Blue, Cyan, White;
    Zorin => "zorin", ["zorin os"], include_str!("../assets/logos/zorin.txt"), Blue, Cyan, White;
    Linuxmint => "linuxmint", [], include_str!("../assets/logos/linuxmint.txt"), Green, Cyan, White;
    Tuxedo => "tuxedo", ["tuxedo os"], include_str!("../assets/logos/tuxedo_os.txt"), Blue, Cyan, White;
    Vanilla => "vanilla", ["vanillaos"], include_str!("../assets/logos/vanilla.txt"), Cyan, Blue, White;
    Peppermint => "peppermint", ["peppermintos"], include_str!("../assets/logos/peppermint.txt"), Cyan, Blue, White;
    Linuxlite => "linuxlite", [], include_str!("../assets/logos/linuxlite.txt"), Blue, Cyan, White;
    Lliurex => "lliurex", [], include_str!("../assets/logos/lliurex.txt"), Cyan, Magenta, White;
    Kubuntu => "kubuntu", [], include_str!("../assets/logos/kubuntu.txt"), Blue, Cyan, White;
    Xubuntu => "xubuntu", [], include_str!("../assets/logos/xubuntu.txt"), Blue, Cyan, White;
    Lubuntu => "lubuntu", [], include_str!("../assets/logos/lubuntu.txt"), Blue, Cyan, White;
    UbuntuStudio => "ubuntu-studio", [], include_str!("../assets/logos/ubuntu_studio.txt"), Yellow, White, White;
    UbuntuCinnamon => "ubuntu-cinnamon", [], include_str!("../assets/logos/ubuntu_cinnamon.txt"), Green, Cyan, White;
    UbuntuBudgie => "ubuntu-budgie", [], include_str!("../assets/logos/ubuntu_budgie.txt"), Green, Cyan, White;
    UbuntuMate => "ubuntu-mate", [], include_str!("../assets/logos/ubuntu_mate.txt"), Green, Cyan, White;
    UbuntuUnity => "ubuntu-unity", [], include_str!("../assets/logos/ubuntu_unity.txt"), Yellow, White, White;
    UbuntuKylin => "ubuntu-kylin", [], include_str!("../assets/logos/ubuntu_kylin.txt"), Red, Yellow, White;
    UbuntuSway => "ubuntu-sway", [], include_str!("../assets/logos/ubuntu_sway.txt"), Magenta, Blue, White;
    UbuntuTouch => "ubuntu-touch", [], include_str!("../assets/logos/ubuntu_touch.txt"), Yellow, White, White;
    UbuntuGnome => "ubuntu-gnome", [], include_str!("../assets/logos/ubuntu_gnome.txt"), Yellow, White, White;
    Opensuse => "opensuse", [], include_str!("../assets/logos/opensuse.txt"), Green, Yellow, White;
    OpensuseLeap => "opensuse-leap", ["leap"], include_str!("../assets/logos/opensuse_leap.txt"), Green, Yellow, White;
    OpensuseTumbleweed => "opensuse-tumbleweed", ["tumbleweed"], include_str!("../assets/logos/opensuse_tumbleweed.txt"), Green, Yellow, White;
    OpensuseMicroos => "opensuse-microos", ["microos"], include_str!("../assets/logos/opensuse_microos.txt"), Green, Yellow, White;
    OpensuseSlowroll => "opensuse-slowroll", ["slowroll"], include_str!("../assets/logos/opensuse_slowroll.txt"), Green, Yellow, White;
    Gentoo => "gentoo", ["gentoo linux"], include_str!("../assets/logos/gentoo.txt"), Indigo, Magenta, White;
    Funtoo => "funtoo", [], include_str!("../assets/logos/funtoo.txt"), Indigo, Magenta, White;
    Calculate => "calculate", ["calculate linux"], include_str!("../assets/logos/calculate.txt"), Indigo, Magenta, White;
    Redcore => "redcore", ["redcore linux"], include_str!("../assets/logos/redcore.txt"), Indigo, Magenta, White;
    Pentoo => "pentoo", [], include_str!("../assets/logos/pentoo.txt"), Indigo, Magenta, White;
    Nixos => "nixos", ["nixos linux"], include_str!("../assets/logos/nixos.txt"), Blue, Cyan, White;
    Guix => "guix", ["gnu guix"], include_str!("../assets/logos/guix.txt"), Yellow, Red, White;
    Void => "void", ["void linux"], include_str!("../assets/logos/void.txt"), Green, Cyan, White;
    Alpine => "alpine", ["alpine linux"], include_str!("../assets/logos/alpine.txt"), Blue, Cyan, White;
    Postmarketos => "postmarketos", [], include_str!("../assets/logos/postmarketos.txt"), Blue, Cyan, White;
    Solus => "solus", [], include_str!("../assets/logos/solus.txt"), Blue, Cyan, White;
    Chakra => "chakra", ["chakra linux"], include_str!("../assets/logos/chakra.txt"), Blue, Cyan, White;
    PuroS => "pureos", [], include_str!("../assets/logos/pureos.txt"), Blue, Cyan, White;
    Openmandriva => "openmandriva", [], include_str!("../assets/logos/openmandriva.txt"), Blue, Cyan, White;
    Openeuler => "openeuler", [], include_str!("../assets/logos/openeuler.txt"), Red, Yellow, White;
    Antergos => "antergos", [], include_str!("../assets/logos/antergos.txt"), Blue, Cyan, White;
    Kaos => "kaos", [], include_str!("../assets/logos/kaos.txt"), Blue, Cyan, White;
    Altlinux => "altlinux", [], include_str!("../assets/logos/altlinux.txt"), Blue, Cyan, White;
    Rosa => "rosa", ["rosa linux"], include_str!("../assets/logos/rosa.txt"), Blue, Cyan, White;
    Lunar => "lunar", ["lunar linux"], include_str!("../assets/logos/lunar.txt"), Blue, Cyan, White;
    Sourcemage => "source-mage", [], include_str!("../assets/logos/source_mage.txt"), Blue, Cyan, White;
    Slackware => "slackware", [], include_str!("../assets/logos/slackware.txt"), Blue, Cyan, White;
    Mageia => "mageia", [], include_str!("../assets/logos/mageia.txt"), Cyan, Blue, White;
    Lfs => "lfs", [], include_str!("../assets/logos/lfs.txt"), Blue, Cyan, White;
    Puppy => "puppy", ["puppylinux"], include_str!("../assets/logos/puppy.txt"), Blue, Cyan, White;
    ClearLinux => "clear-linux-os", ["clear linux"], include_str!("../assets/logos/clear_linux.txt"), Blue, Cyan, White;
    Amazon => "amazon", ["amzn", "amazon linux", "amazon linux 2", "amazon linux 2023"], include_str!("../assets/logos/amazon.txt"), Yellow, Cyan, White;
    Openwrt => "openwrt", [], include_str!("../assets/logos/openwrt.txt"), Blue, Cyan, White;
    Opnsense => "opnsense", [], include_str!("../assets/logos/opnsense.txt"), Red, Yellow, White;
    Haiku => "haiku", ["haiku os"], include_str!("../assets/logos/haiku.txt"), Green, Cyan, White;
    Qubes => "qubes", ["qubes os"], include_str!("../assets/logos/qubes.txt"), Blue, Cyan, White;
    Openkylin => "openkylin", [], include_str!("../assets/logos/openkylin.txt"), Blue, Cyan, White;
    Kylin => "kylin", [], include_str!("../assets/logos/kylin.txt"), Red, Yellow, White;
    Uos => "uos", ["uniontech os"], include_str!("../assets/logos/uos.txt"), Blue, Cyan, White;
    AstraLinux => "astra", ["astra linux"], include_str!("../assets/logos/astra_linux.txt"), Blue, Cyan, White;
    CblMariner => "cbl-mariner", ["mariner"], include_str!("../assets/logos/cbl_mariner.txt"), Blue, Cyan, White;
    Android => "android", [], include_str!("../assets/logos/android.txt"), Green, Cyan, White;
    Harmonyos => "harmonyos", [], include_str!("../assets/logos/harmonyos.txt"), Red, Cyan, White;
    Sailfish => "sailfish", ["sailfish os"], include_str!("../assets/logos/sailfish.txt"), Blue, Cyan, White;
    Bredos => "bredos", [], include_str!("../assets/logos/bredos.txt"), Blue, Cyan, White;
    Smartos => "smartos", [], include_str!("../assets/logos/smartos.txt"), Blue, Cyan, White;
    Freebsd => "freebsd", [], include_str!("../assets/logos/freebsd.txt"), Red, Yellow, White;
    Netbsd => "netbsd", [], include_str!("../assets/logos/netbsd.txt"), Red, Yellow, White;
    Openbsd => "openbsd", [], include_str!("../assets/logos/openbsd.txt"), Red, Yellow, White;
    Dragonfly => "dragonfly", ["dragonfly bsd"], include_str!("../assets/logos/dragonfly.txt"), Red, Yellow, White;
    Ghostbsd => "ghostbsd", [], include_str!("../assets/logos/ghostbsd.txt"), Red, Yellow, White;
    Midnightbsd => "midnightbsd", [], include_str!("../assets/logos/midnightbsd.txt"), Red, Yellow, White;
    Nomadbsd => "nomadbsd", [], include_str!("../assets/logos/nomadbsd.txt"), Red, Yellow, White;
    Bsd => "bsd", [], include_str!("../assets/logos/bsd.txt"), Red, Yellow, White;
    Aix => "aix", [], include_str!("../assets/logos/aix.txt"), Green, Cyan, White;
    Exherbo => "exherbo", [], include_str!("../assets/logos/exherbo.txt"), Blue, Cyan, White;
    Mandriva => "mandriva", [], include_str!("../assets/logos/mandriva.txt"), Green, Yellow, White;
    Scientific => "scientific", ["scientific linux"], include_str!("../assets/logos/scientific.txt"), Blue, Cyan, White;
    Solaris => "solaris", ["illumos", "solaris 11"], include_str!("../assets/logos/solaris.txt"), Yellow, Cyan, White;
    Zos => "zos", [], include_str!("../assets/logos/zos.txt"), Blue, Cyan, White;
    Windows11 => "windows-11", [], include_str!("../assets/logos/windows_11.txt"), Blue, Cyan, White;
    Windows8 => "windows-8", [], include_str!("../assets/logos/windows_8.txt"), Blue, Cyan, White;
    Windows95 => "windows-95", [], include_str!("../assets/logos/windows_95.txt"), Blue, Cyan, White;
    Windows2025 => "windows-2025", [], include_str!("../assets/logos/windows_2025.txt"), Blue, Cyan, White;
    Tails => "tails", [], include_str!("../assets/logos/tails.txt"), Cyan, Blue, White;
    Armbian => "armbian", [], include_str!("../assets/logos/armbian.txt"), Yellow, Red, White;
    Trisquel => "trisquel", [], include_str!("../assets/logos/trisquel.txt"), Indigo, Magenta, White;
    Sparky => "sparky", ["sparkylinux"], include_str!("../assets/logos/sparky.txt"), Blue, Cyan, White;
    Siduction => "siduction", [], include_str!("../assets/logos/siduction.txt"), Green, Cyan, White;
    Parabola => "parabola", [], include_str!("../assets/logos/parabola.txt"), Blue, Cyan, White;
    Pardus => "pardus", [], include_str!("../assets/logos/pardus.txt"), Blue, Cyan, White;
    Pclinuxos => "pclinuxos", [], include_str!("../assets/logos/pclinuxos.txt"), Blue, Cyan, White;
    Openmamba => "openmamba", [], include_str!("../assets/logos/openmamba.txt"), Green, Cyan, White;
    Pearos => "pearos", [], include_str!("../assets/logos/pearos.txt"), Green, Cyan, White;
    Lmde => "lmde", [], include_str!("../assets/logos/lmde.txt"), Green, Cyan, White;
    Slackel => "slackel", [], include_str!("../assets/logos/slackel.txt"), Blue, Cyan, White;
    Slitaz => "slitaz", [], include_str!("../assets/logos/slitaz.txt"), Blue, Cyan, White;
    Sulin => "sulin", [], include_str!("../assets/logos/sulin.txt"), Blue, Cyan, White;
    Linspire => "linspire", [], include_str!("../assets/logos/linspire.txt"), Blue, Cyan, White;
    Bodhi => "bodhi", [], include_str!("../assets/logos/bodhi.txt"), Green, Cyan, White;
    Blankon => "blankon", [], include_str!("../assets/logos/blankon.txt"), Blue, Cyan, White;
    Crux => "crux", [], include_str!("../assets/logos/crux.txt"), Blue, Cyan, White;
    Arkane => "arkane", [], include_str!("../assets/logos/arkane.txt"), Blue, Cyan, White;
    Arselinux => "arselinux", [], include_str!("../assets/logos/arselinux.txt"), Blue, Cyan, White;
    Archstrike => "archstrike", [], include_str!("../assets/logos/archstrike.txt"), Blue, Cyan, White;
    Archbox => "archbox", [], include_str!("../assets/logos/archbox.txt"), Blue, Cyan, White;
    Aiscos => "aoscos", [], include_str!("../assets/logos/aoscos.txt"), Blue, Cyan, White;
    Astos => "astos", [], include_str!("../assets/logos/astos.txt"), Blue, Cyan, White;
    Aster => "aster", [], include_str!("../assets/logos/aster.txt"), Blue, Cyan, White;
    Blackpanther => "blackpanther", [], include_str!("../assets/logos/blackpanther.txt"), Blue, Cyan, White;
    Blag => "blag", [], include_str!("../assets/logos/blag.txt"), Blue, Cyan, White;
    Bunsenlabs => "bunsenlabs", [], include_str!("../assets/logos/bunsenlabs.txt"), Blue, Cyan, White;
    Calinixos => "calinixos", [], include_str!("../assets/logos/calinixos.txt"), Blue, Cyan, White;
    Cleanjaro => "cleanjaro", [], include_str!("../assets/logos/cleanjaro.txt"), Blue, Cyan, White;
    Condres => "condres", [], include_str!("../assets/logos/condres.txt"), Blue, Cyan, White;
    Cutefishos => "cutefishos", [], include_str!("../assets/logos/cutefishos.txt"), Blue, Cyan, White;
    Dracos => "dracos", [], include_str!("../assets/logos/dracos.txt"), Blue, Cyan, White;
    Elbrus => "elbrus", [], include_str!("../assets/logos/elbrus.txt"), Blue, Cyan, White;
    Elive => "elive", [], include_str!("../assets/logos/elive.txt"), Blue, Cyan, White;
    Evolutionos => "evolutionos", [], include_str!("../assets/logos/evolutionos.txt"), Blue, Cyan, White;
    Grombyang => "grombyang", [], include_str!("../assets/logos/grombyang.txt"), Blue, Cyan, White;
    Gxde => "gxde", [], include_str!("../assets/logos/gxde.txt"), Blue, Cyan, White;
    Hardclanz => "hardclanz", [], include_str!("../assets/logos/hardclanz.txt"), Blue, Cyan, White;
    Iglunix => "iglunix", [], include_str!("../assets/logos/iglunix.txt"), Blue, Cyan, White;
    Instantos => "instantos", [], include_str!("../assets/logos/instantos.txt"), Blue, Cyan, White;
    Januslinux => "januslinux", [], include_str!("../assets/logos/januslinux.txt"), Blue, Cyan, White;
    Kaisen => "kaisen", [], include_str!("../assets/logos/kaisen.txt"), Blue, Cyan, White;
    Kogaion => "kogaion", [], include_str!("../assets/logos/kogaion.txt"), Blue, Cyan, White;
    Massos => "massos", [], include_str!("../assets/logos/massos.txt"), Blue, Cyan, White;
    Midos => "midos", [], include_str!("../assets/logos/midos.txt"), Blue, Cyan, White;
    Obarun => "obarun", [], include_str!("../assets/logos/obarun.txt"), Blue, Cyan, White;
    Orchid => "orchid", [], include_str!("../assets/logos/orchid.txt"), Blue, Cyan, White;
    Oreon => "oreon", [], include_str!("../assets/logos/oreon.txt"), Blue, Cyan, White;
    Parsix => "parsix", [], include_str!("../assets/logos/parsix.txt"), Blue, Cyan, White;
    Radix => "radix", [], include_str!("../assets/logos/radix.txt"), Blue, Cyan, White;
    Ravynos => "ravynos", [], include_str!("../assets/logos/ravynos.txt"), Blue, Cyan, White;
    Regata => "regata", [], include_str!("../assets/logos/regata.txt"), Blue, Cyan, White;
    Secureblue => "secureblue", [], include_str!("../assets/logos/secureblue.txt"), Blue, Cyan, White;
    Xinux => "xinux", [], include_str!("../assets/logos/xinux.txt"), Blue, Cyan, White;
    Sabayon => "sabayon", [], include_str!("../assets/logos/sabayon.txt"), Blue, Cyan, White;
    Korora => "korora", [], include_str!("../assets/logos/korora.txt"), Blue, Cyan, White;
    Netrunner => "netrunner", [], include_str!("../assets/logos/netrunner.txt"), Blue, Cyan, White;
    Openindiana => "openindiana", [], include_str!("../assets/logos/openindiana.txt"), Blue, Cyan, White;
    Omnios => "omnios", [], include_str!("../assets/logos/omnios.txt"), Blue, Cyan, White;
    Porteus => "porteus", [], include_str!("../assets/logos/porteus.txt"), Blue, Cyan, White;
    Serpent => "serpent", ["serpentos"], include_str!("../assets/logos/serpent_os.txt"), Blue, Cyan, White;
    Skiffos => "skiffos", [], include_str!("../assets/logos/skiffos.txt"), Blue, Cyan, White;
    Swagarch => "swagarch", [], include_str!("../assets/logos/swagarch.txt"), Blue, Cyan, White;
    Torizon => "torizon", ["torizonos"], include_str!("../assets/logos/torizoncore.txt"), Blue, Cyan, White;
    Uwuntu => "uwuntu", [], include_str!("../assets/logos/uwuntu.txt"), Blue, Cyan, White;
    Valhalla => "valhalla", [], include_str!("../assets/logos/valhalla.txt"), Blue, Cyan, White;
    Venom => "venom", [], include_str!("../assets/logos/venom.txt"), Blue, Cyan, White;
    Vnux => "vnux", [], include_str!("../assets/logos/vnux.txt"), Blue, Cyan, White;
    Wolfos => "wolfos", [], include_str!("../assets/logos/wolfos.txt"), Blue, Cyan, White;
    Macaronos => "macaronos", [], include_str!("../assets/logos/macaronios.txt"), Blue, Cyan, White;
    Maui => "maui", [], include_str!("../assets/logos/maui.txt"), Blue, Cyan, White;
    Nuros => "nuros", [], include_str!("../assets/logos/nuros.txt"), Blue, Cyan, White;
    Nurunner => "nurunner", [], include_str!("../assets/logos/nurunner.txt"), Blue, Cyan, White;
    Laxeros => "laxeros", [], include_str!("../assets/logos/laxeros.txt"), Blue, Cyan, White;
    Minix => "minix", [], include_str!("../assets/logos/minix.txt"), Blue, Cyan, White;
    Msys2 => "msys2", [], include_str!("../assets/logos/msys2.txt"), Blue, Cyan, White;
    Nitrux => "nitrux", [], include_str!("../assets/logos/nitrux.txt"), Blue, Cyan, White;
    Osmc => "osmc", [], include_str!("../assets/logos/osmc.txt"), Blue, Cyan, White;
    Redos => "redos", [], include_str!("../assets/logos/redos.txt"), Blue, Cyan, White;
    Salix => "salix", [], include_str!("../assets/logos/salix.txt"), Blue, Cyan, White;
    Bitrig => "bitrig", [], include_str!("../assets/logos/bitrig.txt"), Blue, Cyan, White;
    Endless => "endless", ["endless os"], include_str!("../assets/logos/endless.txt"), Blue, Cyan, White;
    Refracta => "refracta", [], include_str!("../assets/logos/refracta.txt"), Blue, Cyan, White;
    Frugalware => "frugalware", [], include_str!("../assets/logos/frugalware.txt"), Blue, Cyan, White;
    Snigdhaos => "snigdhaos", [], include_str!("../assets/logos/snigdhaos.txt"), Blue, Cyan, White;
    Univention => "univention", [], include_str!("../assets/logos/univention.txt"), Blue, Cyan, White;
    Openstage => "openstage", [], include_str!("../assets/logos/openstage.txt"), Blue, Cyan, White;
    Galliumos => "galliumos", [], include_str!("../assets/logos/galliumos.txt"), Blue, Cyan, White;
    Grapheneos => "grapheneos", [], include_str!("../assets/logos/grapheneos.txt"), Blue, Cyan, White;
    Libreelec => "libreelec", [], include_str!("../assets/logos/libreelec.txt"), Blue, Cyan, White;
    Pcbsd => "pcbsd", [], include_str!("../assets/logos/pcbsd.txt"), Blue, Cyan, White;
}

/// Resolve the logo to use.
///
/// An explicit request is returned unchanged. `Auto` matches the running
/// distribution against the catalogue, trying the most specific os-release
/// field first.
pub fn resolve(requested: LogoPreset, os_info: Option<&OsInfo>) -> LogoPreset {
    if !matches!(requested, LogoPreset::Auto) {
        return requested;
    }

    if let Some(os_info) = os_info {
        let mut candidates = Vec::new();
        if let Some(variant) = &os_info.variant {
            candidates.push(variant.as_str());
        }
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

/// The definition of a logo.
pub fn definition(preset: LogoPreset) -> LogoDefinition {
    match preset {
        LogoPreset::Auto => definition(platform_fallback()),
        other => definitions()
            .iter()
            .copied()
            .find(|definition| definition.preset == other)
            .unwrap_or_else(|| definition(platform_fallback())),
    }
}

/// The colour to use for one line of a logo.
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
    use std::collections::{HashMap, HashSet};
    use std::str::FromStr;

    use super::{LogoPreset, definition, definitions, resolve, strip_fastfetch_markers};
    use crate::providers::system::OsInfo;

    fn os_info(id: &str, name: &str, id_like: &[&str], variant: Option<&str>) -> OsInfo {
        OsInfo {
            name: name.to_string(),
            id: Some(id.to_string()),
            id_like: id_like.iter().map(|value| value.to_string()).collect(),
            variant: variant.map(|value| value.to_string()),
        }
    }

    #[test]
    fn resolves_explicit_logo_without_detection() {
        let os = os_info("ubuntu", "Ubuntu", &["debian"], None);

        assert_eq!(resolve(LogoPreset::Arch, Some(&os)), LogoPreset::Arch);
    }

    #[test]
    fn resolves_logo_from_os_id() {
        let os = os_info("arch", "Arch Linux", &[], None);

        assert_eq!(resolve(LogoPreset::Auto, Some(&os)), LogoPreset::Arch);
    }

    #[test]
    fn resolves_logo_from_os_family() {
        let os = os_info("linuxmint", "Linux Mint", &["ubuntu", "debian"], None);

        assert_eq!(resolve(LogoPreset::Auto, Some(&os)), LogoPreset::Linuxmint);
    }

    #[test]
    fn resolves_logo_from_name_alias() {
        let os = os_info("raspbian", "Raspberry Pi OS", &["debian"], None);

        assert_eq!(resolve(LogoPreset::Auto, Some(&os)), LogoPreset::Raspbian);
    }

    #[test]
    fn resolves_arch_derivative_logo() {
        let os = os_info("arcolinux", "ArcoLinux", &["arch"], None);

        assert_eq!(resolve(LogoPreset::Auto, Some(&os)), LogoPreset::Arcolinux);
    }

    #[test]
    fn resolves_fedora_derivative_logo() {
        let os = os_info("bazzite", "Bazzite", &["fedora"], None);

        assert_eq!(resolve(LogoPreset::Auto, Some(&os)), LogoPreset::Fedora);
    }

    #[test]
    fn falls_back_to_platform_logo() {
        let os = os_info("mystery", "Unknown Distribution", &["something"], None);

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

    #[test]
    fn resolves_pop_os_to_its_own_logo() {
        let os = os_info("pop", "Pop!_os", &["ubuntu", "debian"], None);

        assert_eq!(resolve(LogoPreset::Auto, Some(&os)), LogoPreset::Pop);
    }

    #[test]
    fn resolves_distro_specific_logos() {
        let cases = [
            ("rocky", "Rocky Linux", LogoPreset::Rocky),
            ("almalinux", "AlmaLinux", LogoPreset::Almalinux),
            ("centos", "CentOS Linux", LogoPreset::Centos),
            ("kali", "Kali GNU/Linux", LogoPreset::Kali),
            ("elementary", "elementary OS", LogoPreset::Elementary),
            ("zorin", "Zorin OS", LogoPreset::Zorin),
            ("devuan", "Devuan", LogoPreset::Devuan),
            (
                "proxmox",
                "Proxmox Virtual Environment",
                LogoPreset::Proxmox,
            ),
            ("opensuse-leap", "openSUSE Leap", LogoPreset::OpensuseLeap),
            ("calculate", "Calculate Linux", LogoPreset::Calculate),
            ("arcolinux", "ArcoLinux", LogoPreset::Arcolinux),
        ];

        for (id, name, expected) in cases {
            let os = os_info(id, name, &[], None);
            assert_eq!(resolve(LogoPreset::Auto, Some(&os)), expected, "{id}");
        }
    }

    #[test]
    fn resolves_ubuntu_flavour_from_the_variant() {
        let os = os_info("ubuntu", "Ubuntu 24.04.1 LTS", &["debian"], Some("Kubuntu"));

        assert_eq!(resolve(LogoPreset::Auto, Some(&os)), LogoPreset::Kubuntu);
    }

    #[test]
    fn resolves_fedora_variant_from_the_variant() {
        let os = os_info(
            "fedora",
            "Fedora Silverblue",
            &["fedora"],
            Some("Silverblue"),
        );

        assert_eq!(resolve(LogoPreset::Auto, Some(&os)), LogoPreset::Silverblue);
    }

    #[test]
    fn every_preset_has_artwork() {
        for preset in LogoPreset::all() {
            if *preset == LogoPreset::None {
                continue;
            }
            assert!(
                !definition(*preset).lines().is_empty(),
                "{preset} has no artwork"
            );
        }
    }

    #[test]
    fn every_preset_has_a_definition() {
        for preset in LogoPreset::all() {
            if *preset == LogoPreset::Auto {
                continue;
            }
            let found = definitions()
                .iter()
                .find(|definition| definition.preset == *preset);
            assert!(found.is_some(), "missing definition for {preset}");
        }
    }

    #[test]
    fn every_preset_round_trips_through_its_name() {
        for preset in LogoPreset::all() {
            let parsed = LogoPreset::from_str(&preset.to_string())
                .unwrap_or_else(|error| panic!("{preset} did not parse: {error}"));
            assert_eq!(parsed, *preset);
        }
    }

    #[test]
    fn auto_is_not_part_of_the_catalogue() {
        assert!(
            definitions()
                .iter()
                .all(|definition| definition.preset != LogoPreset::Auto)
        );
    }

    #[test]
    fn aliases_are_unique_across_the_catalogue() {
        let mut seen: HashMap<String, &str> = HashMap::new();
        for definition in definitions() {
            let mut own: HashSet<String> = HashSet::new();
            for alias in definition.aliases {
                let normalized: String = alias
                    .chars()
                    .filter(|character| character.is_ascii_alphanumeric())
                    .flat_map(char::to_lowercase)
                    .collect();
                assert!(
                    own.insert(normalized.clone()),
                    "{} lists the alias {alias} twice",
                    definition.preset
                );
                if let Some(other) = seen.insert(normalized.clone(), definition.preset.name()) {
                    assert_eq!(
                        other,
                        definition.preset.name(),
                        "the alias {alias} is claimed by {other} and by {}",
                        definition.preset
                    );
                }
            }
        }
    }
}
