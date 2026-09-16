//! Opening, saving and closing the manifest, and the dirty state that follows.

pub mod guard;
mod manifest_vm;

pub use guard::with_unsaved_settled;
pub use manifest_vm::ManifestViewModel;
