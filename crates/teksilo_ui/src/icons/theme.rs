//! The light/dark toggle glyphs.
//!
//! Each icon advertises the **action**, not the current state: while the app is
//! light the button shows a moon, because pressing it goes dark.

use teksilo::res;
use teksilo::widgets::IconWidget;

const SIZE: f32 = 16.0;

/// Shown while the app is light. Pressing it switches to dark.
pub fn to_dark() -> IconWidget {
    IconWidget::from_svg_icon(res!("assets/icons/theme/to-dark.svg")).icon_size(SIZE)
}

/// Shown while the app is dark. Pressing it switches to light.
pub fn to_light() -> IconWidget {
    IconWidget::from_svg_icon(res!("assets/icons/theme/to-light.svg")).icon_size(SIZE)
}
