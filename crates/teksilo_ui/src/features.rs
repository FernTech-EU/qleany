//! The Features screen: features, their use cases, and each use case's DTOs.
//!
//! Split the way the state is. [`FeaturesViewModel`] owns the feature list,
//! [`UseCaseViewModel`] the use cases and the flags, and [`DtoViewModel`] one DTO
//! pane, of which there are two: a use case has an input DTO and an output one, and
//! they are edited side by side and entirely independently.

pub mod dto_pane;
pub mod dto_vm;
pub mod features_vm;
pub mod page;
pub mod use_case_form;
pub mod use_case_vm;

pub use dto_vm::{DtoSide, DtoViewModel};
pub use features_vm::FeaturesViewModel;
pub use page::FeaturesPage;
pub use use_case_vm::UseCaseViewModel;
