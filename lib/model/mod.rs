//! Task model per the Open Productivity Standard (OPS): required
//! `title` + `status`, optional `notes`, recursive `subtasks` (schema only
//! in the MVP), and a `metadata` escape hatch preserved verbatim.
//! See MVP.md §3 and docs/adr/0004.

pub mod capacity;
pub mod data_source;
pub mod data_source_configuration;
pub mod data_source_message;
pub mod form;
pub mod message;
pub mod task;

pub use capacity::{check_capacity, CapacityError};
pub use data_source::DataSource;
pub use data_source_configuration::{Availability, DataSourceConfiguration};
pub use data_source_message::{messages, DataSourceContext, DataSourceMessage};
pub use form::{Field, FieldKind};
pub use message::Message;
pub use task::{new_id, Freq, Recurrence, Status, Task, WeekdayName};
