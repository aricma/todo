//! `--format`: the two mechanical fixes, and nothing else.

use super::Doc;

impl Doc {
    /// Fix the checkbox case and the priority letter case. Nothing else, ever.
    /// Returns whether any line changed, so a clean file is never rewritten.
    pub fn format(&mut self) -> bool {
        let mut changed = false;
        for task in self.tasks() {
            let line = &mut self.lines[task.line];
            let checkbox = task.indent + task.marker.len() + 2;
            let mut bytes = line.clone().into_bytes();
            if bytes[checkbox] == b'X' {
                bytes[checkbox] = b'x';
            }
            if let Some(prio) = task.prio {
                let letter = checkbox + 4;
                let canonical = prio.letter() as u8;
                if bytes[letter] != canonical {
                    bytes[letter] = canonical;
                }
            }
            if let Ok(formatted) = String::from_utf8(bytes) {
                changed |= formatted != *line;
                *line = formatted;
            }
        }
        changed
    }
}
