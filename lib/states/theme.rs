//! Theme preference and the palette it resolves to (DESIGN.md §8, §11) — the
//! pure half.
//!
//! `ThemePref` is what the user chose and `Theme` is what it resolves to;
//! `resolve` is the only rule between them. System follows the OS, which the
//! caller reports as a `bool`, so the decision is testable away from a
//! browser. Everything that touches one — localStorage, `matchMedia`,
//! `<html data-theme>` — is an app side effect (`app/side_effects/theme.rs`),
//! and the *first* paint is decided before either runs: `index.html` carries
//! the same three-value resolution inline so a dark-OS visit doesn't flash the
//! light canvas while the wasm bundle loads. Change the stored values and that
//! inline script needs the same edit.
//!
//! System is resolved here rather than in CSS: the bundle stays "light by
//! default, `[data-theme="dark"]` for the dark scale", so a consumer that
//! never opts in (a static demo, the storylite canvas) can't be flipped by an
//! OS setting.

/// The palette actually applied to the document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    Light,
    Dark,
}

/// What the user chose. `System` follows the OS preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemePref {
    Light,
    Dark,
    System,
}

impl Theme {
    /// Whether this palette is the dark one (drives the sun/moon icon).
    pub fn is_dark(self) -> bool {
        matches!(self, Theme::Dark)
    }

    /// `<html data-theme>` value for this palette. `None` means remove the
    /// attribute, which is the bundle's light default.
    pub fn data_theme_attr(self) -> Option<&'static str> {
        match self {
            Theme::Light => None,
            Theme::Dark => Some("dark"),
        }
    }
}

impl ThemePref {
    /// The stored value for this preference (also the picker tile's `value`).
    pub fn key(self) -> &'static str {
        match self {
            ThemePref::Light => "light",
            ThemePref::Dark => "dark",
            ThemePref::System => "system",
        }
    }

    /// [`key`](Self::key) inverse — unknown values are None (garbage in
    /// storage).
    pub fn from_key(s: &str) -> Option<ThemePref> {
        ThemePref::ALL.iter().copied().find(|p| p.key() == s)
    }

    /// Every preference, in display order.
    pub const ALL: [ThemePref; 3] = [ThemePref::Light, ThemePref::Dark, ThemePref::System];

    /// The preference behind a raw stored value: `System` when nothing (or
    /// garbage) is stored, so a first visit follows the OS without locking a
    /// choice in that the user never made.
    pub fn from_stored(value: Option<&str>) -> ThemePref {
        value
            .and_then(ThemePref::from_key)
            .unwrap_or(ThemePref::System)
    }

    /// The palette this preference resolves to, given whether the OS prefers
    /// dark. An explicit choice ignores the OS entirely (that is the whole
    /// difference between `Dark` and `System`).
    pub fn resolve(self, os_dark: bool) -> Theme {
        match self {
            ThemePref::Light => Theme::Light,
            ThemePref::Dark => Theme::Dark,
            ThemePref::System => {
                if os_dark {
                    Theme::Dark
                } else {
                    Theme::Light
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preference_round_trips_through_its_stored_key() {
        for pref in ThemePref::ALL {
            assert_eq!(ThemePref::from_key(pref.key()), Some(pref));
        }
    }

    #[test]
    fn garbage_in_storage_means_system() {
        assert_eq!(ThemePref::from_stored(None), ThemePref::System);
        assert_eq!(ThemePref::from_stored(Some("")), ThemePref::System);
        assert_eq!(ThemePref::from_stored(Some("solarized")), ThemePref::System);
        assert_eq!(ThemePref::from_stored(Some("dark")), ThemePref::Dark);
    }

    #[test]
    fn an_explicit_choice_ignores_the_operating_system() {
        assert_eq!(ThemePref::Light.resolve(true), Theme::Light);
        assert_eq!(ThemePref::Dark.resolve(false), Theme::Dark);
    }

    #[test]
    fn system_follows_the_operating_system() {
        assert_eq!(ThemePref::System.resolve(true), Theme::Dark);
        assert_eq!(ThemePref::System.resolve(false), Theme::Light);
    }

    /// The attribute is what `index.html`'s first-paint script writes by hand:
    /// light is the absence of the attribute, dark is `"dark"`.
    #[test]
    fn the_attribute_names_the_dark_palette_only() {
        assert_eq!(Theme::Light.data_theme_attr(), None);
        assert_eq!(Theme::Dark.data_theme_attr(), Some("dark"));
        assert!(Theme::Dark.is_dark());
        assert!(!Theme::Light.is_dark());
    }
}
