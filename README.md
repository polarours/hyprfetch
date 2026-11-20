# hyprfetch

`hyprfetch` is a Hyprland-native fetch utility for people who care about live session state, desktop rice, customization, and terminal presentation.

Unlike generic fetch tools, it treats Hyprland runtime data as first-class output. The MVP combines classic system facts with current Hyprland state, GTK theme detection, wallpaper daemon detection, and a terminal-friendly renderer that degrades cleanly when parts of the environment are unavailable.

## Current MVP

- Basic system modules: OS, host, kernel, uptime, packages, shell, terminal, CPU (with realtime usage), GPU (with VRAM and usage), memory, swap, disk (with several mount points), battery, locale, network (with WiFi SSID/signal), audio (with the volume), media, temperature, datetime
- Hyprland modules: version, health summary (sockets, wallpaper, bar, notifications, idle, lock, portal, polkit agent), runtime instance summary, current splash, active workspace, focused monitor and session summary, workspace history (with monitor, window counts and fullscreen state per row), workspace list with active/special/fullscreen state, active window state, cursor position, monitor summary, config health, workspace rules, input summary (with per-device pointer speed and scroll factors), keybind summary, global shortcut summary, the layout of each workspace, layer-shell surface summary, hypr-rice (layout, gaps, border, rounding, blur, animations)
- Rice modules: GTK theme, icon theme, cursor theme, font, wallpaper backend
- Ecosystem detection: wallpaper, bar, launcher, notification, clipboard, screenshot, idle, lock, portal, polkit, tray, colour picker, network (NetworkManager/iwd/ConnMan/systemd-networkd), audio (PipeWire/PulseAudio), bluetooth, greeter (SDDM/GDM/LightDM/ly/greetd), IME (fcitx5/ibus), night light (gammastep/wlsunset/hyprsunset/redshift), power (power-profiles/tuned) and automount (udisks) daemons (with version info when the process can be probed)
- Output modes: styled split layout, plain mode, minimal mode, JSON mode, diagnostics mode, configurable module order, width-aware layout fallback for narrow terminals, share preset that leads with the theme and the live session state, an animated line-by-line reveal on by default, `--watch` continuous refresh mode
- Customization: TOML-driven module order, per-module options, user-defined display labels, and command plugins
- Palettes: 17 built-in themes — hypr, mono, catppuccin, dracula, gruvbox, gruvbox-light, nord, one-dark, solarized-dark, solarized-light, rose-pine, ayu, everforest, kanagawa, monokai, material, github (list with `--list-themes`)
- Logo system: bundled distro-aware text logos stored in `assets/logos/`, with `os-release` auto detection, family fallback, and explicit override support (429 built-in presets, including numbered and retro variants of the artwork selectable by name)
- Package managers: pacman, dpkg, rpm, nix, homebrew
- Config: `~/.config/hyprfetch/config.toml` or `$XDG_CONFIG_HOME/hyprfetch/config.toml`

## Why It Exists

- Hyprland users often want current session context, not just hardware specs.
- Rice screenshots benefit from fetch output that understands the compositor and desktop stack.
- A focused tool can stay fast and maintainable instead of becoming a generic “detect everything” binary.

## Build

```bash
cargo build --release
```

## Install Locally

```bash
cargo install --path . --locked
```

Or install the built binary manually:

```bash
install -Dm755 target/release/hyprfetch ~/.local/bin/hyprfetch
```

## Quick Start

Default output:

```bash
hyprfetch
```

On the machine this README was written on, that prints (colours left out here —
run it in a terminal to see the theme, the usage bars and the logo in colour):

```text
                  ··                    polarours@polarours-archlinux
                 ····                   -----------------------------
                ····/                  OS:  Arch Linux
               ·······                 Host:  83DG (Legion Y7000P IRX9)
              ·········                Kernel:  7.3.4-arch1-2
              ··········               Uptime:  9m
            ·/···········              Packages:  2318 (pacman)
           ·/····/········             Shell:  zsh
          ·/···············            Terminal:  kitty
         ·/················/·          CPU:  Intel(R) Core(TM) i7-14700HX  26.4%… ###.......
        ·/····················         GPU:  NVIDIA GeForce RTX 4060 Max-Q /… ####......
       ··············/·········        Memory:  8.2 GiB / 15 GiB (53%) #####.....
      ··········      ··········       Swap:  2.3 GiB / 7.7 GiB (30%) ###.......
     ·········/        ··········      Disk (/):  836 GiB / 953 GiB (88%) - btrfs
    /·········/        ········/·      Battery (L23M4PK5):  100% [Full]
  ·/········/··        ··/·········    Locale:  en_US.UTF-8
 ·········                 ···/·····   Network:  172.20.10.2 (wlan0), SSID: howie, Signal: 95%, 1…
·····                           ··/·/  Temp:  76°C (CPU)
··                                 ·/  Audio:  PipeWire (Built-in Audio Analog Stereo), 35%
                                       DateTime:  2026-11-20 15:12:17 (HKT)
                                       Colors:  WhiteSur-Dark
                                       Hyprland:  v0.57.2
                                       Workspace:  2
                                       Clients:  2
                                       Focused:  eDP-1 (ws:2)
                                       Session:  1 outputs, 2 ws, 2 wins
                                       Health:  idle missing, lock missing
                                       Instances:  1 instance, current:11cbc6c…, pid:10728,…
                                       Splash:  "Hello everyone this is YOUR daily dose of ‘read…
                                       Window:  Hermes [com.nousresearch.hermes, ws:2]
                                       Cursor:  1262, 940
                                       History:  1:Bistutu/GoMusic: 迁移网易云/汽… (eDP-1, 1…
                                       Workspaces:  1:1 *2:1
                                       Monitors:  1 (*eDP-1:2 2560x1600@165Hz x1.25)
                                       Config:  ok
                                       Rules:  0 rules
                                       Input:  9 keyboards, 4 mice, 1 switch, keymap:English (US)
                                       Binds:  55 binds, 2 mouse, 6 repeat, top:HL.Dispatcher(ex…
                                       Shortcuts:  6 shortcuts, apps:com.nousresearch.hermes/quicksh…
                                       Layouts:  dwindle
                                       Layers:  5 surfaces, 1 output, ns:awww-daemon/quickshell/u…
                                       GTK:  WhiteSur-Dark
                                       Icons:  breeze
                                       Cursor:  breeze_cursors
                                       Font:  Noto Sans, 10
                                       Wallpaper:  awww
                                       Ecosystem:  bar:quickshell launch:walker notify:swaync…
```

This uses `os-release` to choose a built-in distro logo automatically. If the distro is not recognized, `hyprfetch` falls back to a generic Linux logo.
Styled output uses `key: value` rows, a session title, and optional terminal color bars similar to classic fetch screenshots. CPU, GPU, memory and swap values include a compact colored usage bar (green → gold → red by load).
On narrow terminals, `hyprfetch` first switches to a narrower drawing of the logo, and only drops from the logo split layout to denser text layouts when even that does not fit, instead of relying on hard wrapping.
On very narrow terminals, it also prioritizes core system and Hyprland session modules before auxiliary modules like wallpaper or device summaries.

Plain output:

```bash
hyprfetch --plain
```

Share preset (optimized for rice screenshots):

```bash
hyprfetch --share
```

Minimal style (key:value only, no header or color bars):

```bash
hyprfetch --style minimal
```

Continuous refresh (redraws in place when attached to a terminal):

```bash
hyprfetch --watch --watch-interval 1
```

Create a starter config:

```bash
hyprfetch --init-config
```

Preview the default config without writing a file:

```bash
hyprfetch --print-config
```

Choose modules explicitly:

```bash
hyprfetch --modules os,kernel,uptime,hyprland,hypr-health,hypr-instances,hypr-splash,hypr-active-window,hypr-cursor,hypr-workspaces,hypr-monitors,hypr-config,hypr-rules,hypr-binds,hypr-shortcuts,hypr-layouts,hypr-layers,theme,wallpaper,hypr-ecosystem
```

Pick the logo, the palette and the colors yourself:

```bash
hyprfetch --logo minimal --theme catppuccin --color never
```

17 themes ship in the binary, inspired by the palettes collected in projects like
[kitty-themes](https://github.com/dexpota/kitty-themes). Pick one by name:

```bash
hyprfetch --theme gruvbox
hyprfetch --list-themes          # show every available name
```

Each theme sets the key colors, title, rule, logo tones and the color bars. Values are drawn in a bright near-white tone derived from the theme's foreground (dark themes brighten, light themes darken) so they stay readable next to the colored keys. The same name works in the config as `palette = "gruvbox"`.

The spinner and the reveal are on by default, and the animation is a personal choice — pick one with `--animation <style>` on the command line or `animation = "..."` in the config file:

| style | what you see |
|---|---|
| `split` | (default) the dot-outline logo skeleton grows first, then the session rows light up behind it |
| `wave` | the logo cells light up as a wave sweeping from the left edge, then the rows appear |
| `lines` | one line after another, a plain printing-press feel |
| `static` | no animation at all, and no spinner either |

Whatever the style, the whole story lands inside roughly half a second (the rhythm adapts to the output length), it only runs on a real terminal, and the finished frame is byte-identical to the static output so screenshots are unaffected. Piped output, `--json`, `--plain` and `--diagnostics` never animate, and `--watch` animates only its first frame.

List what is available:

```bash
hyprfetch --list-modules
hyprfetch --list-logos
hyprfetch --list-themes
```

Ignore the config file for a single run:

```bash
hyprfetch --no-config
```

Machine-readable output:

```bash
hyprfetch --json
```

Diagnose missing or partial modules:

```bash
hyprfetch --diagnostics
```

Show built-in logo presets:

```bash
hyprfetch --list-logos
```

Force a specific logo:

```bash
hyprfetch --logo arch
hyprfetch --logo hypr
hyprfetch --logo windows
hyprfetch --logo macos
```

## Example Config

`~/.config/hyprfetch/config.toml`

```toml
version = 1

[appearance]
style = "split"
palette = "hypr"
color = "auto"
plain = false
color_bars = true
padding = 1
separator = "  "

[logo]
preset = "auto"

[[modules]]
name = "os"
label = "Distro"

[[modules]]
name = "host"

[[modules]]
name = "kernel"

[[modules]]
name = "uptime"

[[modules]]
name = "packages"

[[modules]]
name = "shell"

[[modules]]
name = "terminal"

[[modules]]
name = "cpu"

[[modules]]
name = "gpu"

[[modules]]
name = "memory"

[[modules]]
name = "swap"

[[modules]]
name = "disk"
mounts = ["/", "/home"]

[[modules]]
name = "battery"

[[modules]]
name = "locale"

[[modules]]
name = "network"

[[modules]]
name = "temperature"

[[modules]]
name = "audio"

[[modules]]
name = "datetime"

[[modules]]
name = "hyprland"
show_runtime = false
labels = { Workspace = "WS", Clients = "Windows", Focused = "Output" }

[[modules]]
name = "hypr-health"

[[modules]]
name = "hypr-instances"
compact = true

[[modules]]
name = "hypr-splash"

[[modules]]
name = "hypr-active-window"
compact = true

[[modules]]
name = "hypr-cursor"

[[modules]]
name = "hypr-workspaces"
compact = true

[[modules]]
name = "hypr-monitors"
compact = true

[[modules]]
name = "hypr-config"
compact = true

[[modules]]
name = "hypr-rules"
compact = true

[[modules]]
name = "hypr-devices"
compact = true

[[modules]]
name = "hypr-binds"
compact = true

[[modules]]
name = "hypr-shortcuts"
compact = true

[[modules]]
name = "hypr-layouts"
compact = true

[[modules]]
name = "hypr-layers"
compact = true

[[modules]]
name = "hypr-rice"
compact = true

[[modules]]
name = "theme"
compact = false
show_font = true

[[modules]]
name = "wallpaper"

[[modules]]
name = "hypr-ecosystem"
compact = true
# versions = true   # probe and show daemon versions (off by default: skips ~100ms of --version commands)

[[plugins]]
name = "battery"
command = "sh"
args = ["-c", "printf 'Battery\t88%%\n'"]
timeout_ms = 1000
```

Module options are set next to the module they belong to. The disk module reports the
root file system by default; list the mount points you care about to report more of them:

```toml
[[modules]]
name = "disk"
mounts = ["/", "/home", "/boot"]
```

Selecting modules on the command line narrows the output but keeps the options
configured for them, so `hyprfetch --modules disk` still reports the mount points from
the config file.

The Hyprland modules can list their details instead of only counting them:

```toml
[[modules]]
name = "hypr-devices"
devices = true

[[modules]]
name = "hypr-binds"
list = true

[[modules]]
name = "hypr-layers"
list = true

[[modules]]
name = "hypr-rules"
list = true

[[modules]]
name = "hypr-shortcuts"
list = true

[[modules]]
name = "hypr-layouts"
list = true
```

The listed rows name the window, the binding, the surface, the rule, the
shortcut or the workspace they belong to, so a rice screenshot shows the setup
instead of a count of it.

## Command Plugins

Command plugins are the first step toward a richer plugin system. They let users add custom rows without recompiling `hyprfetch`.

Each plugin runs a command and reads stdout:

- A single plain line becomes one row using `label` or `name`.
- `Key<TAB>Value` lines become explicit rows.
- `Key=Value` lines also become explicit rows.

Example:

```toml
[[plugins]]
name = "music"
command = "sh"
args = ["-c", "printf 'Track\t%s\n' \"Lo-fi radio\""]
timeout_ms = 1000
```

A plugin that fails or runs into its timeout keeps its reason in the output, so a
broken plugin is never mistaken for a plugin that produced no rows. The modules that
do not apply to the machine stay out of the output instead.

## Module IDs

- `os`
- `host`
- `kernel`
- `uptime`
- `packages`
- `shell`
- `terminal`
- `cpu`
- `gpu`
- `memory`
- `swap`
- `disk`
- `battery`
- `locale`
- `network`
- `temperature`
- `audio`
- `media`
- `datetime`
- `colorscheme`
- `hyprland`
- `hypr-health`
- `hypr-instances`
- `hypr-splash`
- `hypr-workspaces`
- `hypr-monitors`
- `hypr-active-window`
- `hypr-cursor`
- `hypr-workspace-history`
- `hypr-rice` — available but not part of the default module order: it costs several extra `hyprctl getoption` calls, so add it to the config when you want the rice rows
- `hypr-config`
- `hypr-rules`
- `hypr-devices`
- `hypr-binds`
- `hypr-shortcuts`
- `hypr-layouts`
- `hypr-layers`
- `hypr-ecosystem`
- `theme`
- `wallpaper`

## Data Sources

- `/etc/os-release`
- `/proc`
- `/sys`
- `/proc/stat` for realtime CPU utilisation, sampled across the collection window
- `nvidia-smi` for NVIDIA GPU utilisation, `/sys/class/drm/*/device/gpu_busy_percent` and `gt_perf_status` for AMD and Intel
- `/var/lib/pacman/local`
- `/var/lib/dpkg/status`
- `/sys/class/power_supply`
- `/proc/net/route`, `ip` command for network detection, and `iw` or `iwconfig` for the SSID, signal and link speed of a wireless interface
- PipeWire/PulseAudio runtime sockets and the `wpctl` and `pactl` commands for audio detection
- the `playerctl` command for the media players, which speak MPRIS over the session bus
- environment variables, including Hyprland runtime variables and socket paths
- Hyprland runtime instance directories under `$XDG_RUNTIME_DIR/hypr`
- GTK settings files under `$XDG_CONFIG_HOME`
- `hyprctl`, including `splash` for the current Hyprland splash and `-j` endpoints such as `cursorpos` for pointer location, `configerrors` for config health, `workspacerules` for workspace rule summaries, `binds` for keybind summaries, `globalshortcuts` for global shortcut summaries, `layers` for layer-shell surfaces, and `getoption` for the rice and workspace history modules
- the `date` command for the current date and time
- `rpm -qa` for RPM-based package counting
- `/nix/store` for Nix package counting
- `/opt/homebrew/Cellar` or `/usr/local/Cellar` for Homebrew package counting

The binary stays usable outside Hyprland. Hyprland-specific modules degrade to unavailable instead of aborting the run.

## Built-in Logos

`hyprfetch` ships with a bundled logo catalogue in `assets/logos/`. It covers the major
families and their well known derivatives, so a machine running a niche distro lands on
its own artwork instead of a generic fallback:

- **Arch family**: `arch`, `arcolinux`, `archcraft`, `archlabs`, `archbox`, `archstrike`,
  `arkane`, `arselinux`, `blackarch`, `hyperbola`, `steamos`, `steamdeck`, `chimera`,
  `biglinux`, `manjaro`, `garuda`, `cachyos`, `endeavouros`, `artix`, `asahi`, `crux`,
  `aoscos`, `astos`, `aster`, `blackpanther`, `blag`, `bunsenlabs`, `calinixos`,
  `cleanjaro`, `condres`, `cutefishos`, `dracos`, `elbrus`, `elive`, `evolutionos`,
  `grombyang`, `gxde`, `hardclanz`, `iglunix`, `instantos`, `januslinux`, `kaisen`,
  `kogaion`, `massos`, `midos`, `obarun`, `orchid`, `oreon`, `parsix`, `radix`, `ravynos`,
  `regata`, `secureblue`, `xinux`
- **Fedora and RHEL family**: `fedora`, `rocky`, `almalinux`, `centos`, `oracle`,
  `silverblue`, `kinoite`, `sericea`, `coreos`, `nobara`
- **Debian family**: `debian`, `devuan`, `kali`, `parrot`, `raspbian`, `mx`, `antix`,
  `dietpi`, `deepin`, `q4os`, `proxmox`, `truenas`, `emmabuntus`, `lilidog`, `locos`,
  `tails`, `trisquel`, `sparky`, `siduction`, `parabola`, `pardus`, `pclinuxos`,
  `openmamba`, `pearos`, `lmde`, `slackel`, `slitaz`, `sulin`, `linspire`, `bodhi`,
  `blankon`
- **Ubuntu family**: `ubuntu`, `pop`, `elementary`, `zorin`, `linuxmint`, `tuxedo`,
  `vanilla`, `peppermint`, `linuxlite`, `lliurex`, plus the `kubuntu`, `xubuntu`,
  `lubuntu`, `ubuntu-studio`, `ubuntu-cinnamon`, `ubuntu-budgie`, `ubuntu-mate`,
  `ubuntu-unity`, `ubuntu-kylin`, `ubuntu-sway`, `ubuntu-touch` and `ubuntu-gnome` flavours
- **SUSE family**: `opensuse`, `opensuse-leap`, `opensuse-tumbleweed`,
  `opensuse-microos`, `opensuse-slowroll`
- **Gentoo family**: `gentoo`, `funtoo`, `calculate`, `redcore`, `pentoo`
- **Other Linux**: `nixos`, `guix`, `void`, `alpine`, `postmarketos`, `solus`, `chakra`,
  `pureos`, `openmandriva`, `openeuler`, `antergos`, `kaos`, `altlinux`, `rosa`, `lunar`,
  `source-mage`, `slackware`, `mageia`, `mandriva`, `exherbo`, `scientific`, `lfs`,
  `puppy`, `armbian`, `sabayon`, `korora`, `netrunner`, `porteus`, `skiffos`, `swagarch`,
  `uwuntu`, `valhalla`, `venom`, `vnux`, `wolfos`, `macaronos`, `maui`, `nuros`,
  `nurunner`, `laxeros`, `minix`, `msys2`, `nitrux`, `osmc`, `redos`, `salix`,
  `bitrig`, `endless`, `refracta`, `frugalware`, `snigdhaos`, `univention`, `openstage`
- **Enterprise and appliance**: `clear-linux-os`, `amazon`, `openwrt`, `opnsense`,
  `haiku`, `qubes`, `openkylin`, `kylin`, `uos`, `astra`, `cbl-mariner`, `android`,
  `harmonyos`, `sailfish`, `bredos`, `smartos`, `openindiana`, `omnios`, `torizon`,
  `serpent`, `galliumos`, `grapheneos`, `libreelec`, `pcbsd`
- **BSD and Unix**: `freebsd`, `netbsd`, `openbsd`, `dragonfly`, `ghostbsd`,
  `midnightbsd`, `nomadbsd`, `bsd`, `solaris`, `zos`, `aix`
- **Niche and community distros**: 170 more presets, from `adelie`, `anarchy`,
  `athenaos`, `cereus`, `crystal`, `cosmic`, `hamonikr`, `parch` and `pikaos` to
  `redstar`, `t2`, `templeos` and `yiffos`, plus alternate platforms like `amiga`,
  `irix`, `os2warp`, `pacbsd`, `pengwin` and `wii_linux`. Alternate drawings of
  covered distros — numbered takes (`arch2`, `artix2`, `macos2`, `void2`, ...), older
  artwork (`fedora_old`, `linuxmint_old`, `ubuntu_old`, ...) and retro variants — are
  wired as name-only presets: they claim no os-release alias, so auto-detection never
  picks them; a user selects one explicitly with `--logo arch2`. The `minimal` preset
  renders the bundled wordmark artwork.
- **Platform and generic**: `hypr`, `linux`, `windows`, `windows-95`, `windows-8`,
  `windows-11`, `windows-2025`, `macos`

Two extra presets are also available: `minimal`, which renders the `hyprfetch` wordmark instead of distro art, and `none`, which hides the logo entirely. Run `hyprfetch --list-logos` for the authoritative list of all 429 presets plus `auto`.

ASCII logos are rendered as a minimal `·` dot-outline skeleton: the source symbols (fastfetch's `o/+/s`, figlet letters, etc.) are replaced with dots while structural strokes keep each distro's silhouette; the selected theme supplies the color. Logos already drawn with Unicode block or braille characters keep their own art.

Auto detection follows the same basic idea that makes `fastfetch` strong here: check `VARIANT`, then `ID`, then `NAME`, then `ID_LIKE`, and only then fall back to the platform family logo. That means unsupported derivatives still land on a sensible family logo in many cases instead of dropping straight to a generic fallback.

## Development

```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## Packaging

The release profile enables thin LTO and symbol stripping, so `cargo build --release --locked` produces a compact single binary.

A man page lives at `contrib/hyprfetch.1` and ships with the releases.

An Arch packaging template lives at `contrib/PKGBUILD`:

```bash
cd contrib
makepkg -si
```

Before submitting to the AUR, confirm the GitHub repository URL in `Cargo.toml` and `contrib/PKGBUILD`, then tag releases as `v0.1.0`, `v0.2.0`, and so on.

## Roadmap

The two open items are done: every bundled artwork file is wired into the logo
catalogue, the ecosystem covers twenty daemon categories with versions, and the
workspace history and input modules report their full detail. New ideas are
tracked in the changelog's Unreleased section.

The logo catalogue is complete: every usable artwork file in `assets/logos/` is
wired into a preset (429 total), and the alternate drawings are selectable by
name. Only a handful of files stay unwired on purpose: artwork whose stem
(`rhel`, `aurora`, `rebornos`, `amazon_linux`, `ultramarine`, `unknown`) is
already an os-release alias of a covered distro, the fastfetch logo, and two
orphaned `_small` files whose parent drawing does not exist.

## License

hyprfetch is released under the [GPL-3.0](LICENSE) license.
