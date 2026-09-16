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

/// Add a row to a list.
pub fn add() -> IconWidget {
    IconWidget::from_svg_icon(res!("assets/icons/action/add.svg")).icon_size(SIZE)
}

/// Delete a row.
pub fn delete() -> IconWidget {
    IconWidget::from_svg_icon(res!("assets/icons/action/delete.svg")).icon_size(SIZE)
}

/// The row overflow menu. A visible affordance rather than right-click only:
/// a menu nothing points at is a menu most users never find.
pub fn more() -> IconWidget {
    IconWidget::from_svg_icon(res!("assets/icons/action/more.svg")).icon_size(SIZE)
}

/// Export the model as a diagram.
pub fn diagram() -> IconWidget {
    IconWidget::from_svg_icon(res!("assets/icons/action/diagram.svg")).icon_size(SIZE)
}

/// The manifest validates.
pub fn check_ok() -> IconWidget {
    IconWidget::from_svg_icon(res!("assets/icons/action/check.svg")).icon_size(SIZE)
}

/// The manifest has warnings: it will generate, but something looks wrong.
pub fn check_warning() -> IconWidget {
    IconWidget::from_svg_icon(res!("assets/icons/action/warning.svg")).icon_size(SIZE)
}

/// The manifest has a critical error: it will not generate.
pub fn check_critical() -> IconWidget {
    IconWidget::from_svg_icon(res!("assets/icons/action/error.svg")).icon_size(SIZE)
}

/// Dismiss a panel.
pub fn close() -> IconWidget {
    IconWidget::from_svg_icon(res!("assets/icons/action/close.svg")).icon_size(SIZE)
}

/// Run something again.
pub fn refresh() -> IconWidget {
    IconWidget::from_svg_icon(res!("assets/icons/action/refresh.svg")).icon_size(SIZE)
}
