//! The kitty keyboard protocol: push it on the way in, pop it on the way out.
//!
//! Without it a terminal can only say "some modifier was held" for a handful of
//! keys, and `opt+↑` is undecodable where the terminal sends `ESC ESC [ A` (macOS
//! Terminal.app with option-as-meta): crossterm reads the doubled `ESC` as the Esc
//! key and the rest as letters. With it every key arrives as `CSI code ; modifiers
//! u`, so `opt+↑` is `CSI 1;3A` and arrows and tabs keep their modifiers.
//!
//! The push is unconditional. A terminal that does not know the sequence ignores
//! it, and asking first costs a round trip that sits for two seconds on a pty
//! nobody answers — being ignored is cheaper than waiting.

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};

use ratatui::crossterm::event::{
    DisableBracketedPaste, EnableBracketedPaste, KeyboardEnhancementFlags,
    PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};

/// Unambiguous `esc` and `tab`, and press/release kinds, so a modifier is never
/// guessed and a held key is not typed twice.
const FLAGS: KeyboardEnhancementFlags = KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
    .union(KeyboardEnhancementFlags::REPORT_EVENT_TYPES);

/// Whether we pushed, so only a terminal we told is popped.
static PUSHED: AtomicBool = AtomicBool::new(false);

pub fn push() {
    let mut out = std::io::stdout();
    if ratatui::crossterm::execute!(out, PushKeyboardEnhancementFlags(FLAGS)).is_ok() {
        PUSHED.store(true, Ordering::SeqCst);
    }
    // Bracketed paste, so a copied document arrives as one string instead of a
    // hundred key presses — which in a normal screen would be a hundred keys, some of
    // them `d` and `x` (RFC-014). A terminal that ignores the sequence sends the text
    // as keys, which is what it did before this existed.
    let _ = ratatui::crossterm::execute!(out, EnableBracketedPaste);
    let _ = out.flush();
}

/// Give the keyboard back exactly as it was found.
pub fn pop() {
    if !PUSHED.swap(false, Ordering::SeqCst) {
        return;
    }
    let mut out = std::io::stdout();
    let _ = ratatui::crossterm::execute!(out, DisableBracketedPaste, PopKeyboardEnhancementFlags);
    let _ = out.flush();
}
