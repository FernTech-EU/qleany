//! The four-step wizard that creates a manifest.

pub mod new_manifest_vm;
pub mod wizard;

pub use new_manifest_vm::{NewManifestViewModel, Template};
pub use wizard::present;
