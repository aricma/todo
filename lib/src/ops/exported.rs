/// What an export produced, plus the todo lines that could not be exported.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Exported {
    pub text: String,
    /// `path:line` for each task with no title, which OPS refuses.
    pub skipped: Vec<String>,
}
