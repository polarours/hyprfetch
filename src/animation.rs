use std::fmt::{Display, Formatter};
use std::io::{self, Write};
use std::str::FromStr;
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::model::PaletteName;
use crate::theme;

pub struct Spinner {
    frames: &'static [&'static str],
    color: String,
    message: String,
}

impl Spinner {
    pub fn new(message: &str) -> Self {
        Self {
            frames: &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
            color: "\x1b[36m".to_string(),
            message: message.to_string(),
        }
    }

    pub fn with_palette(message: &str, palette: PaletteName, colors_enabled: bool) -> Self {
        let colors = palette.colors();
        let (frames, color) = if !colors_enabled || colors.mono {
            (&["-", "\\", "|", "/"] as &[&str], String::new())
        } else {
            (
                &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"] as &[&str],
                theme::sgr(colors.accents[0]),
            )
        };

        Self {
            frames,
            color,
            message: message.to_string(),
        }
    }

    /// Milliseconds to wait before drawing the first frame. A local run
    /// finishes in roughly 400 ms, so the spinner stays invisible on a healthy
    /// machine and there is no prompt to clean up. It only appears when a
    /// probe or plugin genuinely stalls, where a progress hint is useful.
    const SPINNER_DELAY_MS: u64 = 500;

    pub fn run<F, T>(self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        let (tx, rx) = std::sync::mpsc::channel();
        let spinner = self;

        let handle = thread::spawn(move || {
            let mut stdout = io::stdout();
            let start = Instant::now();
            let mut printed = false;

            // Wait out the grace period first. If collection finishes before
            // the deadline, return without ever touching stdout.
            loop {
                if rx.try_recv().is_ok() {
                    return false;
                }
                if start.elapsed() >= Duration::from_millis(Self::SPINNER_DELAY_MS) {
                    break;
                }
                thread::sleep(Duration::from_millis(20));
            }

            let mut frame = 0;
            loop {
                if rx.try_recv().is_ok() {
                    break;
                }

                let spinner_char = spinner.frames[frame % spinner.frames.len()];
                let elapsed = start.elapsed().as_secs();
                let dots = ".".repeat(elapsed as usize % 4);

                write!(
                    stdout,
                    "\r\x1b[2K{}{} {}{}\x1b[0m",
                    spinner.color, spinner_char, spinner.message, dots
                )
                .ok();
                stdout.flush().ok();
                printed = true;

                frame += 1;
                thread::sleep(Duration::from_millis(80));
            }

            printed
        });

        let result = f();

        let _ = tx.send(());
        let printed = handle.join().unwrap_or(false);

        if printed {
            print!("\r\x1b[2K");
            io::stdout().flush().ok();
        }

        result
    }
}

pub fn rainbow_text(text: &str, offset: usize) -> String {
    let colors = [
        "\x1b[31m", // red
        "\x1b[33m", // yellow
        "\x1b[32m", // green
        "\x1b[36m", // cyan
        "\x1b[34m", // blue
        "\x1b[35m", // magenta
    ];

    let mut result = String::new();
    for (i, ch) in text.chars().enumerate() {
        let color = colors[(i + offset) % colors.len()];
        result.push_str(color);
        result.push(ch);
    }
    result.push_str("\x1b[0m");
    result
}

pub fn animated_color_bar(frame: usize, bright: bool) -> String {
    let shift = frame % 8;
    let colors = if bright {
        [
            "\x1b[90m", // bright black
            "\x1b[91m", // bright red
            "\x1b[92m", // bright green
            "\x1b[93m", // bright yellow
            "\x1b[94m", // bright blue
            "\x1b[95m", // bright magenta
            "\x1b[96m", // bright cyan
            "\x1b[97m", // bright white
        ]
    } else {
        [
            "\x1b[30m", // black
            "\x1b[31m", // red
            "\x1b[32m", // green
            "\x1b[33m", // yellow
            "\x1b[34m", // blue
            "\x1b[35m", // magenta
            "\x1b[36m", // cyan
            "\x1b[37m", // white
        ]
    };

    let reset = "\x1b[0m";
    let mut result = String::from(" ");
    for i in 0..8 {
        let color = colors[(i + shift) % 8];
        result.push_str(color);
        result.push_str("███");
    }
    result.push_str(reset);
    result
}

pub fn particle_effect(line: usize, max_lines: usize) -> String {
    if line >= max_lines {
        return String::new();
    }

    let positions = ['*', '+', '.', '·', '•', '∙'];
    let mut result = String::new();

    for i in 0..3 {
        let pos = (line * 3 + i * 7) % 20;
        if pos < 15 {
            result.push_str("  ");
        } else {
            let particle = positions[(line + i) % positions.len()];
            let color = match (line + i) % 4 {
                0 => "\x1b[36m",
                1 => "\x1b[35m",
                2 => "\x1b[33m",
                _ => "\x1b[32m",
            };
            result.push_str(color);
            result.push(particle);
            result.push_str("\x1b[0m");
        }
    }

    result
}

/// Total wall time a reveal may take regardless of how many lines it draws.
const REVEAL_BUDGET_MS: u64 = 180;
/// The delay after one revealed line, clamped so the rhythm stays visible on
/// short outputs and never gets sluggish on long dense ones.
const MIN_LINE_DELAY_MS: u64 = 2;
const MAX_LINE_DELAY_MS: u64 = 9;
/// The two-phase reveal splits the budget: the logo skeleton first, then the
/// session rows lighting up behind it.
const SKELETON_BUDGET_MS: u64 = 65;
const ROWS_BUDGET_MS: u64 = 115;
const MIN_PHASE_DELAY_MS: u64 = 1;
const MAX_PHASE_DELAY_MS: u64 = 5;
/// The wave reveal lights two logo columns per frame, under its own budget.
const WAVE_BUDGET_MS: u64 = 90;
const WAVE_COLUMNS_PER_FRAME: usize = 2;

/// The pause after each revealed line of an animated output.
///
/// The delay after a line shrinks as the output grows so the whole reveal
/// always lands inside roughly the same budget: a short split layout reveals
/// with a calm rhythm while a long dense one hurries instead of crawling.
/// Nothing is waited after the final line.
pub fn reveal_delays(line_count: usize) -> Vec<Duration> {
    phase_delays(
        line_count,
        REVEAL_BUDGET_MS,
        MIN_LINE_DELAY_MS,
        MAX_LINE_DELAY_MS,
    )
}

/// The delays of one animation phase, under its own time budget.
fn phase_delays(line_count: usize, budget_ms: u64, min_ms: u64, max_ms: u64) -> Vec<Duration> {
    let per_line = if line_count == 0 {
        0
    } else {
        (budget_ms / line_count as u64).clamp(min_ms, max_ms)
    };
    vec![Duration::from_millis(per_line); line_count.saturating_sub(1)]
}

/// Whether the rendered output should be revealed line by line instead of
/// printed at once. Animation only makes sense on a real terminal, and the
/// machine-readable modes must stay byte-identical for consumers.
pub fn should_reveal(
    animated: bool,
    json: bool,
    diagnostics: bool,
    plain: bool,
    is_tty: bool,
) -> bool {
    animated && !json && !diagnostics && !plain && is_tty
}

/// The animation a user picks for presenting the output: `--animation` on the
/// command line or `animation = "..."` in the config file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum RevealStyle {
    /// The logo skeleton grows first, then the session rows light up (default).
    #[default]
    Split,
    /// One line after another, for a plain printing-press feel.
    Lines,
    /// The logo columns light up as a wave sweeping from the left edge.
    Wave,
    /// No animation at all; the output prints at once.
    Static,
}

impl Display for RevealStyle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Split => "split",
            Self::Lines => "lines",
            Self::Wave => "wave",
            Self::Static => "static",
        };
        f.write_str(value)
    }
}

impl FromStr for RevealStyle {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "split" => Ok(Self::Split),
            "lines" => Ok(Self::Lines),
            "wave" => Ok(Self::Wave),
            "static" => Ok(Self::Static),
            _ => Err(format!("invalid animation: {value}")),
        }
    }
}

/// How a rendered output is presented to the terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reveal {
    /// Print the output at once, the way every non-terminal consumer needs.
    Static,
    /// Reveal line by line, for layouts without a logo column.
    Lines,
    /// Tell the two-phase story: the logo skeleton grows first, then the
    /// session rows light up behind it, within `width` display columns.
    TwoPhase { width: usize },
    /// Light the logo columns of every line as a wave sweeping from the left
    /// edge to the right, then reveal the session rows behind it.
    Wave { width: usize },
}

/// Show a rendered output the chosen way.
pub fn present(output: &str, reveal: &Reveal) {
    match reveal {
        Reveal::Static => print!("{output}"),
        Reveal::Lines => print_reveal_lines(output),
        Reveal::TwoPhase { width } => print_reveal_split(output, *width),
        Reveal::Wave { width } => print_reveal_wave(output, *width),
    }
}

/// The visible prefix of a styled line, cut at `width` display columns.
///
/// SGR escape sequences carry no width, so they pass through untouched; the
/// cut ends with a reset when styling was seen, so the caller never inherits
/// a dangling color. A wide character counts its full width and is never cut
/// in half.
pub(crate) fn visible_prefix(line: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }

    let mut output = String::new();
    let mut styled = false;
    let mut columns = 0usize;
    let mut chars = line.chars();
    while let Some(character) = chars.next() {
        if character == '\x1b' {
            styled = true;
            output.push(character);
            // The escape body runs until its final letter, e.g. `[38;2;1;2;3m`.
            for tail in chars.by_ref() {
                output.push(tail);
                if tail.is_ascii_alphabetic() {
                    break;
                }
            }
            continue;
        }
        let character_width = unicode_width::UnicodeWidthChar::width(character).unwrap_or(0);
        if columns + character_width > width {
            break;
        }
        output.push(character);
        columns += character_width;
    }
    if styled {
        output.push_str("\x1b[0m");
    }
    output
}

/// Reveal a rendered output one line after another.
pub fn print_reveal_lines(output: &str) {
    let lines: Vec<&str> = output.lines().collect();
    if lines.is_empty() {
        return;
    }
    let delays = reveal_delays(lines.len());
    let mut stdout = io::stdout();
    for (index, line) in lines.iter().enumerate() {
        let _ = writeln!(stdout, "{line}");
        let _ = stdout.flush();
        if let Some(delay) = delays.get(index) {
            thread::sleep(*delay);
        }
    }
}

/// Tell the two-phase story: only the logo columns of each line appear first,
/// so the skeleton grows on its own; then the session rows light up row by
/// row behind the finished logo.
pub fn print_reveal_split(output: &str, logo_width: usize) {
    let lines: Vec<&str> = output.lines().collect();
    if lines.is_empty() {
        return;
    }
    let mut stdout = io::stdout();

    let skeleton_delays = phase_delays(
        lines.len(),
        SKELETON_BUDGET_MS,
        MIN_PHASE_DELAY_MS,
        MAX_PHASE_DELAY_MS,
    );
    for (index, line) in lines.iter().enumerate() {
        let _ = writeln!(stdout, "{}", visible_prefix(line, logo_width));
        let _ = stdout.flush();
        if let Some(delay) = skeleton_delays.get(index) {
            thread::sleep(*delay);
        }
    }

    reveal_rows_phase(&mut stdout, &lines);
}

/// Light the logo cells as a wave sweeping from the left edge to the right,
/// then let the session rows appear behind the finished logo.
pub fn print_reveal_wave(output: &str, logo_width: usize) {
    let lines: Vec<&str> = output.lines().collect();
    if lines.is_empty() {
        return;
    }
    let mut stdout = io::stdout();

    // The blank canvas fixes the region the wave draws into and keeps the
    // finished line count without a single visible byte.
    let _ = write!(stdout, "{}", "\n".repeat(lines.len()));
    let _ = stdout.flush();

    let cells: Vec<Vec<(usize, String)>> = lines
        .iter()
        .map(|line| line_cells(line, logo_width))
        .collect();
    let frames = logo_width.div_ceil(WAVE_COLUMNS_PER_FRAME).max(1);
    let frame_delay = Duration::from_millis((WAVE_BUDGET_MS / frames as u64).max(1));
    for frame in 0..frames {
        let band_start = frame * WAVE_COLUMNS_PER_FRAME;
        let band_end = band_start + WAVE_COLUMNS_PER_FRAME;

        // Every frame steps back to the top of the canvas, then walks down
        // one row at a time, drawing only the cells of this column band.
        let _ = write!(stdout, "\x1b[{}A", lines.len());
        for row_cells in &cells {
            let _ = write!(stdout, "\r");
            let mut column = 0usize;
            for (cell_column, bytes) in row_cells {
                if *cell_column >= band_end {
                    break;
                }
                if *cell_column < band_start {
                    continue;
                }
                if *cell_column > column {
                    let _ = write!(stdout, "\x1b[{}C", cell_column - column);
                    column = *cell_column;
                }
                let _ = write!(stdout, "{bytes}");
                // Every logo cell is a single column wide (dots, strokes).
                column += 1;
            }
            let _ = write!(stdout, "\x1b[B");
        }
        // Park the cursor at column zero below the canvas for the next frame.
        let _ = write!(stdout, "\r");
        let _ = stdout.flush();
        thread::sleep(frame_delay);
    }

    reveal_rows_phase(&mut stdout, &lines);
}

/// The second phase every animated story shares: step back up and rewrite
/// every line whole, so the session rows light up row by row behind the
/// finished logo. Rewriting the logo columns writes identical bytes, which no
/// terminal treats as a change.
fn reveal_rows_phase(stdout: &mut io::Stdout, lines: &[&str]) {
    let _ = write!(stdout, "\x1b[{}A", lines.len());
    let rows_delays = phase_delays(
        lines.len(),
        ROWS_BUDGET_MS,
        MIN_PHASE_DELAY_MS,
        MAX_PHASE_DELAY_MS,
    );
    for (index, line) in lines.iter().enumerate() {
        let _ = write!(stdout, "\r{line}\n");
        let _ = stdout.flush();
        if let Some(delay) = rows_delays.get(index) {
            thread::sleep(*delay);
        }
    }
}

/// The non-blank cells of a styled line within its first `width` columns:
/// the display column each one sits at and the bytes that draw it, carrying
/// the SGR style in effect at that point. Blank columns are skipped — the
/// wave only ever writes ink.
fn line_cells(line: &str, width: usize) -> Vec<(usize, String)> {
    if width == 0 {
        return Vec::new();
    }

    let mut cells = Vec::new();
    let mut style = String::new();
    let mut columns = 0usize;
    let mut chars = line.chars();
    while let Some(character) = chars.next() {
        if character == '\x1b' {
            let mut run = String::from("\x1b");
            for tail in chars.by_ref() {
                run.push(tail);
                if tail.is_ascii_alphabetic() {
                    break;
                }
            }
            // A run applies from here on; it replaces whatever was before.
            style = run;
            continue;
        }
        let character_width = unicode_width::UnicodeWidthChar::width(character).unwrap_or(0);
        if columns + character_width > width {
            break;
        }
        if character != ' ' {
            cells.push((columns, format!("{style}{character}")));
        }
        columns += character_width;
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_LINE_DELAY_MS, MIN_LINE_DELAY_MS, ROWS_BUDGET_MS, RevealStyle, SKELETON_BUDGET_MS,
        line_cells, phase_delays, reveal_delays, should_reveal, visible_prefix,
    };

    #[test]
    fn an_empty_or_single_line_output_reveals_without_waiting() {
        assert!(reveal_delays(0).is_empty());
        assert!(reveal_delays(1).is_empty(), "the only line is the last one");
    }

    #[test]
    fn short_outputs_reveal_at_the_calm_pace() {
        // Twenty lines: the budget alone would allow more, so the clamp caps it.
        let delays = reveal_delays(20);
        assert_eq!(delays.len(), 19, "nothing is waited after the final line");
        assert!(
            delays
                .iter()
                .all(|delay| delay.as_millis() == MAX_LINE_DELAY_MS as u128)
        );
    }

    #[test]
    fn long_outputs_finish_within_the_budget() {
        // A hundred dense lines must hurry, not crawl.
        let delays = reveal_delays(100);
        assert_eq!(delays.len(), 99);
        assert!(
            delays
                .iter()
                .all(|delay| delay.as_millis() == MIN_LINE_DELAY_MS as u128)
        );
        let total: u64 = delays.iter().map(|delay| delay.as_millis() as u64).sum();
        assert!(
            total < 600,
            "the reveal must stay under a second, took {total}ms"
        );
    }

    #[test]
    fn reveal_needs_a_terminal_and_a_human_readable_mode() {
        assert!(should_reveal(true, false, false, false, true));
        // Piped output and the machine modes stay static.
        assert!(!should_reveal(true, false, false, false, false));
        assert!(!should_reveal(true, true, false, false, true));
        assert!(!should_reveal(true, false, true, false, true));
        assert!(!should_reveal(true, false, false, true, true));
        assert!(!should_reveal(false, false, false, false, true));
    }

    #[test]
    fn the_two_phases_split_the_reveal_budget() {
        let skeleton = phase_delays(20, SKELETON_BUDGET_MS, 2, 12);
        let rows = phase_delays(20, ROWS_BUDGET_MS, 2, 12);
        assert_eq!(skeleton.len(), 19);
        assert_eq!(rows.len(), 19);
        let skeleton_total: u64 = skeleton.iter().map(|d| d.as_millis() as u64).sum();
        let rows_total: u64 = rows.iter().map(|d| d.as_millis() as u64).sum();
        assert!(
            skeleton_total + rows_total < 600,
            "the full story must stay under a second, took {}",
            skeleton_total + rows_total
        );
    }

    #[test]
    fn a_visible_prefix_cuts_at_the_display_width() {
        let line = "\x1b[38;2;122;162;247mpolarours\x1b[0m@host";
        // The escape sequence passes through without counting towards the
        // width; the cut ends with a reset so the style never leaks.
        let prefix = visible_prefix(line, 9);
        assert!(prefix.starts_with("\x1b[38;2;122;162;247mpolar"));
        assert!(prefix.ends_with("\x1b[0m"));
        assert!(
            !prefix.contains('@'),
            "the cut must not reach the host part"
        );
    }

    #[test]
    fn a_short_line_keeps_its_prefix_intact() {
        assert_eq!(visible_prefix("OS: Arch", 40), "OS: Arch");
        assert_eq!(visible_prefix("", 10), "");
        assert_eq!(visible_prefix("plain", 0), "");
    }

    #[test]
    fn animation_styles_round_trip_through_their_names() {
        use std::str::FromStr;
        for style in [
            RevealStyle::Split,
            RevealStyle::Lines,
            RevealStyle::Wave,
            RevealStyle::Static,
        ] {
            let parsed = RevealStyle::from_str(&style.to_string())
                .unwrap_or_else(|error| panic!("{style} did not parse: {error}"));
            assert_eq!(parsed, style);
        }
        assert!(RevealStyle::from_str("glitter").is_err());
        assert_eq!(RevealStyle::default(), RevealStyle::Split);
    }

    #[test]
    fn the_wave_collects_only_ink_cells_in_order() {
        // A styled skeleton row: two dots, a blank run, then a stroke.
        let line = "  \x1b[38;2;1;2;3m\u{b7}\u{b7}  /  \x1b[0m";
        let cells = line_cells(line, 8);
        assert_eq!(
            cells,
            vec![
                (2, "\x1b[38;2;1;2;3m\u{b7}".to_string()),
                (3, "\x1b[38;2;1;2;3m\u{b7}".to_string()),
                (6, "\x1b[38;2;1;2;3m/".to_string()),
            ],
            "blank columns are skipped and the style in effect carries into every cell"
        );
    }

    #[test]
    fn the_wave_honours_the_width_boundary() {
        // Cutting inside a run of ink stops before the boundary cell.
        let cells = line_cells("abcdef", 3);
        assert_eq!(cells.len(), 3);
        assert_eq!(cells.last().expect("three cells").0, 2);
        assert!(line_cells("abcdef", 0).is_empty());
    }

    #[test]
    fn a_wide_character_is_never_cut_in_half() {
        // The CJK title spans two columns; cutting at an odd width stops
        // before it instead of slicing it.
        let prefix = visible_prefix("ab\u{4e2d}c", 3);
        assert_eq!(prefix, "ab");
    }
}
