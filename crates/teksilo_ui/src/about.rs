//! The About box, and the confirmations that precede a destructive delete.

pub mod confirm;
pub mod dialog;

pub use confirm::confirm_delete;
pub use dialog::present;
