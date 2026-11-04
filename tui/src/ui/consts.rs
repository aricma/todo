//! The numbers the screen is made of: sizes, weights, and its one accent colour.
//!
//! Every value a layout or a renderer needs is named here, so "why 38?" has one
//! answer in one place.

use ratatui::style::{Color, Modifier, Style};

use crate::state::message::Tone;

/// The line at the top: the app's name and the tree it is open on.
pub const HEADER_ROWS: u16 = 1;
/// Status bar rows: the general keys, then whoever has the keyboard.
pub const STATUS_ROWS: u16 = 2;
/// The sidebar takes a bit more than a third: the file list needs the width, and
/// the view reads fine in the rest.
pub const SIDEBAR_PERCENT: u16 = 38;
pub const VIEW_PERCENT: u16 = 62;
/// Between the two columns: the same gap a stack of sections leaves.
pub const COLUMN_GAP: u16 = 2;
/// What a section asks for, in units normalised over the sections on screen.
pub const FILES_WEIGHT: u32 = 34;
pub const TODOS_WEIGHT: u32 = 45;
pub const SIDE_WEIGHT: u32 = 33;
/// The selection moves through the top third of a panel before the rest scrolls.
pub const SCROLL_MARGIN_NUM: usize = 3;
pub const SCROLL_MARGIN_DEN: usize = 10;
/// A path in a bar, cut from the front so the file name survives.
pub const SHORT_PATH: usize = 20;
/// A popover is half the screen wide — the add field is narrower than that, because
/// a line of text does not need half a terminal and what does not fit wraps.
pub const POPOVER_HALF: u16 = 2;
pub const ADD_MAX_WIDTH: u16 = 60;
pub const POPOVER_MIN_WIDTH: u16 = 30;
pub const POPOVER_THIRD: u16 = 3;
/// The search popover is wider than the others — a list of results, not a line —
/// and taller, because it is the only popover with something to read.
pub const SEARCH_WIDTH_NUM: u16 = 3;
pub const SEARCH_WIDTH_DEN: u16 = 4;
pub const SEARCH_MIN_WIDTH: u16 = 20;
pub const SEARCH_HEIGHT_NUM: u16 = 2;
pub const SEARCH_HEIGHT_DEN: u16 = 3;
pub const SEARCH_MIN_HEIGHT: u16 = 5;
/// The keys page is a reference, not a newspaper: three columns stop getting
/// wider than this.
pub const KEYS_MAX_WIDTH: u16 = 100;
/// Three columns of keys are one wall of text without a gap between them, and one
/// column looks like part of the words on either side.
pub const KEYS_COLUMN_GAP: u16 = 2;
/// One row of air under the title of the keys page.
pub const KEYS_PAD_ROWS: u16 = 1;
/// One more column of air each side, on top of the frame's own padding: a page of
/// keys is read column by column, and the first one should not touch the border.
pub const KEYS_PAD_COLS: u16 = 1;
/// A popover's frame: a top border and a bottom one, which is what its height is
/// counted in.
pub const POPOVER_BORDER_ROWS: u16 = 2;
/// One row of text: the line typed into a field.
pub const TEXT_ROWS: u16 = 1;
/// Never eat a list to say something about it.
pub const MIN_LIST_ROWS: u16 = 3;
/// Nothing wraps into a column narrower than this; it is unreadable before then.
pub const MIN_WRAP: usize = 8;
/// One column of padding inside a frame, so text never touches the border.
pub const PAD: u16 = 1;
/// The accent: the frame of whatever has the cursor.
pub const ACCENT: Color = Color::Cyan;
/// The selection: a light band with dark text.
///
/// A tinted band was tried and was not enough — on a dark terminal a dark green
/// reads as "slightly different", not "this is the row". Pure white is what reverse
/// video gives you and it glares on every arrow key, so this is almost white:
/// `Rgb(210, 210, 210)` with black text. One idea, everywhere — a heading, a file,
/// a history entry and a todo all wear it.
pub const SELECTED: Color = Color::Rgb(210, 210, 210);
pub const SELECTED_TEXT: Color = Color::Black;

/// What being the selected row looks like, text included.
pub fn selection() -> Style {
    Style::default().fg(SELECTED_TEXT).bg(SELECTED)
}

/// A name under the band: its own colour behind it and white on top, so a tag stays
/// cyan and a person stays magenta even on the selected row.
///
/// Borrowed from the palette rather than chosen here: the highlight and the names
/// must not disagree about what a tag looks like, and the sigil alone is too small a
/// difference to find at a glance.
pub fn selection_name(colour: Color) -> Style {
    Style::default().fg(Color::White).bg(colour)
}

/// A rule being explained: not red, because nothing failed, and not yellow, which
/// is the colour of a highlight.
pub const WARNING: Color = Color::Rgb(255, 165, 0);
/// Red is kept for a write that did not happen, and for nothing else.
pub const ERROR: Color = Color::Red;
/// A soft green: read on a light terminal as well as a dark one, which `Green`
/// alone is not.
pub const SUCCESS: Color = Color::Rgb(80, 200, 120);

/// How a message is drawn: its tone's colour, in bold, because it is one line in a
/// panel of dim text. Information is neither, so it is drawn as ordinary text —
/// nothing is wrong, and a colour would say something is.
/// What stands in front of a message: `!` marks something the reader has to deal
/// with, and information gets no mark at all.
pub fn message_prefix(tone: Tone) -> &'static str {
    match tone {
        // nothing worked and nothing failed: what is left is worth knowing
        Tone::Info | Tone::Success => "",
        Tone::Warning | Tone::Error => "! ",
    }
}

pub fn message_style(tone: Tone) -> Style {
    let style = Style::default().add_modifier(Modifier::BOLD);
    match tone {
        Tone::Info => style,
        Tone::Warning => style.fg(WARNING),
        Tone::Error => style.fg(ERROR),
        Tone::Success => style.fg(SUCCESS),
    }
}
