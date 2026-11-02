/// One recognisable piece of a task body, in body order.
///
/// Produced by `doc::spans::segments` and used only for colouring, so what is
/// highlighted is exactly what the parser saw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Segment {
    Prio,
    Title,
    Tag,
    Assignee,
}
