//! Task model per the Open Productivity Standard (OPS): required
//! `title` + `status`, optional `notes`, recursive `subtasks` (schema only
//! in the MVP), and a `metadata` escape hatch preserved verbatim.
//! See MVP.md §3 and docs/adr/0004.

pub mod capacity;
pub mod message;
pub mod storage;
pub mod task;

pub use capacity::{check_capacity, CapacityError};
pub use message::Message;
pub use storage::{enabled, Availability, Storage};
pub use task::{new_id, Freq, Recurrence, Status, Task, WeekdayName};
