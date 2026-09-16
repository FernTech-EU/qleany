//! Manifest validation: the badge that is always on screen, and the panel behind it.

pub mod check_vm;
pub mod panel;

pub use check_vm::{CheckStatus, CheckViewModel};
pub use panel::CheckBadge;
