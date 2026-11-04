//! What a history row says.

use aricma_todo_lib::model::Task;

/// A row names what was done, then what it was done to: `check · pay rent`.
///
/// The whole line would be unreadable in the sidebar, and the state is already the
/// checkbox in front of it. A todo with no title — a bare checkbox — is named by
/// its body, which is all there is.
pub fn label(action: &str, task: &Task) -> String {
    let subject = if task.title.trim().is_empty() {
        &task.body
    } else {
        &task.title
    };
    format!("{action} · {subject}")
}
