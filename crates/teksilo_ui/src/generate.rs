//! The Generate screen: which files Qleany would write, what they would contain,
//! and writing them.

pub mod file_row;
pub mod generate_vm;
pub mod page;
pub mod preview;

pub use file_row::{FileRow, Filters};
pub use generate_vm::GenerateViewModel;
pub use page::GeneratePage;
