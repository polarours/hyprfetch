# Changelog

All notable changes to hyprfetch will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.1] - 2026-11-19

The initial public release of hyprfetch, a Hyprland-native fetch utility.

### Added

- 40 built-in modules covering the classic system rows — OS, host, kernel, uptime, packages,
  shell, terminal, CPU with live usage, GPU with VRAM and utilization, memory, swap, disk with
  several mount points, battery, locale, network with WiFi SSID and signal, temperature, audio
  with the default sink's volume and mute state, media through MPRIS, datetime and colorscheme —
  plus the Hyprland session rows fastfetch cannot show: compositor version, health summary,
  runtime instances, splash, active workspace, workspace history with monitor and window
  counts, the workspace list, monitors, active window, cursor position, config health,
  workspace rules, input devices, keybinds, global shortcuts, per-workspace layouts,
  layer-shell surfaces, hypr-rice (layout, gaps, border, rounding, blur, animations), GTK/Qt
  theme and wallpaper backend.
- Ecosystem detection across twenty daemon categories, each with its version when the process
  can be probed: wallpaper, bar, launcher, notifications, clipboard, screenshot, idle, lock,
  portal, polkit, tray, colour picker, network (NetworkManager/iwd/ConnMan/systemd-networkd),
  audio (PipeWire/PulseAudio), bluetooth, greeter (SDDM/GDM/LightDM/ly/greetd), IME
  (fcitx5/ibus), night light (gammastep/wlsunset/hyprsunset/redshift), power
  (power-profiles/tuned) and automount (udisks). The compact row stays short with a `+N`
  remainder, a fixed name width and an opt-in `versions = true`.
- A logo catalogue of 395 presets drawn from the bundled artwork and rendered in a minimal
  dot-outline house style with a brand-matched tone gradient: `os-release` auto detection with
  family fallback, `--logo` override, and coverage of the Arch, Fedora, Debian, Ubuntu, SUSE and
  Gentoo families, their derivatives, the BSDs, niche Linux distros and alternate platforms.
- 17 built-in color themes — hypr, mono, catppuccin, dracula, gruvbox, gruvbox-light, nord,
  one-dark, solarized-dark, solarized-light, rose-pine, ayu, everforest, kanagawa, monokai,
  material, github — selectable with `--theme` or `palette` in the config; values are drawn in
  a bright near-white tone derived per theme so they stay readable next to the colored keys.
- Usage bars (green → gold → red by load) on the CPU, GPU, memory and swap rows, with the
  value width budgeted so long lines never overflow the terminal.
- An animated reveal on every run, and a personal choice of story: `--animation split` (the
  dot-outline logo skeleton grows first, then the session rows light up behind it), `wave` (the
  logo cells light up column by column from the left edge), `lines` (one row after another) or
  `static` for no motion at all — also settable as `animation = "..."` in the config file. The
  reveal lands inside roughly half a second whatever the output length, only runs on a real
  terminal, and the finished frame is byte-identical to the static output.
- TOML-driven configuration (`--init-config` to create one): module order, per-module options,
  display labels, theme, logo and animation; command plugins that turn any script's output
  into rows; and output modes for every consumer — styled split layout, plain, minimal, JSON,
  diagnostics, a share preset that leads with the theme and the live session state, and
  `--watch` continuous refresh with in-place redraw.
- Width-aware layout fallback: a narrow terminal first switches to a compact logo drawing,
  then to denser text layouts that prioritize the core system and Hyprland rows, so nothing
  wraps or overflows.
- A man page (`contrib/hyprfetch.1`) and an Arch PKGBUILD.

- Every theme's foreground and background has been verified against the palettes of the
  theme projects themselves (Tokyo Night, Catppuccin Mocha, Rosé Pine, Dracula, Atom One Dark,
  Solarized, Nord, Everforest, Kanagawa and the rest), and the verified values are locked in a
  test so a future edit cannot drift from the official palettes. The kanagawa, ayu and github
  entries deliberately use the classic tones their terminal ports are built on.

### License

- hyprfetch is released under GPL-3.0; see `LICENSE`.
