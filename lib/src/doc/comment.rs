//! Where a file's comments are — the first half of reading a comment todo.
//!
//! This is **not** a parser. It finds the character a language starts a comment with
//! and stops there: strings, escapes, raw strings and nesting are out of scope, and
//! the quote rule in [`super::parse`] is what stands in for them (RFC-019, RFC-020).
//! A real lexer per language is the alternative RFC-016 rejected, and this is the same
//! trade one level down.

use std::path::Path;

/// The characters a file's language starts a comment with.
///
/// Three shapes cover almost every language people write todos in — `//`, `#`, and
/// `--` — and everything else is `Any`, which accepts all of them. A file whose
/// language is not known is read permissively because that is what it is: a notes file
/// writes its todos as bare lines, and refusing them would hide the format that needs
/// no marker at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Syntax {
    /// Every marker, and a line with no marker at all.
    Any,
    /// `//` and `/*`: the C family, and the languages that borrow it.
    Slash,
    /// `#`: shells, scripts, and the config files that follow them.
    Hash,
    /// `--`: SQL, Lua, Haskell, VHDL.
    Dash,
}

impl Syntax {
    /// What a file's own language says a comment looks like.
    ///
    /// The extension decides, because that is what a person's editor decides from. A
    /// file with no extension is read by its name — `Makefile`, `Dockerfile`, and the
    /// dotfiles that are shell fragments.
    pub fn of(path: &Path) -> Self {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_lowercase();
        match extension(&name) {
            Some(
                "rs" | "js" | "mjs" | "cjs" | "jsx" | "ts" | "tsx" | "go" | "c" | "h" | "cc"
                | "cpp" | "cxx" | "hpp" | "hh" | "java" | "kt" | "kts" | "swift" | "cs" | "scala"
                | "dart" | "php" | "proto" | "zig" | "v" | "groovy" | "sol" | "cu",
            ) => Self::Slash,
            Some(
                "py" | "pyi" | "sh" | "bash" | "zsh" | "fish" | "rb" | "pl" | "pm" | "r" | "nix"
                | "nim" | "ex" | "exs" | "jl" | "cr" | "ps1" | "yaml" | "yml" | "toml" | "ini"
                | "cfg" | "conf" | "mk",
            ) => Self::Hash,
            Some("sql" | "lua" | "hs" | "elm" | "ada" | "adb" | "ads" | "vhd" | "vhdl") => {
                Self::Dash
            }
            _ => match name.as_str() {
                "makefile" | "gnumakefile" | "dockerfile" | "containerfile" | "gemfile"
                | "rakefile" | "vagrantfile" | "justfile" | "procfile" => Self::Hash,
                // an extensionless dotfile is a shell fragment — `.gitignore`, `.env` —
                // where `.eslintrc.json` is a file with an extension and is not
                _ if name.starts_with('.') && !name.trim_start_matches('.').contains('.') => {
                    Self::Hash
                }
                _ => Self::Any,
            },
        }
    }

    /// The characters this syntax starts a comment with.
    fn markers(self) -> &'static str {
        match self {
            Self::Any => "#/-;%*",
            Self::Slash => "/",
            Self::Hash => "#",
            Self::Dash => "-",
        }
    }

    /// Whether a line with no marker at all can be a comment. True only where the
    /// language is unknown: a notes file writes its todos as bare lines.
    fn bare(self) -> bool {
        self == Self::Any
    }
}

/// The extension of a lowercased file name, when it has one worth reading.
///
/// A leading dot is part of the name and not an extension: `.env` has none, and
/// `.eslintrc.json` has `json`.
fn extension(name: &str) -> Option<&str> {
    let (stem, extension) = name.rsplit_once('.')?;
    (!stem.is_empty()).then_some(extension)
}

/// One line's comment, as far as this tool needs to know it: which line, and where on
/// that line the comment starts.
///
/// A region with an end would be needed for a block comment, and a block comment is not
/// read yet (RFC-020 leaves it open): a todo is one line, and `at` is enough for one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Comment {
    /// Zero-based index into the file's lines.
    pub line: usize,
    /// Byte offset of the marker on that line — `0` for a bare line.
    pub at: usize,
}

/// Every line that carries a comment, in file order.
pub fn comments(lines: &[String], syntax: Syntax) -> Vec<Comment> {
    lines
        .iter()
        .enumerate()
        .filter_map(|(line, text)| {
            Some(Comment {
                line,
                at: comment_at(text, syntax)?,
            })
        })
        .collect()
}

/// Where the comment on this line starts, when the line carries one.
///
/// The first character the syntax starts a comment with — not a whole marker and not a
/// word — because finding `//` where a `/` already is is a second rule to keep in step
/// with the first. Nothing about strings is decided here.
fn comment_at(line: &str, syntax: Syntax) -> Option<usize> {
    let markers = syntax.markers();
    line.char_indices()
        .find(|(_, character)| markers.contains(*character))
        .map(|(at, _)| at)
        .or_else(|| syntax.bare().then_some(0))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{comments, Syntax};

    fn at(path: &str) -> Option<usize> {
        let syntax = Syntax::of(Path::new(path));
        comments(&["// TODO: x".to_owned()], syntax)
            .first()
            .map(|comment| comment.at)
    }

    #[test]
    fn the_c_family_reads_a_double_slash() {
        for path in ["a.rs", "a.ts", "a.c", "a.hpp", "a.java", "a.swift", "a.kt"] {
            assert_eq!(at(path), Some(0), "{path}");
        }
    }

    #[test]
    fn shells_and_scripts_read_a_hash() {
        for path in [
            "a.py",
            "a.sh",
            "a.rb",
            "a.yml",
            "a.toml",
            "Makefile",
            ".gitignore",
        ] {
            assert_eq!(Syntax::of(Path::new(path)), Syntax::Hash, "{path}");
        }
        assert_eq!(Syntax::of(Path::new("a.lua")), Syntax::Dash, "lua");
        assert_eq!(Syntax::of(Path::new("a.txt")), Syntax::Any, "a notes file");
        assert_eq!(Syntax::of(Path::new("TODOs")), Syntax::Any, "no extension");
        assert_eq!(Syntax::of(Path::new(".eslintrc.json")), Syntax::Any, "json");
    }

    #[test]
    fn a_hash_in_a_rust_file_is_not_a_comment_and_the_slash_still_is() {
        let rust = Syntax::of(Path::new("a.rs"));
        assert_eq!(comments(&["#[derive(Debug)]".to_owned()], rust).len(), 0);
        assert_eq!(
            comments(&["fn f() {} // TODO: x".to_owned()], rust)[0].at,
            10
        );
        // the same line in a shell script is a hash comment, and a bash one it is
        let shell = Syntax::of(Path::new("a.sh"));
        assert_eq!(comments(&["# TODO: x".to_owned()], shell)[0].at, 0);
    }

    #[test]
    fn a_bare_line_is_a_comment_only_where_the_language_is_unknown() {
        let bare = ["TODO: x".to_owned()];
        assert_eq!(comments(&bare, Syntax::Any)[0].at, 0);
        assert!(comments(&bare, Syntax::Slash).is_empty());
    }

    #[test]
    fn every_comment_on_the_file_is_returned_in_order() {
        let lines = [
            "fn main() {".to_owned(),
            "    // TODO: one".to_owned(),
            "    let x = 1;".to_owned(),
            "    // TODO: two".to_owned(),
            "}".to_owned(),
        ];
        let found = comments(&lines, Syntax::Slash);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].line, 1);
        assert_eq!(found[1].line, 3);
    }
}
