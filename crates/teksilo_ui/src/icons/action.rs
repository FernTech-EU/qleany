//! Glyphs for the title-bar and toolbar actions.

use teksilo::res;
use teksilo::widgets::IconWidget;

/// Title-bar glyph size. The bar is [`crate::shell::TITLE_BAR_HEIGHT`] tall and
/// lets an oversized child overflow, so this pairs with `IconButtonSize::Toolbar`.
const SIZE: f32 = 16.0;

/// Save, with everything already on disk.
pub fn save() -> IconWidget {
    IconWidget::from_svg_icon(res!("assets/icons/action/save.svg")).icon_size(SIZE)
}

/// Save, with unsaved changes. A filled marker, not a tint: a colour difference
/// alone would read as "disabled" rather than "there is work to write".
pub fn save_unsaved() -> IconWidget {
    IconWidget::from_svg_icon(res!("assets/icons/action/save-unsaved.svg")).icon_size(SIZE)
}

pub fn undo() -> IconWidget {
    IconWidget::from_svg_icon(res!("assets/icons/action/undo.svg")).icon_size(SIZE)
}

pub fn redo() -> IconWidget {
    IconWidget::from_svg_icon(res!("assets/icons/action/redo.svg")).icon_size(SIZE)
}
