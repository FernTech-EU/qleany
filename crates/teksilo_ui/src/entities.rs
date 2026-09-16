//! The Entities screen: the manifest's entities, their fields, and the detail forms
//! for both.
//!
//! The largest screen in the app, and the one with the most rules. It is split the
//! way its state is: [`EntitiesViewModel`] owns the entity list and the entity form,
//! [`FieldViewModel`] owns the field list and the field form, and the page holds
//! both.

pub mod entities_vm;
pub mod entity_form;
pub mod field_form;
pub mod field_vm;
pub mod page;

pub use entities_vm::EntitiesViewModel;
pub use field_vm::FieldViewModel;
pub use page::EntitiesPage;
