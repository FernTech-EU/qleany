//! The design language the app is built in.
//!
//! Qleany is a Fluent app. There is no style flag and no style setting, because a
//! runtime family switch cannot deliver what the name promises: colour tokens
//! resolve at paint time and retint instantly, but widget chrome is resolved when a
//! widget is *built*, so switching family in a live window changes the palette and
//! keeps the previous shapes. Light against dark is a different matter, and that one
//! is a real setting.
//!
//! Every theme the app can hold comes from here, so the startup seed, the title-bar
//! toggle and the View menu cannot disagree about what "dark" means.

use teksilo::prelude::Theme;
use teksilo::prelude::fluent;

/// The light variant. `theme.id` is `fluent.light`.
pub fn light() -> Theme {
    fluent::light()
}

/// The dark variant. `theme.id` is `fluent.dark`.
pub fn dark() -> Theme {
    fluent::dark()
}

/// The theme for a given dark/light answer.
pub fn for_dark(dark_mode: bool) -> Theme {
    if dark_mode { dark() } else { light() }
}

/// Whether a theme is one of ours and dark.
///
/// Read from the live theme rather than from the stored preference, so the
/// title-bar glyph reflects what is actually on screen.
pub fn is_dark(theme: &Theme) -> bool {
    theme.is_dark()
}
