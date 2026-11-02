//! The key hints the status bar shows: which section is active, then the keys
//! that work right now — navigation first, then what the section itself offers.

use super::super::action::Pane;
use super::super::App;

/// Where the cursor is, and the keys every section shares: `Tab` stays in the
/// sidebar, the sideways arrows cross to the file view and back.
///
/// The sidebar keys name their destination — `tab outline`, not `tab section` —
/// because which panel `tab` reaches is the thing a person cannot guess.
pub fn where_and_navigation(app: &App) -> String {
    let mut parts = vec![format!("▸ {}", app.pane.name())];
    if let Some(next) = app.focus_target(true) {
        parts.push(format!("tab {}", next.name()));
    }
    if let Some(previous) = app.focus_target(false) {
        parts.push(format!("shift+tab {}", previous.name()));
    }
    if app.pane == Pane::View {
        parts.push(format!("← {}", app.last_sidebar.name()));
    } else if !app.single {
        // name the file the arrow would put you in, not the word "view"
        parts.push(format!(
            "→ {}",
            short(&app.display(&app.current().path), 20)
        ));
    }
    parts.push("r reload".to_owned());
    parts.push("q quit".to_owned());
    parts.join(" · ")
}

/// A path that fits a status bar, cut from the *front*: the file name is the
/// part that says which file it is, so `…/nested/place/TODOs.md` beats
/// `some/deeply/nested…`.
fn short(text: &str, width: usize) -> String {
    use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

    if UnicodeWidthStr::width(text) <= width {
        return text.to_owned();
    }
    let budget = width.saturating_sub(1); // one column for the leading ellipsis
    let mut kept: Vec<char> = Vec::new();
    let mut used = 0;
    for character in text.chars().rev() {
        let character_width = UnicodeWidthChar::width(character).unwrap_or(0);
        if used + character_width > budget {
            break;
        }
        kept.push(character);
        used += character_width;
    }
    let mut out = String::from("…");
    out.extend(kept.into_iter().rev());
    out
}

/// What this section adds to the keys above.
pub fn section_keys(pane: Pane) -> &'static [&'static str] {
    match pane {
        Pane::Files => &["↑/↓ file"],
        Pane::Outline => &["↑/↓ heading"],
        Pane::View | Pane::Todos => &[
            "↑/↓ todo",
            "x/space check",
            "d delete",
            "a add",
            "e edit",
            "A-D prio (again clears)",
        ],
        Pane::History => &["↑/↓ entry", "x/space undo/redo", "d forget"],
    }
}

#[cfg(test)]
mod tests {
    use crate::action::Pane;

    #[test]
    fn the_navigation_names_where_it_goes() {
        let (_tmp, mut app) =
            crate::testing::app(&[("TODOs.md", "# one\n\n- [ ] a\n\n## two\n\n- [ ] b\n")]);
        let line = super::where_and_navigation(&app);
        assert!(line.contains("tab outline"), "{line}");
        assert!(line.contains("shift+tab history"), "{line}");
        assert!(line.contains("→ TODOs.md"), "{line}");

        app.pane = Pane::Outline;
        let line = super::where_and_navigation(&app);
        assert!(line.contains("tab history"), "{line}");
        assert!(line.contains("shift+tab files"), "{line}");

        app.pane = Pane::View;
        let line = super::where_and_navigation(&app);
        assert!(line.contains("← files"), "{line}");
        assert!(!line.contains("→ "), "{line}");
        assert!(
            !line.contains("tab"),
            "the view is not in the sidebar, so tab does nothing here: {line}"
        );
    }

    #[test]
    fn the_arrow_hint_with_a_deep_path_reads_as_the_file() {
        let (_tmp, mut app) = crate::testing::app(&[
            ("a/deeply/nested/place/where/TODOs.md", "- [ ] a\n"),
            ("b/TASKS.md", "- [ ] b\n"),
        ]);
        app.pane = Pane::Files;
        let line = super::where_and_navigation(&app);
        assert!(line.contains("→ …"), "{line}");
        assert!(line.contains("TODOs.md"), "{line}");
        assert!(!line.contains("a/deeply/nested"), "{line}");
    }

    #[test]
    fn the_sideways_arrows_name_the_panel_they_reach() {
        let (_tmp, mut app) =
            crate::testing::app(&[("a/TODOs.md", "- [ ] a\n"), ("b/TASKS.md", "- [ ] b\n")]);
        let line = super::where_and_navigation(&app);
        assert!(line.contains("→ a/TODOs.md"), "{line}");

        // entering from the outline comes back to the outline
        app.pane = Pane::Outline;
        app.enter_view();
        let line = super::where_and_navigation(&app);
        assert!(line.contains("← outline"), "{line}");
    }

    #[test]
    fn a_long_path_keeps_its_file_name() {
        let short = super::short("some/deeply/nested/place/where/TODOs.md", 20);
        assert!(
            unicode_width::UnicodeWidthStr::width(short.as_str()) <= 20,
            "{short}"
        );
        assert!(short.starts_with('…'), "cut from the front: {short}");
        assert!(
            short.ends_with("TODOs.md"),
            "the file name survives: {short}"
        );
        assert_eq!(super::short("TODOs.md", 20), "TODOs.md");
        assert_eq!(super::short("a/TODOs.md", 20), "a/TODOs.md");
    }

    #[test]
    fn navigation_and_quit_are_not_repeated_per_section() {
        for pane in [
            Pane::Files,
            Pane::Outline,
            Pane::View,
            Pane::Todos,
            Pane::History,
        ] {
            let hints = super::section_keys(pane);
            assert!(
                hints
                    .iter()
                    .all(|hint| !hint.contains("tab") && !hint.contains("quit")),
                "{hints:?}"
            );
        }
    }

    #[test]
    fn the_history_section_offers_undo_and_not_editing() {
        let hints = super::section_keys(Pane::History);
        assert!(hints.contains(&"x/space undo/redo"), "{hints:?}");
        for absent in ["a add", "d delete", "e edit"] {
            assert!(!hints.contains(&absent), "{absent} in {hints:?}");
        }
    }
}
