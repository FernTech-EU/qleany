//! The overflow menu shared by editable list rows.
use teksilo::prelude::*;
use teksilo::widgets::{IconButton, IconButtonSize, PopoverIconButton};

/// Open from a normal click or keyboard activation, and also allow right-click.
/// Each factory captures the row's identity rather than the current selection.
pub fn row_menu_button(menu: impl Fn() -> Box<dyn Widget> + 'static) -> impl Widget {
    PopoverIconButton::new(
        IconButton::new(crate::icons::action::more())
            .size(IconButtonSize::Toolbar)
            .tooltip(tr!(common_more_actions())),
    )
    .content(menu())
    .bare()
    .show_disclosure_caret(false)
    .context_menu(move |_pos, _ctx| Some(menu()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};
    use teksilo::core::{
        WidgetTree,
        event::{Key, Modifiers, PointerButton, WidgetEvent},
    };
    use teksilo::widgets::{MenuItem, MenuList};

    fn press_enter(tree: &mut WidgetTree) {
        tree.dispatch_event(WidgetEvent::KeyDown {
            key: Key::Enter,
            modifiers: Modifiers::NONE,
            text: None,
        });
        tree.dispatch_event(WidgetEvent::KeyUp {
            key: Key::Enter,
            modifiers: Modifiers::NONE,
        });
    }

    #[test]
    fn click_keyboard_and_right_click_open_the_row_menu() {
        for button in [
            Some(PointerButton::Primary),
            None,
            Some(PointerButton::Secondary),
        ] {
            let activated = Rc::new(Cell::new(None));
            let target = activated.clone();
            let menu = row_menu_button(move || {
                let target = target.clone();
                Box::new(MenuList::new().item(
                    MenuItem::new(lit!("Delete row")).on_activate_fn(move |_| target.set(Some(42))),
                ))
            });
            let mut tree = WidgetTree::new().with_theme(crate::style::light());
            let root = tree.add(menu);
            tree.layout(SizeProposal::exact(300.0, 200.0));
            let trigger = tree.first_focusable_descendant(root).expect("menu trigger");
            assert!(tree.active_overlays().is_empty());
            if let Some(button) = button {
                let bounds = tree.bounds(trigger);
                let point = Point::new(
                    bounds.x + bounds.width / 2.0,
                    bounds.y + bounds.height / 2.0,
                );
                tree.dispatch_event(WidgetEvent::pointer_down(point, button, Modifiers::NONE));
                tree.dispatch_event(WidgetEvent::pointer_up(point, button, Modifiers::NONE));
            } else {
                tree.focus(trigger);
                press_enter(&mut tree);
            }
            tree.layout(SizeProposal::exact(300.0, 200.0));
            assert_eq!(tree.active_overlays().len(), 1, "activation: {button:?}");
            tree.dispatch_event(WidgetEvent::KeyDown {
                key: Key::ArrowDown,
                modifiers: Modifiers::NONE,
                text: None,
            });
            tree.dispatch_event(WidgetEvent::KeyUp {
                key: Key::ArrowDown,
                modifiers: Modifiers::NONE,
            });
            press_enter(&mut tree);
            assert_eq!(
                activated.get(),
                Some(42),
                "the menu action targets its own row"
            );
        }
    }
}
