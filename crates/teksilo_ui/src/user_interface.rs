//! The User Interface screen: which frontends Qleany scaffolds.

pub mod page;
pub mod user_interface_vm;

pub use page::UserInterfacePage;
pub use user_interface_vm::{Target, UserInterfaceViewModel};
