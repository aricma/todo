//! A form, as data: the fields a host has to fill in before it can be used.
//!
//! This is not about data_sources. A storage happens to be the first thing the app
//! configures — its endpoint, its credentials — but what is modelled here is a
//! dynamic form: a list of fields with a label, a kind and whether they must have
//! a value. A host renders a field per entry; whoever owns the form stores the
//! values, never the model.

/// How a field is drawn and validated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    Text,
    Password,
    Url,
    Number,
}

/// One field of a form. A host renders one per entry; the values are the caller's
/// to store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    /// Stable key the value is stored under (`endpoint`, `secret_key`, …).
    pub key: &'static str,
    pub label: &'static str,
    /// One line of help rendered under the input. Empty means no help.
    pub description: &'static str,
    pub kind: FieldKind,
    /// Whether the form is incomplete until this has a value. The view marks the
    /// input `required`, so the browser enforces it.
    pub required: bool,
    pub placeholder: Option<&'static str>,
    /// The value a fresh form starts with, and the one saved when the user
    /// leaves the field alone. Empty means blank. A placeholder only hints at
    /// the shape; this is real data the form submits.
    pub default: &'static str,
}
