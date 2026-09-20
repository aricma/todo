//! Task model per the Open Productivity Standard (OPS): required
//! `title` + `status`, optional `notes`, recursive `subtasks` (schema only
//! in the MVP), and a `metadata` escape hatch preserved verbatim.
//! See MVP.md §3 and docs/adr/0004.

pub mod capacity;
pub mod form;
pub mod message;
pub mod storage;
pub mod storage_configuration;
pub mod storage_message;
pub mod task;

pub use capacity::{check_capacity, CapacityError};
pub use form::{Field, FieldKind};
pub use message::Message;
pub use storage::Storage;
pub use storage_configuration::{Availability, StorageConfiguration};
pub use storage_message::{messages, StorageContext, StorageMessage};
pub use task::{new_id, Freq, Recurrence, Status, Task, WeekdayName};
