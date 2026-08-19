use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};

use crate::model::PaletteName;

pub struct Spinner {
    frames: &'static [&'static str],
    color: &'static str,
    message: String,
}

impl Spinner {
    pub fn new(message: &str) -> Self {
        Self {
            frames: &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
            color: "\x1b[36m",
            message: message.to_string(),
        }
    }

    pub fn with_palette(message: &str, palette: PaletteName, colors_enabled: bool) -> Self {
        let (frames, color) = if !colors_enabled {
            (&["-", "\\", "|", "/"] as &[&str], "")
        } else {
            match palette {
                PaletteName::Hypr => (
                    &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"] as &[&str],
                    "\x1b[36m",
                ),
                PaletteName::Mono => (&["-", "\\", "|", "/"] as &[&str], ""),
                PaletteName::Catppuccin => (
                    &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"] as &[&str],
                    "\x1b[35m",
                ),
                PaletteName::Dracula => (
                    &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"] as &[&str],
                    "\x1b[35m",
                ),
            }
        };

        Self {
            frames,
            color,
            message: message.to_string(),
        }
    }

    pub fn run<F, T>(self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        let (tx, rx) = std::sync::mpsc::channel();
        let spinner = self;

        let handle = thread::spawn(move || {
            let mut stdout = io::stdout();
            let mut frame = 0;
            let start = Instant::now();

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

                frame += 1;
                thread::sleep(Duration::from_millis(80));
            }
        });

        let result = f();

        let _ = tx.send(());
        let _ = handle.join();

        print!("\r\x1b[2K");
        io::stdout().flush().ok();

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
