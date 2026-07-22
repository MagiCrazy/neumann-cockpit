//! Desktop notifications for completed long tasks (issue #203).
//!
//! Emits an OSC 9 escape sequence — surfaced as a desktop notification by
//! terminals that support it (iTerm2, WezTerm, ConEmu, kitty via its own
//! protocol, …) — terminated with ST, followed by a bell as an audible
//! fallback for terminals that ignore OSC 9. It is the terminal, not the OS,
//! that decides how to present it, so this is cross-platform by construction.

use std::io::{self, Write};

/// Post a desktop notification carrying `message`. Best-effort: any write or
/// flush error is ignored (a notification is never worth failing a frame over).
/// Control characters in `message` are neutralised so they cannot break out of
/// the escape sequence.
pub fn desktop_notify(message: &str) {
    let clean: String = message.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    // OSC 9 ; <text> ST  (notification), then BEL (audible fallback).
    let seq = format!("\x1b]9;{clean}\x1b\\\x07");
    let mut out = io::stdout();
    let _ = out.write_all(seq.as_bytes());
    let _ = out.flush();
}
