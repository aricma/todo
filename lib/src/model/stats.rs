/// How many tasks a file holds, and how many of them are done.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub all: usize,
    pub done: usize,
}

#[cfg(test)]
mod tests {
    use super::Stats;

    #[test]
    fn default_is_empty() {
        assert_eq!(Stats::default(), Stats { all: 0, done: 0 });
    }
}
