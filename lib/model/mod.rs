//! Task model per the Open Productivity Standard (OPS): required
//! `title` + `status`, optional `notes`, recursive `subtasks` (schema only
//! in the MVP), and a `metadata` escape hatch preserved verbatim.
//! See MVP.md §3 and docs/adr/0004.

pub mod capacity;
pub mod source;
pub mod task;

pub use capacity::{check_capacity, CapacityError};
pub use source::{Availability, Provider};
pub use task::{new_id, Freq, Recurrence, Status, Task, WeekdayName};
