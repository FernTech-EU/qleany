//! Helpers with more than one caller.
//!
//! Nothing lands here on the strength of one screen needing it: a helper written
//! for a single call site is harder to read than the code it replaced, because the
//! reader has to go and find it. Everything here is used by at least two screens.

pub mod form;
pub mod list_or_empty;
pub mod reorder;
