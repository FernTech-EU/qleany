//! The Project settings screen: the five fields of the manifest's `Global` row.

pub mod page;
pub mod project_vm;

pub use page::ProjectPage;
pub use project_vm::{Language, ProjectViewModel};
