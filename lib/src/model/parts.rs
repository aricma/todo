use crate::model::{Assignee, Prio, Tag};

/// A task body cut into what this tool recognises.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Parts {
    /// The letter right after the checkbox, if the body opens with one.
    pub prio: Option<Prio>,
    /// The body with the priority token and the tags/assignees set aside, used
    /// only to answer "is there any title text left".
    pub title: String,
    pub tags: Vec<Tag>,
    pub assignees: Vec<Assignee>,
}

#[cfg(test)]
mod tests {
    use super::Parts;

    #[test]
    fn default_is_empty() {
        assert_eq!(
            Parts::default(),
            Parts {
                prio: None,
                title: String::new(),
                tags: Vec::new(),
                assignees: Vec::new(),
            }
        );
    }
}
