//! The OpenGL implementation behind the compositor.
//!
//! `glxinfo -B` reports the version string and the renderer, which is the pair
//! people actually look for when a game or a compositor misbehaves. It needs
//! XWayland on a Wayland session, so the module is opt-in and carries a
//! timeout: a missing binary or a slow X server must not stall the render.

use std::time::Duration;

use super::{SystemProvider, run_with_timeout};

/// What `glxinfo` reported.
#[derive(Debug, Clone, PartialEq)]
pub struct OpenGlInfo {
    /// `4.6.0 NVIDIA 620.73.04` — version plus driver, as Mesa/NVIDIA print it.
    pub version: String,
    /// `NVIDIA GeForce RTX 4060 Laptop GPU` — the renderer without the
    /// `/PCIe/SSE2` transport suffix, which is noise in a one-line row.
    pub renderer: String,
}

impl OpenGlInfo {
    pub fn display(&self) -> String {
        self.version.clone()
    }
}

impl SystemProvider {
    /// OpenGL core-profile version and renderer.
    pub fn opengl(&self) -> Option<OpenGlInfo> {
        let output = run_with_timeout("glxinfo", &["-B"], Duration::from_millis(900))?;
        parse_glxinfo(&output)
    }
}

/// Pull the version and renderer out of `glxinfo -B`.
fn parse_glxinfo(output: &str) -> Option<OpenGlInfo> {
    let mut version = None;
    let mut renderer = None;

    for line in output.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        let key = key.trim();
        if value.is_empty() {
            continue;
        }
        // Prefer the core profile; fall back to the compatibility one, which
        // glxinfo prints first and which must not overwrite a core string.
        let is_core_version = key.starts_with("OpenGL core profile version string");
        let is_compat_version = version.is_none() && key.starts_with("OpenGL version string");
        if is_core_version || is_compat_version {
            version = Some(value.to_string());
        } else if key.starts_with("OpenGL core profile renderer")
            || key.starts_with("OpenGL renderer string")
        {
            renderer = Some(trim_transport(value));
        }
    }

    let version = version?;
    let renderer = renderer.unwrap_or_default();
    Some(OpenGlInfo { version, renderer })
}

/// Drop the `/PCIe/SSE2` (or similar) transport suffix glxinfo appends.
fn trim_transport(renderer: &str) -> String {
    renderer
        .split('/')
        .next()
        .unwrap_or(renderer)
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{OpenGlInfo, parse_glxinfo, trim_transport};

    const SAMPLE: &str = "\
name of display: :0
display: :0  screen: 0
direct rendering: Yes
Extended renderer info (GLX_MESA_query_renderer):
    Vendor: NVIDIA Corporation (0x10de)
    Device: NVIDIA GeForce RTX 4060 Laptop GPU (0x28e0)
OpenGL vendor string: NVIDIA Corporation
OpenGL renderer string: NVIDIA GeForce RTX 4060 Laptop GPU/PCIe/SSE2
OpenGL core profile version string: 4.6.0 NVIDIA 620.73.04
OpenGL core profile shading language version string: 4.60 NVIDIA
";

    #[test]
    fn parses_the_core_profile() {
        let info = parse_glxinfo(SAMPLE).unwrap();
        assert_eq!(info.version, "4.6.0 NVIDIA 620.73.04");
        assert_eq!(info.renderer, "NVIDIA GeForce RTX 4060 Laptop GPU");
    }

    #[test]
    fn falls_back_to_the_compatibility_version() {
        let output = "OpenGL renderer string: llvmpipe (LLVM 17.0.6, 256 bits)/PCIe/SSE2\n\
                      OpenGL version string: 4.5 (Compatibility Profile) Mesa 24.0.0\n";
        let info = parse_glxinfo(output).unwrap();
        assert_eq!(info.version, "4.5 (Compatibility Profile) Mesa 24.0.0");
        assert_eq!(info.renderer, "llvmpipe (LLVM 17.0.6, 256 bits)");
    }

    #[test]
    fn an_empty_payload_has_no_row() {
        assert!(parse_glxinfo("").is_none());
        assert!(parse_glxinfo("Error: unable to open display\n").is_none());
    }

    #[test]
    fn transport_suffix_is_trimmed() {
        assert_eq!(trim_transport("Foo/PCIe/SSE2"), "Foo");
        assert_eq!(trim_transport("Bar"), "Bar");
    }

    #[test]
    fn display_is_the_version() {
        let info = OpenGlInfo {
            version: "4.6.0 NVIDIA 620".into(),
            renderer: "GPU".into(),
        };
        assert_eq!(info.display(), "4.6.0 NVIDIA 620");
    }
}
