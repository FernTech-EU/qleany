//! The pieces every settings form is built from.
//!
//! Screens differ in what they edit, not in how a heading or a field label looks,
//! so those two live here and each screen states only its own content.

use teksilo::prelude::*;
use teksilo::widgets::{HStack, TextWidget};

/// A screen's heading.
///
/// `TextStyleRole::BodyBold` rather than a `TextStyle` built here with a larger
/// size: a style value is frozen when the widget is built, so a hand-built one
/// would ignore the user's text-scale accessibility setting from then on, while a
/// role is resolved against the live typography every time it is painted. Teksilo's
/// typography has no heading role yet, and bold body is what its own tokens call
/// the closest thing to a heading.
pub fn heading(text: LocalizedString) -> impl Widget {
    TextWidget::new(text).style(TextStyleRole::BodyBold)
}

/// A form field's visible label.
pub fn field_label(text: LocalizedString) -> impl Widget {
    TextWidget::new(text)
}

/// A form field's visible label, marked as required.
///
/// The marker is hidden from assistive technology: it carries no meaning a screen
/// reader can use, and read aloud it turns "Application name" into "Application
/// name star". What a screen reader needs instead is the field's own validation
/// message, which the input publishes.
pub fn required_field_label(text: LocalizedString) -> impl Widget {
    HStack::new()
        .spacing(2.0)
        .child(TextWidget::new(text))
        .child(
            TextWidget::new(tr!(form_required_marker()))
                .color(TextRole::Error)
                .a11y_hidden(),
        )
}
