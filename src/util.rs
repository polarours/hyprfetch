use std::io::{self, Read};
use std::process::Child;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use std::path::Path;

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub fn basename(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .map_or_else(|| path.to_string(), ToString::to_string)
}

pub fn strip_wrapping_quotes(value: &str) -> String {
    value.trim_matches('"').trim().to_string()
}

pub fn humanize_uptime(total_seconds: f64) -> String {
    let mut remaining = total_seconds.max(0.0).floor() as u64;
    let days = remaining / 86_400;
    remaining %= 86_400;
    let hours = remaining / 3_600;
    remaining %= 3_600;
    let minutes = remaining / 60;

    let mut parts = Vec::new();
    if days > 0 {
        parts.push(format!("{days}d"));
    }
    if hours > 0 {
        parts.push(format!("{hours}h"));
    }
    if minutes > 0 || parts.is_empty() {
        parts.push(format!("{minutes}m"));
    }
    parts.join(" ")
}

pub fn humanize_kib(kib: u64) -> String {
    let gib = kib as f64 / 1024.0 / 1024.0;
    if gib >= 10.0 {
        format!("{gib:.0} GiB")
    } else {
        format!("{gib:.1} GiB")
    }
}

pub fn format_refresh_hz(refresh_hz: f64) -> String {
    let rounded = refresh_hz.round();
    if (refresh_hz - rounded).abs() < 0.05 {
        format!("{rounded:.0}Hz")
    } else {
        format!("{refresh_hz:.1}Hz")
    }
}

pub fn truncate_visible(value: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }

    if UnicodeWidthStr::width(value) <= max_chars {
        return value.to_string();
    }

    if max_chars == 1 {
        return "…".to_string();
    }

    let mut output = String::new();
    let mut width = 0;
    for character in value.chars() {
        let character_width = UnicodeWidthChar::width(character).unwrap_or(0);
        if width + character_width + 1 > max_chars {
            break;
        }
        output.push(character);
        width += character_width;
    }

    // Cutting a value like "launch:walker (2.17.2)" in the middle of the
    // version is worse than dropping the last few words, so the cut moves
    // back to the last space when that does not throw away much.
    // `rfind` returns a byte offset, so measure the tail by display width;
    // comparing it against the byte index would drop multibyte text wholesale.
    if let Some(space) = output.rfind(' ')
        && UnicodeWidthStr::width(&output[space + 1..]) <= max_chars / 4
    {
        output.truncate(space);
    }

    output.push('…');
    output
}

/// Whether standard output is attached to a terminal.
///
/// Interactive-only output such as screen redraws must be skipped when the
/// output is piped or redirected to a file.
pub fn stdout_is_tty() -> bool {
    #[cfg(unix)]
    {
        // SAFETY: `isatty` only inspects the file descriptor and has no side effects.
        unsafe { libc::isatty(libc::STDOUT_FILENO) == 1 }
    }
    #[cfg(not(unix))]
    {
        false
    }
}

/// A child that has finished, with everything it wrote.
pub struct FinishedCommand {
    pub status: std::process::ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// The result of waiting for a command under a deadline.
pub enum CommandWait {
    Finished(FinishedCommand),
    TimedOut,
}

/// Wait for a spawned child (whose stdout and stderr are piped) with a single
/// deadline covering both the process and the pipe reads.
///
/// Two failure modes drove this design: a child that writes more than a pipe
/// buffer blocks in `write` unless somebody drains the pipe, which used to be
/// reported as a timeout; and `wait_with_output` reads until EOF with no
/// deadline, so a background descendant holding the pipe hung the caller.
/// Reader threads drain both pipes continuously, and the final read is capped
/// by a short grace period once the child exits.
// A background descendant can inherit a command's pipes and keep them open
// after the command exits, so a reader blocked on `read()` never sees EOF.
// Once the child is gone, the reader threads are told to stop and the output
// already buffered in the channel is drained; this keeps the wait bounded
// without a fixed sleep that would add latency to every command.
const DRAIN_POLL_MS: u64 = 2;

pub fn wait_with_deadline(mut child: Child, timeout: Duration) -> io::Result<CommandWait> {
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let (tx_out, rx_out) = mpsc::channel();
    let (tx_err, rx_err) = mpsc::channel();

    // Stream chunks instead of read_to_end: a descendant that inherited the
    // pipe may keep it open forever, so waiting for EOF would lose all data.
    // The stop flag lets us end the readers once the child exits, even when a
    // descendant still holds the pipe open.
    let stop = Arc::new(AtomicBool::new(false));
    if let Some(pipe) = stdout {
        let stop = Arc::clone(&stop);
        thread::spawn(move || drain_pipe(pipe, tx_out, stop));
    }
    if let Some(pipe) = stderr {
        let stop = Arc::clone(&stop);
        thread::spawn(move || drain_pipe(pipe, tx_err, stop));
    }

    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(CommandWait::TimedOut);
        }
        thread::sleep(Duration::from_millis(10));
    };

    // The child has exited: tell the readers to stop and drain the chunks
    // they already delivered. No waiting for EOF or for the full timeout —
    // a background descendant that inherited the pipe would hold both open.
    stop.store(true, Ordering::Relaxed);
    let stdout = collect_stopped(rx_out);
    let stderr = collect_stopped(rx_err);
    Ok(CommandWait::Finished(FinishedCommand {
        status,
        stdout,
        stderr,
    }))
}

fn drain_pipe(
    mut pipe: impl Read + std::os::fd::AsRawFd,
    tx: mpsc::Sender<Vec<u8>>,
    stop: Arc<AtomicBool>,
) {
    // Non-blocking reads remove the core race: once the child exits we set the
    // stop flag, but a reader must first hand over every byte still buffered
    // in the pipe. With a blocking read a descendant holding the pipe could
    // keep the reader asleep past the stop signal; with O_NONBLOCK the reader
    // drains everything available, then observes the stop on the next empty
    // poll and exits. No data is lost and no descendant can pin the waiter.
    #[cfg(unix)]
    {
        unsafe {
            let fd = pipe.as_raw_fd();
            let flags = libc::fcntl(fd, libc::F_GETFL);
            if flags >= 0 {
                libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
            }
        }
    }

    let mut buffer = [0u8; 8192];
    loop {
        match pipe.read(&mut buffer) {
            Ok(0) => break, // EOF: no descendant holds the write end
            Ok(count) => {
                if tx.send(buffer[..count].to_vec()).is_err() {
                    break;
                }
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                thread::sleep(Duration::from_millis(1));
            }
            Err(_) => break,
        }
    }
}

fn collect_stopped(rx: mpsc::Receiver<Vec<u8>>) -> Vec<u8> {
    let mut output = Vec::new();
    // Wait for the reader threads to hand over buffered chunks. The deadline
    // is refreshed whenever a chunk arrives: a reader that was slow to be
    // scheduled still gets a window to deliver the rest, while an idle pipe
    // (a descendant holding it open) stops after one short poll. A hard cap
    // bounds the whole drain so a chatty replica cannot pin the caller.
    let grace = Duration::from_millis(DRAIN_POLL_MS);
    let hard_cap = Instant::now() + Duration::from_millis(25);
    let mut deadline = Instant::now() + grace;
    loop {
        let now = Instant::now();
        let wait_until = deadline.min(hard_cap);
        if now >= wait_until {
            break;
        }
        match rx.recv_timeout(wait_until.saturating_duration_since(now)) {
            Ok(chunk) => {
                output.extend_from_slice(&chunk);
                deadline = Instant::now() + grace;
            }
            Err(_) => break,
        }
    }
    // Channels close when the readers exit; take anything already queued.
    while let Ok(chunk) = rx.try_recv() {
        output.extend_from_slice(&chunk);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{
        CommandWait, format_refresh_hz, humanize_kib, humanize_uptime, truncate_visible,
        wait_with_deadline,
    };
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    #[test]
    fn wait_drains_large_output_instead_of_timing_out() {
        // A child that writes far more than a pipe buffer used to block on
        // write while the parent polled try_wait, then get killed as "timed
        // out". Draining the pipes while waiting avoids the false timeout.
        let child = Command::new("sh")
            .args(["-c", "python3 -c \"print('x'*200000)\""])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        match wait_with_deadline(child, Duration::from_secs(5)).unwrap() {
            CommandWait::Finished(output) => {
                assert!(output.status.success());
                assert_eq!(output.stdout.len(), 200_001);
            }
            CommandWait::TimedOut => panic!("large output was mistaken for a hang"),
        }
    }

    #[test]
    fn wait_does_not_hang_on_a_background_descendant() {
        // The direct child exits immediately but leaves a background process
        // holding stdout open. wait_with_output would block until EOF forever;
        // the deadline on the pipe reads must bound this.
        let child = Command::new("sh")
            .args(["-c", "printf hi; sleep 2 &"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        let started = Instant::now();
        let result = wait_with_deadline(child, Duration::from_millis(200)).unwrap();
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "wait hung on a background descendant: {:?}",
            started.elapsed()
        );
        assert!(matches!(result, CommandWait::Finished(_)));
    }

    #[test]
    fn humanizes_uptime() {
        assert_eq!(humanize_uptime(61.0), "1m");
        assert_eq!(humanize_uptime(9_061.0), "2h 31m");
        assert_eq!(humanize_uptime(90_061.0), "1d 1h 1m");
    }

    #[test]
    fn humanizes_kib() {
        assert_eq!(humanize_kib(8 * 1024 * 1024), "8.0 GiB");
        assert_eq!(humanize_kib(32 * 1024 * 1024), "32 GiB");
    }

    #[test]
    fn formats_refresh_rate() {
        assert_eq!(format_refresh_hz(60.0), "60Hz");
        assert_eq!(format_refresh_hz(59.94), "59.9Hz");
    }

    #[test]
    fn truncates_long_strings() {
        assert_eq!(truncate_visible("hyprfetch", 5), "hypr…");
        assert_eq!(truncate_visible("hypr", 10), "hypr");
    }

    #[test]
    fn truncates_visible_width() {
        assert_eq!(truncate_visible("你好world", 6), "你好w…");
        assert_eq!(truncate_visible("你好", 1), "…");
    }

    #[test]
    fn keeps_the_last_word_of_multibyte_text_when_cutting() {
        // The backtrack-to-last-space check compares display width, not the
        // byte index returned by rfind. A Greek letter is two bytes but one
        // column, so a byte offset made the check drop far too much text.
        assert_eq!(truncate_visible("ααααα αααααααααα", 12), "ααααα ααααα…");
    }
}
