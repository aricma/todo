//! Which OPS format a name or a path means.

use std::path::Path;

use ops_lib::v0::Format;

/// The format a `--as` value names, else the one the path's extension names,
/// else JSON.
pub fn format_from(name: &str, path: &Path) -> Format {
    if !name.is_empty() {
        if let Some(format) = named(name) {
            return format;
        }
    }
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default();
    named(extension).unwrap_or(Format::Json)
}

fn named(name: &str) -> Option<Format> {
    let name = if name.eq_ignore_ascii_case("yml") {
        "yaml"
    } else {
        name
    };
    Format::ALL
        .iter()
        .copied()
        .find(|format| format.name().eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use ops_lib::v0::Format;

    use super::format_from;

    #[test]
    fn a_name_wins() {
        assert_eq!(format_from("yaml", Path::new("x.json")), Format::Yaml);
        assert_eq!(format_from("CSV", Path::new("x.json")), Format::Csv);
        assert_eq!(format_from("yml", Path::new("x.csv")), Format::Yaml);
    }

    #[test]
    fn the_extension_is_the_fallback() {
        assert_eq!(format_from("", Path::new("todos.jsonl")), Format::Jsonl);
        assert_eq!(format_from("", Path::new("todos.yaml")), Format::Yaml);
    }

    #[test]
    fn json_is_the_default() {
        assert_eq!(format_from("", Path::new("todos")), Format::Json);
        assert_eq!(format_from("nonsense", Path::new("todos")), Format::Json);
    }
}
