//! Terminal background detection (issue #233).
//!
//! The cockpit never paints a background: `Palette` has no `bg` slot, so
//! ratatui lets the terminal's own ground show through — which keeps the
//! pilot's transparency, blur and colour profile intact, and means the only
//! way to stay legible is to pick foregrounds that contrast with *their*
//! ground. So we have to ask what that ground is.
//!
//! We ask the terminal, not the OS. `AppleInterfaceStyle`, the Windows
//! personalisation registry key and the freedesktop colour-scheme portal all
//! report the *desktop* preference, and a dark terminal on a light desktop (or
//! the reverse) is common enough that using them would guess wrong for exactly
//! the people who care most about their terminal's appearance.
//!
//! ```text
//! query:  ESC ] 11 ; ? BEL
//! reply:  ESC ] 11 ; rgb:RRRR/GGGG/BBBB ST
//! ```
//!
//! Two rules govern everything below:
//!
//! 1. **Never read stdin.** crossterm's `EventStream` owns it, and a reply we
//!    half-consume there is a stolen keypress. On Unix we open `/dev/tty`
//!    ourselves (the same trick crossterm uses for its own terminal queries);
//!    on Windows we open the console input handle directly.
//! 2. **Never block.** Plenty of terminals never answer — tmux and screen
//!    swallow the query, some SSH setups drop it, Apple Terminal answers
//!    partially, CI has no terminal at all. Every read is bounded by
//!    [`QUERY_TIMEOUT`], and every failure means [`Polarity::Dark`], which is
//!    what every palette assumed before this existed.

use crate::app::Polarity;
use std::time::Duration;

/// How long to wait for a reply before giving up and assuming a dark ground.
/// Short on purpose: this sits in the startup path, and a terminal that is
/// going to answer answers immediately.
pub const QUERY_TIMEOUT: Duration = Duration::from_millis(100);

/// Detect the terminal's background polarity, falling back to `Dark`.
///
/// Must run **before** `crossterm::EventStream` starts consuming input.
pub fn detect_polarity() -> Polarity {
    if let Some(polarity) = polarity_from_colorfgbg() {
        return polarity;
    }
    match query_background() {
        Some((r, g, b)) => polarity_for(r, g, b),
        None => Polarity::Dark,
    }
}

/// `COLORFGBG` is set by a handful of terminals (rxvt, konsole, some Windows
/// setups) and reads `foreground;background` in ANSI colour numbers. It costs
/// nothing to consult and answers where OSC 11 often cannot, so it goes first.
fn polarity_from_colorfgbg() -> Option<Polarity> {
    let raw = std::env::var("COLORFGBG").ok()?;
    // The background is the last field: some terminals emit `fg;bg`, others
    // `fg;<something>;bg`.
    let bg: u8 = raw.rsplit(';').next()?.trim().parse().ok()?;
    // 0-6 and 8 are the dark half of the ANSI 16; 7 and 9-15 are the light one.
    Some(if bg == 7 || bg >= 9 {
        Polarity::Light
    } else {
        Polarity::Dark
    })
}

/// Perceived-luminance threshold. The classic NTSC weighting is enough here:
/// we are answering "is this ground dark or light", not matching colours.
pub fn polarity_for(r: u16, g: u16, b: u16) -> Polarity {
    let norm = |v: u16| v as f64 / u16::MAX as f64;
    let luma = 0.299 * norm(r) + 0.587 * norm(g) + 0.114 * norm(b);
    if luma > 0.5 {
        Polarity::Light
    } else {
        Polarity::Dark
    }
}

/// Parse an OSC 11 reply into 16-bit RGB.
///
/// Terminals differ in how many hex digits they emit per channel (`rgb:1/2/3`
/// through `rgb:1111/2222/3333`) and in how they terminate the reply (BEL or
/// ST), so both are scaled and both terminators tolerated. Anything else is
/// `None` — a malformed reply must not become a confident answer.
///
/// The `11` is matched, not skipped: OSC 10 (foreground) has the identical
/// `rgb:` payload, and reading one as the other would invert the polarity on
/// any terminal that answers both.
pub fn parse_osc11(reply: &str) -> Option<(u16, u16, u16)> {
    let osc = reply.find("]11;")? + "]11;".len();
    let start = reply[osc..].find("rgb:")? + osc + "rgb:".len();
    let rest = &reply[start..];
    let end = rest.find(['\x07', '\x1b']).unwrap_or(rest.len());
    let mut parts = rest[..end].split('/');
    let mut channel = || -> Option<u16> {
        let raw = parts.next()?.trim();
        if raw.is_empty() || raw.len() > 4 || !raw.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let value = u32::from_str_radix(raw, 16).ok()?;
        // Scale whatever width the terminal used up to 16 bits: `f` is full
        // intensity just as much as `ffff` is.
        let max = 16u32.pow(raw.len() as u32) - 1;
        Some((value * u16::MAX as u32 / max) as u16)
    };
    let (r, g, b) = (channel()?, channel()?, channel()?);
    if parts.next().is_some() {
        return None; // more than three channels: not an OSC 11 reply
    }
    Some((r, g, b))
}

/// The OSC 11 query, and the byte sequence that ends a reply.
const QUERY: &[u8] = b"\x1b]11;?\x07";

#[cfg(unix)]
fn query_background() -> Option<(u16, u16, u16)> {
    use std::fs::OpenOptions;
    use std::io::{Read, Write};
    use std::os::unix::io::AsRawFd;

    // `/dev/tty` rather than stdin: crossterm's event reader owns stdin, and a
    // reply read from under it is a keypress the pilot never gets back.
    let mut tty = OpenOptions::new().read(true).write(true).open("/dev/tty").ok()?;
    tty.write_all(QUERY).ok()?;
    tty.flush().ok()?;

    let fd = tty.as_raw_fd();
    let deadline = std::time::Instant::now() + QUERY_TIMEOUT;
    let mut buf = Vec::with_capacity(64);
    let mut chunk = [0u8; 32];
    loop {
        let left = deadline.saturating_duration_since(std::time::Instant::now());
        if left.is_zero() {
            return None;
        }
        // poll(2) so a terminal that never answers costs us 100 ms, not the
        // session: a blocking read here would hang the boot on tmux.
        let mut pfd = libc::pollfd {
            fd,
            events: libc::POLLIN,
            revents: 0,
        };
        let ready = unsafe { libc::poll(&mut pfd, 1, left.as_millis() as libc::c_int) };
        if ready <= 0 {
            return None;
        }
        let n = tty.read(&mut chunk).ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(rgb) = parse_osc11(&String::from_utf8_lossy(&buf)) {
            return Some(rgb);
        }
        if buf.len() > 256 {
            return None; // whatever this is, it is not our reply
        }
    }
}

#[cfg(windows)]
fn query_background() -> Option<(u16, u16, u16)> {
    use std::io::Write;
    use windows_sys::Win32::Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE, WAIT_OBJECT_0};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, ReadFile, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::Threading::WaitForSingleObject;

    // The console input handle, opened directly for the same reason `/dev/tty`
    // is used on Unix: stdin belongs to crossterm.
    let name: Vec<u16> = "CONIN$\0".encode_utf16().collect();
    let handle: HANDLE = unsafe {
        CreateFileW(
            name.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        )
    };
    if handle.is_null() || handle == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
        return None;
    }
    // The reply only arrives as VT input, which crossterm has already enabled
    // by the time the preflight runs; on an older console that never happens
    // and the wait below simply times out into `Dark`.
    let mut out = std::io::stdout();
    let wrote = out.write_all(QUERY).and_then(|_| out.flush()).is_ok();

    let mut rgb = None;
    if wrote {
        let deadline = std::time::Instant::now() + QUERY_TIMEOUT;
        let mut buf: Vec<u8> = Vec::with_capacity(64);
        let mut chunk = [0u8; 32];
        loop {
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            if left.is_zero() {
                break;
            }
            if unsafe { WaitForSingleObject(handle, left.as_millis() as u32) } != WAIT_OBJECT_0 {
                break;
            }
            let mut read: u32 = 0;
            let ok = unsafe {
                ReadFile(
                    handle,
                    chunk.as_mut_ptr(),
                    chunk.len() as u32,
                    &mut read,
                    std::ptr::null_mut(),
                )
            };
            if ok == 0 || read == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..read as usize]);
            if let Some(parsed) = parse_osc11(&String::from_utf8_lossy(&buf)) {
                rgb = Some(parsed);
                break;
            }
            if buf.len() > 256 {
                break;
            }
        }
    }
    unsafe { CloseHandle(handle) };
    rgb
}

#[cfg(not(any(unix, windows)))]
fn query_background() -> Option<(u16, u16, u16)> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_four_digit_reply() {
        // xterm's own form, BEL-terminated.
        let rgb = parse_osc11("\x1b]11;rgb:0000/0000/0000\x07").unwrap();
        assert_eq!(rgb, (0, 0, 0));
        assert_eq!(polarity_for(rgb.0, rgb.1, rgb.2), Polarity::Dark);

        let white = parse_osc11("\x1b]11;rgb:ffff/ffff/ffff\x1b\\").unwrap();
        assert_eq!(white, (65535, 65535, 65535));
        assert_eq!(polarity_for(white.0, white.1, white.2), Polarity::Light);
    }

    #[test]
    fn scales_shorter_channel_widths() {
        // `f` means full intensity just as much as `ffff` does.
        assert_eq!(parse_osc11("\x1b]11;rgb:f/f/f\x07"), Some((65535, 65535, 65535)));
        assert_eq!(parse_osc11("\x1b]11;rgb:ff/ff/ff\x07"), Some((65535, 65535, 65535)));
        assert_eq!(parse_osc11("\x1b]11;rgb:00/00/00\x07"), Some((0, 0, 0)));
    }

    #[test]
    fn refuses_anything_it_cannot_read() {
        // A malformed reply must not become a confident answer.
        assert_eq!(parse_osc11(""), None);
        assert_eq!(parse_osc11("\x1b]11;rgb:zz/00/00\x07"), None);
        assert_eq!(parse_osc11("\x1b]11;rgb:0000/0000\x07"), None, "two channels");
        assert_eq!(parse_osc11("\x1b]11;rgb:1/2/3/4\x07"), None, "four channels");
        assert_eq!(parse_osc11("\x1b]10;rgb:0/0/0\x07"), None, "that is the foreground");
        // A terminal answering both queries must not have its foreground read
        // as its background.
        assert_eq!(
            parse_osc11("\x1b]10;rgb:ffff/ffff/ffff\x07\x1b]11;rgb:0000/0000/0000\x07"),
            Some((0, 0, 0)),
            "the background is the one that counts"
        );
    }

    #[test]
    fn a_solarized_light_ground_reads_as_light() {
        // #fdf6e3 — pale, but not white; the threshold has to catch it.
        let rgb = parse_osc11("\x1b]11;rgb:fdfd/f6f6/e3e3\x07").unwrap();
        assert_eq!(polarity_for(rgb.0, rgb.1, rgb.2), Polarity::Light);
        // #002b36 — Solarized dark, a blue-grey that is nowhere near black.
        let dark = parse_osc11("\x1b]11;rgb:0000/2b2b/3636\x07").unwrap();
        assert_eq!(polarity_for(dark.0, dark.1, dark.2), Polarity::Dark);
    }
}
