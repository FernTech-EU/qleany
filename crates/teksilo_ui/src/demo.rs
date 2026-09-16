//! The demo generator: a whole sample project from one dialog.

pub mod demo_vm;
pub mod dialog;

pub use demo_vm::{DemoViewModel, Phase, Step};
pub use dialog::present;
