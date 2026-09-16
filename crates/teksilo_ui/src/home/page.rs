//! The Home screen.

use teksilo::prelude::*;
use teksilo::widgets::{
    Button, ButtonVariant, Card, Expand, FixedSize, HStack, Link, Padding, ScrollArea, TextWidget,
    VStack, Wrap,
};

use crate::home::home_vm::{DocLink, HomeViewModel, doc_links};
use crate::intents::name;
use crate::manifest::ManifestViewModel;

/// Width of a documentation card. Fixed so the `Wrap` reflows into a tidy grid
/// rather than a ragged one of nine different widths, and wide enough for the
/// longest blurb on two lines.
const CARD_WIDTH: f32 = 300.0;

pub fn page(home: HomeViewModel, manifest: ManifestViewModel) -> impl Widget {
    let open = manifest.is_open();
    let can_save = manifest.can_save();

    let actions = teksu!(
        HStack {
            spacing: 8.0
            Button::new(tr!(home_new_manifest())) {
                variant: ButtonVariant::Filled
                on_activate_fn: |c| c.send_intent(Intent::new(name::NEW_MANIFEST))
            }
            Button::new(tr!(home_open_manifest())) {
                variant: ButtonVariant::Filled
                on_activate_fn: |c| c.send_intent(Intent::new(name::OPEN_MANIFEST))
            }
            Button::new(tr!(home_save_manifest())) {
                enabled: can_save
                on_activate_fn: |c| c.send_intent(Intent::new(name::SAVE_MANIFEST))
            }
            Button::new(tr!(home_save_manifest_as())) {
                enabled: open.clone()
                on_activate_fn: |c| c.send_intent(Intent::new(name::SAVE_MANIFEST_AS))
            }
            Button::new(tr!(home_close_manifest())) {
                enabled: open.clone()
                on_activate_fn: |c| c.send_intent(Intent::new(name::CLOSE_MANIFEST))
            }
            Expand::horizontal
            Button::new(tr!(home_run_demo())) {
                variant: ButtonVariant::Tinted
                on_activate_fn: |c| c.send_intent(Intent::new(name::RUN_DEMO))
            }
        }
    );

    // A `Wrap` rather than a hand-rolled row of rows: the nine cards are static,
    // but a manual `for` over fixed columns is exactly the shape that stops
    // reflowing the moment the window narrows.
    let cards = Wrap::new()
        .spacing(12.0)
        .line_spacing(12.0)
        .children(doc_links().into_iter().map(|link| card(&home, link)));

    let developer = home.developer();
    let dev_block = teksu!(
        VStack {
            spacing: 8.0
            visible_when: developer
            TextWidget::new(tr!(home_for_testing()))
            HStack {
                Button::new(tr!(home_open_qleany_manifest())) {
                    on_activate_fn: |c| c.send_intent(Intent::new(name::OPEN_QLEANY_MANIFEST))
                }
                Expand::horizontal
            }
        }
    );

    teksu!(
        ScrollArea {
            Padding::new(20.0, 24.0, 20.0, 24.0) {
                VStack {
                    spacing: 20.0
                    TextWidget::new(tr!(home_title()))
                    TextWidget::new(tr!(home_subtitle()))
                    child: actions
                    TextWidget::new(tr!(home_documentation()))
                    child: cards
                    child: dev_block
                }
            }
        }
    )
}

fn card(home: &HomeViewModel, link: DocLink) -> impl Widget {
    let vm = home.clone();
    let url = link.url();
    teksu!(
        FixedSize {
            width: CARD_WIDTH
            Card {
                padding: 12.0
                content: VStack {
                    spacing: 4.0
                    Link::new(link.label) {
                        on_activate_fn: move |_c| vm.open_link(&url)
                    }
                    TextWidget::new(link.blurb)
                }
            }
        }
    )
}
