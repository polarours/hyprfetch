# Changelog

All notable changes to hyprfetch will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- A clear [!IMPORTANT] notice at the top of the README states that 0.1.0 is an
  early, testing-stage release with no stability guarantees.
- Concrete AUR publishing steps in the README packaging section (account, SSH
  key, package repo clone, PKGBUILD + .SRCINFO, local validation, push), plus an
  updated `contrib/PKGBUILD` placeholder version.

### Fixed

- The README demo GIF had a stacked-duration bug (one frame lasted ~156 s, making
  it look frozen). Re-encoded from the real PTY capture at 900px, 43 frames,
  ~2.9 s total, ~66 KB — loads and loops promptly.

### Changed

- The README is redesigned around a full-width screenshot of the default output,
  with a badge row, a navigation bar and grouped feature sections. The former
  text-only output dump is gone; detailed reference material (config, plugins,
  module IDs, data sources, logo families) is preserved and the logo families
  now live in a collapsible section.

## [0.1.0] - 2026-11-21

### Added

- `--fast` (or `fast = true` in the config) skips expensive probes: CPU usage
  sampling, the GPU vendor tool, audio/WiFi/media external commands. Every
  module row stays but values needing those probes are omitted, never guessed.
  Fast mode renders in ~25 ms versus ~50 ms in normal mode.

### Performance

- Startup is now ~50 ms in release mode (down from ~2.3 s), close to fastfetch's
  range while keeping the live Hyprland state that fastfetch does not collect.
  The remaining big wins:
  - The command helper's reader threads now stop when the child exits instead of
    waiting a fixed grace period for a pipe held open by a background descendant
    (each such command used to add 100 ms; a run has several).
  - The Hyprland ecosystem is detected once and cached; several modules asked for
    it separately and each paid for a full process scan plus version probes.
  - `--version` probes are opt-in: the default compact output and normal JSON
    render skip them; they run only for `versions=true` and full JSON, where the
    version is actually shown. Probing cost ~100 ms on a normal desktop.
  - The CPU usage module runs last so its 50 ms `/proc/stat` sampling window
    overlaps the other modules' work (hyprctl, GPU, audio, network) instead of
    adding its own sleep. Output order is unchanged and the sample remains a
    real 50 ms window; this alone cut startup from ~80 ms to ~50 ms.

- The "Collecting system information" spinner waits 500 ms before drawing, so it
  never flashes in a healthy run; it only appears when a probe or plugin stalls.

### Fixed

- The `Wallpaper` key was missing from the theme-row color group, so it rendered
  in the muted gray while Theme/Ecosystem/Input shared a colored accent. It now
  shares that accent; a regression test locks all four keys together across
  every palette.

- NetworkManager is detected again: the process list carries lowercased names and a root
  daemon offers no readable executable, so the name comparison is now case-insensitive (it
  reported `iwd` on a machine running both).
- Keys in the minimal layout keep their accent colors: the right-aligned padding made the
  label miss its color match and every short key rendered in the muted gray.
- Three unbounded waits on external commands are bounded: the shared helper now drains the
  pipes under its deadline (a background descendant holding the pipe can no longer hang the
  run), `rpm -qa` gives up while the database is locked by another transaction, and
  `nvidia-smi` no longer stalls behind a wedged driver.
- The spinner respects `NO_COLOR` like the rendered output does.
- The two-phase and wave reveals fall back to the plain line-by-line reveal when the output
  is as tall as the terminal window, whose cursor cannot step back to the first row.
- Twenty-seven bundled artworks lost ink where a literal `$` was mistaken for a fastfetch
  colour marker; literal dollars are now escaped in the assets, so every ink dot survives.
- The material theme's officially verified foreground/background is locked in the ground-truth
  test with the rest of the catalogue.

## [0.0.1] - 2026-11-19

The squashed initial baseline (`init commit`) of hyprfetch, a Hyprland-native fetch utility.

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
- Thirty-four alternate drawings of covered distros — numbered takes (`arch2`, `artix2`,
  `macos2`, `void2`, ...), older artwork (`fedora_old`, `ubuntu_old`, ...) and retro variants —
  are selectable by name without joining the auto detection (`--logo arch2`), and the `minimal`
  preset now renders the bundled wordmark artwork instead of a bare string.
- A logo catalogue of 429 presets drawn from the bundled artwork and rendered in a minimal
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
