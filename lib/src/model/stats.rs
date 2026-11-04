/// How many tasks a file holds, how many of them are done, and how many distinct
/// tags and people it mentions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub all: usize,
    pub done: usize,
    /// Distinct `#tags`, counted once however often they are written.
    pub tags: usize,
    /// Distinct `@people`, counted once.
    pub assignees: usize,
}

#[cfg(test)]
mod tests {
    use super::Stats;

    #[test]
    fn default_is_empty() {
        assert_eq!(
            Stats::default(),
            Stats {
                all: 0,
                done: 0,
                tags: 0,
                assignees: 0
            }
        );
    }
}
