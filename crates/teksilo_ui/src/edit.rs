//! Undo and redo.
//!
//! Qleany keeps one stack per screen rather than one for the application, which
//! `docs/undo-redo-architecture.md` calls Approach B and recommends over a single
//! linear history. The Slint UI records into those four stacks and exposes nothing;
//! this is the half that was missing.

pub mod undo_action;
pub mod undo_vm;

pub use undo_action::{UndoAction, labeled};
pub use undo_vm::UndoViewModel;
