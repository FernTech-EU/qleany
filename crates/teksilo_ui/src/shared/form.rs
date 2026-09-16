//! The pieces every settings form is built from.
//!
//! Screens differ in what they edit, not in how a heading or a field label looks,
//! so those two live here and each screen states only its own content.

use teksilo::prelude::*;
use teksilo::widgets::{Checkbox, HStack, TextWidget};

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

/// A checkbox with its label beside it rather than inside it.
///
/// `Checkbox::label` puts the text inside the widget, where it shrinks before
/// anything else does: in a row beside other controls it collapses to an ellipsis
/// and the user is left with six identical boxes. The label is given to the
/// checkbox as its accessible name and drawn separately, which is the same split
/// every form on this screen already uses.
pub fn inline_checkbox(
    label: LocalizedString,
    checked: Signal<bool>,
    on_change: impl Fn(bool, &mut EventContext) + 'static,
) -> impl Widget {
    HStack::new()
        .spacing(6.0)
        .child(
            Checkbox::new(checked)
                .label(label.clone())
                .labelled_externally()
                .on_change(on_change),
        )
        .child(TextWidget::new(label).no_shrink())
}
