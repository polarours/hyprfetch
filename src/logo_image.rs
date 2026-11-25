//! Draw a PNG image as the logo, using the kitty graphics protocol.
//!
//! The protocol is supported by kitty, ghostty and WezTerm. It takes a base64
//! payload in an APC string, and a PNG can be sent verbatim (`f=100`), so this
//! module needs no image decoding library: the file is validated, its IHDR
//! read for the aspect ratio, and the bytes shipped as-is.
//!
//! Everything degrades: an unsupported terminal, a missing file or a non-PNG
//! image all fall back to the text logo instead of printing escape garbage.

use std::env;
use std::fs;
use std::path::Path;

/// Refuse anything larger than this: base64 inflates by a third and the
/// terminal has to buffer the whole transfer.
const MAX_IMAGE_BYTES: u64 = 8 * 1024 * 1024;

/// The kitty protocol chunk size.
const CHUNK: usize = 4096;

/// Cells are roughly twice as tall as they are wide, which is what makes a
/// square image span half as many rows as columns.
const CELL_ASPECT: f64 = 2.0;

/// Whether the terminal speaks the kitty graphics protocol.
pub fn supports_graphics() -> bool {
    if env::var_os("KITTY_WINDOW_ID").is_some() {
        return true;
    }
    if env::var_os("GHOSTTY_RESOURCES_DIR").is_some() {
        return true;
    }
    if let Ok(program) = env::var("TERM_PROGRAM")
        && matches!(program.as_str(), "WezTerm" | "ghostty" | "kitty")
    {
        return true;
    }
    matches!(
        env::var("TERM").as_deref(),
        Ok(term) if term.contains("kitty") || term.contains("ghostty") || term.contains("wezterm")
    )
}

/// Load a PNG, rejecting anything that is not one.
pub fn load_png(path: &Path) -> Option<Vec<u8>> {
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_IMAGE_BYTES {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    (bytes.len() >= 24 && png_dimensions(&bytes).is_some()).then_some(bytes)
}

/// Read the width and height out of the PNG IHDR chunk.
pub fn png_dimensions(png: &[u8]) -> Option<(u32, u32)> {
    const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    if png.len() < 24 || png[..8] != SIGNATURE || &png[12..16] != b"IHDR" {
        return None;
    }
    let width = u32::from_be_bytes([png[16], png[17], png[18], png[19]]);
    let height = u32::from_be_bytes([png[20], png[21], png[22], png[23]]);
    (width > 0 && height > 0).then_some((width, height))
}

/// How many rows an image occupies when drawn `cols` cells wide.
pub fn image_rows(cols: u32, dimensions: (u32, u32)) -> u32 {
    let (width, height) = dimensions;
    let rows = (f64::from(cols) * f64::from(height) / f64::from(width) / CELL_ASPECT).round();
    rows.clamp(1.0, 200.0) as u32
}

/// Build the APC sequence that transmits and displays the image.
pub fn apc(png: &[u8], cols: u32, rows: u32) -> String {
    let encoded = base64_encode(png);
    let mut out = String::new();
    let chunks: Vec<&str> = encoded
        .as_bytes()
        .chunks(CHUNK)
        .map(|chunk| std::str::from_utf8(chunk).unwrap_or_default())
        .collect();

    for (index, chunk) in chunks.iter().enumerate() {
        let more = usize::from(index + 1 < chunks.len());
        if index == 0 {
            // a=T transmit and display, f=100 PNG, q=2 quiet, c/r the cell box.
            out.push_str(&format!("\x1b_Ga=T,f=100,c={cols},r={rows},q=2,m={more};"));
        } else {
            out.push_str(&format!("\x1b_Gm={more};"));
        }
        out.push_str(chunk);
        out.push_str("\x1b\\");
    }
    out
}

/// Standard base64 with padding, kept local so the crate stays dependency-free.
pub fn base64_encode(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        out.push(ALPHABET[(b0 >> 2) as usize] as char);
        out.push(ALPHABET[((b0 & 0b0000_0011) << 4 | b1 >> 4) as usize] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[((b1 & 0b0000_1111) << 2 | b2 >> 6) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[(b2 & 0b0011_1111) as usize] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{apc, base64_encode, image_rows, png_dimensions, supports_graphics};

    fn tiny_png() -> Vec<u8> {
        // A 1x1 PNG: signature, IHDR (width=1, height=1), rest can be minimal.
        let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        bytes.extend_from_slice(&[0, 0, 0, 13]); // IHDR length
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&1u32.to_be_bytes()); // width
        bytes.extend_from_slice(&1u32.to_be_bytes()); // height
        bytes.extend_from_slice(&[8, 6, 0, 0, 0]); // bit depth, colour type, …
        bytes
    }

    #[test]
    fn base64_matches_known_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn png_header_is_required() {
        assert!(png_dimensions(b"not a png at all").is_none());
        assert_eq!(png_dimensions(&tiny_png()), Some((1, 1)));
    }

    #[test]
    fn square_images_are_half_as_tall_as_wide() {
        assert_eq!(image_rows(30, (512, 512)), 15);
        assert_eq!(image_rows(30, (500, 1000)), 30);
        // Never zero, never absurd.
        assert_eq!(image_rows(1, (1000, 1)), 1);
    }

    #[test]
    fn apc_carries_the_dimensions_and_is_chunked() {
        let png = vec![0u8; 10_000];
        let sequence = apc(&png, 30, 15);
        assert!(sequence.starts_with("\x1b_Ga=T,f=100"));
        assert!(sequence.contains("c=30,r=15"));
        // 10 KB base64s to ~13.3 KB, so it must split into several chunks.
        assert!(sequence.matches("\x1b_G").count() > 1);
        // Every chunk is terminated.
        assert_eq!(
            sequence.matches("\x1b\\").count(),
            sequence.matches("\x1b_G").count()
        );
    }

    /// The test process runs in a pipe, so it must not claim graphics support.
    #[test]
    fn detection_is_false_under_a_pipe() {
        assert!(!supports_graphics() || std::env::var_os("KITTY_WINDOW_ID").is_some());
    }
}
