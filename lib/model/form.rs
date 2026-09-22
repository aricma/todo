//! A form, as data: the fields a host has to fill in before it can be used.
//!
//! This is not about data_sources. A storage happens to be the first thing the app
//! configures — its endpoint, its credentials — but what is modelled here is a
//! dynamic form: a list of fields with a label, a kind, whether they must have
//! a value, and where that value comes from before the user types
//! ([`FieldValue`]). A host renders a field per entry; whoever owns the form
//! stores the values, never the model.

/// How a field is drawn and validated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    Text,
    Password,
    Url,
    Number,
}

/// Where a field's value comes from when the user has not typed one, and whether
/// they may change it.
///
/// Three ways a form can hold a value nobody typed, and they are not
/// interchangeable: a **placeholder** (on [`Field`]) is a hint and is never
/// submitted; a [`Default`](FieldValue::Default) is what the app expects and the
/// user may replace, so it is rendered and saved; a
/// [`Fixed`](FieldValue::Fixed) value is not the user's to set at all, so it is
/// neither rendered nor overridable — the app writes it. That last one is for
/// what the app knows as a fact (MinIO's region): showing it as a field would be a
/// second source of truth for something no user can change, and hiding it
/// entirely would mean the value has nowhere to live.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldValue {
    /// Nothing: the user supplies it, and `required` says whether they must.
    None,
    /// A starting value the user may change. Saving submits it if they never
    /// touch the field.
    Default(&'static str),
    /// This value, always: the field is not rendered, and every save writes it.
    Fixed(&'static str),
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
    /// input `required`, so the browser enforces it. A `Fixed` value satisfies it
    /// by construction.
    pub required: bool,
    pub placeholder: Option<&'static str>,
    /// The value the user has not typed: nothing, a default they may change, or
    /// the value the app owns.
    pub value: FieldValue,
}
