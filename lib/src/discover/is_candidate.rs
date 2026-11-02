/// Whether a file name is one this tool looks at: `TODO*`/`TASK*` markdown, or
/// `README.md`. Whether the file actually holds a task is a separate question.
pub fn is_candidate(name: &str) -> bool {
    let lower = name.to_lowercase();
    let markdown = std::path::Path::new(&lower)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"));
    markdown && (lower.starts_with("todo") || lower.starts_with("task") || lower == "readme.md")
}

#[cfg(test)]
mod tests {
    use super::is_candidate;

    #[test]
    fn accepts_todo_and_task_prefixes_in_any_case() {
        for name in [
            "TODOs.md",
            "todos.md",
            "TODO.md",
            "Tasks.md",
            "tasks-2026.md",
        ] {
            assert!(is_candidate(name), "{name}");
        }
    }

    #[test]
    fn accepts_a_readme_only_as_readme_md() {
        assert!(is_candidate("README.md"));
        assert!(is_candidate("readme.md"));
        assert!(!is_candidate("README.markdown"));
    }

    #[test]
    fn rejects_everything_else() {
        for name in ["notes.md", "plan.md", "TODOs.txt", "TODOs", "my-todos.md"] {
            assert!(!is_candidate(name), "{name}");
        }
    }
}
