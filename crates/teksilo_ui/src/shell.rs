//! The window chrome: title bar, menu model, and the controls that live in the bar.

pub mod menus;
pub mod save_button;
pub mod theme_button;
pub mod window;

/// Logical-pixel height of the title bar.
///
/// The bar does not grow for an oversized child, so every icon button in it is
/// `IconButtonSize::Toolbar` (30 dp) rather than `Large` (40).
pub(crate) const TITLE_BAR_HEIGHT: f32 = 34.0;
