//! Undoing a write, or putting it back on: the same edit, derived from the line as
//! it reads *now*.

use aricma_todo_lib::doc::Doc;
use aricma_todo_lib::model::Inverse;

use super::entry::{change_of, task_of, Change};
use crate::text;

/// Put a write back (`undo`) or back on, deriving the opposite from the line as it
/// reads *now*. A change to the checkbox, the priority or the body does not depend
/// on the text around it, so an unrelated write in between cannot block it — while
/// the part of the line this write owns still has to read as it left it.
pub fn retrace(doc: &mut Doc, forward: &Inverse, undo: bool) -> Result<Inverse, &'static str> {
    const MOVED: &str = text::LINE_MOVED;
    let Inverse::Set { line, expect, text } = forward else {
        // an insert or a delete is anchored to its own text, so it replays
        // literally: undoing applies its opposite, and redoing applies the write
        // itself. Flipping both ways would insert a second copy of a deleted line.
        let apply = if undo {
            forward.clone().flip()
        } else {
            forward.clone()
        };
        return doc.apply(apply.clone()).map(|()| apply).map_err(|_| MOVED);
    };
    let (Some(before), Some(after)) = (task_of(expect), task_of(text)) else {
        return Err(MOVED);
    };
    let Some(change) = change_of(&before, &after) else {
        return Err(MOVED);
    };
    let Some(now) = doc.lines().get(*line).and_then(|text| task_of(text)) else {
        return Err(MOVED);
    };
    // what the line should read now, and what this write should put back on it
    let (expected, target) = if undo {
        (&after, &before)
    } else {
        (&before, &after)
    };
    let inverse = match &change {
        Change::Done { from, to } => {
            let left = if undo { to } else { from };
            if now.done != *left || now.title != expected.title {
                return Err(MOVED);
            }
            // a checkbox is its own opposite, so one call does both directions
            doc.check(*line)
        }
        Change::Prio { from, to } => {
            let left = if undo { to } else { from };
            if &now.prio != left || now.title != expected.title {
                return Err(MOVED);
            }
            doc.set_prio(*line, target.prio)
        }
        Change::Body { from, to } => {
            let left = if undo { to } else { from };
            if now.title != expected.title || now.body != *left && now.body != expected.body {
                return Err(MOVED);
            }
            doc.set_body(*line, &target.body)
        }
    };
    inverse.ok_or(MOVED)
}
