//! The six navigation-rail glyphs, carried over from the Slint UI so the app
//! stays recognisable across the port.

use teksilo::res;
use teksilo::widgets::IconWidget;

/// Rail glyph size in logical pixels.
const SIZE: f32 = 20.0;

pub fn home() -> IconWidget {
    IconWidget::from_svg_icon(res!("assets/icons/nav/home.svg")).icon_size(SIZE)
}

pub fn project() -> IconWidget {
    IconWidget::from_svg_icon(res!("assets/icons/nav/project.svg")).icon_size(SIZE)
}

pub fn entities() -> IconWidget {
    IconWidget::from_svg_icon(res!("assets/icons/nav/entities.svg")).icon_size(SIZE)
}

pub fn features() -> IconWidget {
    IconWidget::from_svg_icon(res!("assets/icons/nav/features.svg")).icon_size(SIZE)
}

pub fn user_interface() -> IconWidget {
    IconWidget::from_svg_icon(res!("assets/icons/nav/user_interface.svg")).icon_size(SIZE)
}

pub fn generate() -> IconWidget {
    IconWidget::from_svg_icon(res!("assets/icons/nav/generate.svg")).icon_size(SIZE)
}
