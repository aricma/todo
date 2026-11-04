//! Leave the terminal as it was found, even when the panic is not ours.

/// The screen is in raw mode on the alternate buffer; a person whose shell is
/// left that way cannot read the reason they were handed. `restore` only reports
/// to stderr when it cannot, so the chain always reaches the previous hook —
/// which is what actually says what went wrong.
pub fn install() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // the keyboard first: a terminal left in the kitty protocol types nothing
        // recognisable afterwards
        super::enhanced::pop();
        ratatui::restore();
        previous(panic_info);
    }));
}
