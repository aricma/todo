//! Metadata-key repair for imports. OPS constrains metadata keys to
//! `^[a-z0-9_]{3,}$` and `ops-lib` rejects any document that breaks it, but
//! foreign files routinely carry keys that do (`Task Name`, `Due-Date`,
//! `priorityLevel`). Before reading we slugify the offending keys: rename-only,
//! deterministic and document-wide (the same key gets the same slug in every
//! row), with every rename reported so the UI can say what it fixed.
//!
//! Only top-level metadata keys are touched — the spec constrains only those;
//! keys nested inside a metadata *value* are data and ride along verbatim.

use ops_lib::v0::Format;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// The reserved CSV columns, which are never metadata keys ([`specs/v0/formats/csv.md`]).
const RESERVED: [&str; 7] = [
    "version", "id", "title", "status", "notes", "subtasks", "metadata",
];

const TYPE_SUFFIXES: [&str; 4] = ["_json", "_string", "_number", "_bool"];

/// One rename applied to the document, for the import notice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repair {
    pub from: String,
    pub to: String,
}

impl Repair {
    pub fn describe(&self) -> String {
        format!("`{}` → `{}`", self.from, self.to)
    }
}

/// OPS rule 5: keys match `^[a-z0-9_]{3,}$`.
pub(super) fn valid_key(key: &str) -> bool {
    key.len() >= 3
        && key
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// A key that fits the OPS charset, derived deterministically from one that
/// does not. Lowercased, every other character becomes `_`, runs collapse,
/// ends trim, and a short key is padded: `id` → `_id`.
fn slug(key: &str) -> String {
    let mut out = String::new();
    let mut underscore = false;
    for c in key.chars() {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            out.push(c);
            underscore = false;
        } else if !underscore {
            out.push('_');
            underscore = true;
        }
    }
    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        return "key".to_string();
    }
    let mut slug = trimmed.to_string();
    while slug.len() < 3 {
        slug.insert(0, '_');
    }
    slug
}

/// The document-wide mapping from invalid keys to their slugs. Keys already in
/// the charset are reserved and never renamed; two keys landing on the same
/// slug are disambiguated with `_2`, `_3`, ….
fn build_mapping<'a>(
    keys: impl IntoIterator<Item = &'a str>,
) -> (BTreeMap<String, String>, Vec<Repair>) {
    let keys: BTreeSet<&str> = keys.into_iter().collect();
    let mut taken: BTreeSet<String> = keys
        .iter()
        .filter(|key| valid_key(key))
        .map(|key| key.to_string())
        .collect();

    let mut mapping = BTreeMap::new();
    let mut repairs = Vec::new();
    for key in &keys {
        if valid_key(key) {
            continue;
        }
        let base = slug(key);
        let mut candidate = base.clone();
        let mut n = 2;
        while taken.contains(&candidate) {
            candidate = format!("{base}_{n}");
            n += 1;
        }
        taken.insert(candidate.clone());
        repairs.push(Repair {
            from: key.to_string(),
            to: candidate.clone(),
        });
        mapping.insert(key.to_string(), candidate);
    }
    (mapping, repairs)
}

/// Repair the document in `format`, returning the text to hand to `ops-lib`
/// and the renames applied. A document whose keys are all valid comes back
/// byte-identical.
pub fn repair(format: Format, text: &str) -> Result<(String, Vec<Repair>), String> {
    match format {
        Format::Json => repair_json(text),
        Format::Jsonl => repair_jsonl(text),
        Format::Yaml => repair_yaml(text),
        Format::Csv => repair_csv(text),
    }
}

// ── JSON-family (also JSONL lines and YAML, which all reduce to a Value) ────

/// Collect every top-level metadata key in a task tree (JSON/doc shape).
fn collect_keys(value: &Value, out: &mut BTreeSet<String>) {
    match value {
        Value::Object(task) => {
            if let Some(Value::Object(metadata)) = task.get("metadata") {
                out.extend(metadata.keys().cloned());
            }
            if let Some(Value::Array(subtasks)) = task.get("subtasks") {
                for subtask in subtasks {
                    collect_keys(subtask, out);
                }
            }
        }
        Value::Array(tasks) => {
            for task in tasks {
                collect_keys(task, out);
            }
        }
        _ => {}
    }
}

/// Rename metadata keys in a task tree, in place.
fn apply_keys(value: &mut Value, mapping: &BTreeMap<String, String>) {
    match value {
        Value::Object(task) => {
            if let Some(Value::Object(metadata)) = task.get_mut("metadata") {
                let old = std::mem::take(metadata);
                for (key, item) in old {
                    let key = mapping.get(&key).cloned().unwrap_or(key);
                    metadata.insert(key, item);
                }
            }
            if let Some(Value::Array(subtasks)) = task.get_mut("subtasks") {
                for subtask in subtasks {
                    apply_keys(subtask, mapping);
                }
            }
        }
        Value::Array(tasks) => {
            for task in tasks {
                apply_keys(task, mapping);
            }
        }
        _ => {}
    }
}

fn repair_value(mut value: Value, original: &str) -> Result<(String, Vec<Repair>), String> {
    let mut keys = BTreeSet::new();
    collect_keys(&value, &mut keys);
    let (mapping, repairs) = build_mapping(keys.iter().map(String::as_str));
    if repairs.is_empty() {
        return Ok((original.to_string(), repairs));
    }
    apply_keys(&mut value, &mapping);
    let text = serde_json::to_string(&value).map_err(|e| e.to_string())?;
    Ok((text, repairs))
}

fn repair_json(text: &str) -> Result<(String, Vec<Repair>), String> {
    let value: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    repair_value(value, text)
}

fn repair_jsonl(text: &str) -> Result<(String, Vec<Repair>), String> {
    let mut values = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            values.push(None);
        } else {
            values.push(Some(
                serde_json::from_str::<Value>(line).map_err(|e| e.to_string())?,
            ));
        }
    }
    let mut keys = BTreeSet::new();
    for value in values.iter().flatten() {
        collect_keys(value, &mut keys);
    }
    let (mapping, repairs) = build_mapping(keys.iter().map(String::as_str));
    if repairs.is_empty() {
        return Ok((text.to_string(), repairs));
    }
    let mut lines = Vec::with_capacity(values.len());
    for value in &mut values {
        match value {
            Some(value) => {
                apply_keys(value, &mapping);
                lines.push(serde_json::to_string(value).map_err(|e| e.to_string())?);
            }
            None => lines.push(String::new()),
        }
    }
    let mut out = lines.join("\n");
    if text.ends_with('\n') {
        out.push('\n');
    }
    Ok((out, repairs))
}

fn repair_yaml(text: &str) -> Result<(String, Vec<Repair>), String> {
    // YAML is a superset of JSON, so a repaired document can go back as JSON.
    let value: Value = serde_yaml::from_str(text).map_err(|e| e.to_string())?;
    let mut keys = BTreeSet::new();
    collect_keys(&value, &mut keys);
    let (mapping, repairs) = build_mapping(keys.iter().map(String::as_str));
    if repairs.is_empty() {
        return Ok((text.to_string(), repairs));
    }
    let mut value = value;
    apply_keys(&mut value, &mapping);
    let out = serde_json::to_string(&value).map_err(|e| e.to_string())?;
    Ok((out, repairs))
}

// ── CSV: repair the header, leave the rows ─────────────────────────────────

/// How a header cell names a metadata key: the key, whether it was written
/// `metadata_`-prefixed, and the type suffix it carried.
struct MetadataColumn {
    key: String,
    prefixed: bool,
    suffix: Option<&'static str>,
}

/// One header cell: a reserved model column, or a metadata column.
enum Header {
    Reserved(String),
    Metadata(MetadataColumn),
}

fn classify_header(cell: &str) -> Header {
    if RESERVED.contains(&cell) {
        return Header::Reserved(cell.to_string());
    }
    let (key, prefixed, suffix) = match cell.strip_prefix("metadata_") {
        Some(rest) => match TYPE_SUFFIXES.iter().find(|s| rest.ends_with(**s)) {
            Some(suffix) => (&rest[..rest.len() - suffix.len()], true, Some(*suffix)),
            None => (rest, true, None),
        },
        None => (cell, false, None),
    };
    Header::Metadata(MetadataColumn {
        key: key.to_string(),
        prefixed,
        suffix,
    })
}

/// The column name for a (possibly renamed) key, keeping the shape it was
/// written with: `metadata_` stays when it was there or when the bare key
/// would be misread as a reserved column.
fn column_name(column: &MetadataColumn, key: &str) -> String {
    let needs_prefix = column.prefixed || RESERVED.contains(&key) || key.starts_with("metadata_");
    let mut name = String::new();
    if needs_prefix {
        name.push_str("metadata_");
    }
    name.push_str(key);
    if let Some(suffix) = column.suffix {
        name.push_str(suffix);
    }
    name
}

fn repair_csv(text: &str) -> Result<(String, Vec<Repair>), String> {
    let (header, consumed) = read_first_record(text)?;
    if header.is_empty() {
        return Err("empty csv: the header row is required".to_string());
    }

    let mut fields: Vec<Header> = Vec::with_capacity(header.len());
    let mut keys = BTreeSet::new();
    for cell in &header {
        // A header the exporter padded is still the field it names.
        let field = classify_header(cell.trim());
        if let Header::Metadata(column) = &field {
            keys.insert(column.key.clone());
        }
        fields.push(field);
    }
    let (mapping, repairs) = build_mapping(keys.iter().map(String::as_str));
    if repairs.is_empty() {
        return Ok((text.to_string(), repairs));
    }

    let names: Vec<String> = fields
        .iter()
        .map(|field| match field {
            Header::Metadata(column) => {
                let key = mapping
                    .get(&column.key)
                    .map(String::as_str)
                    .unwrap_or(&column.key);
                column_name(column, key)
            }
            Header::Reserved(reserved) => reserved.clone(),
        })
        .collect();

    let terminator = if text[..consumed].ends_with('\n') {
        if text[..consumed].ends_with("\r\n") {
            "\r\n"
        } else {
            "\n"
        }
    } else {
        ""
    };
    let head = format!(
        "{}{terminator}",
        names
            .iter()
            .map(|name| csv_cell(name))
            .collect::<Vec<_>>()
            .join(",")
    );
    Ok((head + &text[consumed..], repairs))
}

/// One RFC 4180 record from the start of `input`, with its byte length. The
/// header is the only record repair rewrites.
fn read_first_record(input: &str) -> Result<(Vec<String>, usize), String> {
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut started = false;
    let mut chars = input.char_indices().peekable();
    let mut end = input.len();
    let mut saw_newline = false;

    while let Some((i, c)) = chars.next() {
        if quoted {
            if c == '"' {
                if chars.peek().map(|(_, c)| *c) == Some('"') {
                    chars.next();
                    field.push('"');
                } else {
                    quoted = false;
                }
            } else {
                field.push(c);
            }
            continue;
        }
        match c {
            '"' if field.is_empty() => {
                quoted = true;
                started = true;
            }
            ',' => {
                fields.push(std::mem::take(&mut field));
                started = true;
            }
            '\n' => {
                if started || !field.is_empty() || !fields.is_empty() {
                    fields.push(std::mem::take(&mut field));
                }
                end = i + c.len_utf8();
                saw_newline = true;
                break;
            }
            '\r' => {}
            _ => {
                field.push(c);
                started = true;
            }
        }
    }
    if quoted {
        return Err("unterminated quoted field in the csv header".to_string());
    }
    if !saw_newline && (started || !field.is_empty() || !fields.is_empty()) {
        fields.push(field);
    }
    Ok((fields, end))
}

fn csv_cell(text: &str) -> String {
    if text.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_string()
    }
}
