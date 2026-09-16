//! The `mocks` arm of the generated layer.
//!
//! Every single and every list model carries a `#[cfg(feature = "mocks")] mod imp`
//! twin with the same public surface and no backend behind it. It exists so a UI
//! can be built, laid out and screenshotted before the use cases it will eventually
//! read are written, and so the layout work does not wait on the backend.
//!
//! That only holds if the twin hands out something to lay out. These tests pin the
//! two properties the arm is worth having for: rows exist without a store, and the
//! same id fabricates the same row every time, so anything keyed on a row's
//! identity is stable across a refresh.
#![cfg(feature = "mocks")]

use std::rc::Rc;

use teksilo::prelude::Signal;

use frontend::AppContext;

use teksilo_ui::models::WorkspaceEntitiesListModel;
use teksilo_ui::session::Session;
use teksilo_ui::singles::{LoadingStatus, SingleGlobal};

/// No `initialize_app`: a mocks build is meant to run with nothing behind it, and a
/// context that needed seeding first would defeat the point.
fn ctx() -> Rc<AppContext> {
    Rc::new(AppContext::new())
}

#[test]
fn a_session_hands_out_every_handle_without_a_backend() {
    let session = Session::new(ctx());
    // Reading them is the assertion: the mock twin of any handle the generator emits
    // has to exist and construct, or this file does not compile.
    assert!(session.single_global.dto().is_some());
    assert!(session.single_entity.dto().is_some());
    assert!(session.single_user_interface.dto().is_some());
}

#[test]
fn a_single_fabricates_a_row_for_the_id_it_is_pointed_at() {
    let single = SingleGlobal::new(ctx());
    single.set_id(Some(7));

    assert_eq!(single.id(), Some(7));
    assert_eq!(single.loading_status().get(), LoadingStatus::Loaded);
    assert!(
        !single.application_name().get().is_empty(),
        "a mocked field has to render as something"
    );
}

/// Deterministic, not random. A fresh value per read would make the same row change
/// identity between refreshes, and every selection keyed on it would drop.
#[test]
fn the_same_id_fabricates_the_same_row_every_time() {
    let single = SingleGlobal::new(ctx());
    single.set_id(Some(3));
    let first = single.application_name().get();

    single.set_id(Some(4));
    let other = single.application_name().get();

    single.set_id(Some(3));
    assert_eq!(single.application_name().get(), first);
    assert_ne!(first, other, "two ids must not fabricate the same row");
}

#[test]
fn a_list_model_is_populated_without_an_owner_to_read_from() {
    let model = WorkspaceEntitiesListModel::new(ctx(), Signal::new(Some(1)));
    assert!(
        !model.is_empty(),
        "an empty mock list gives a layout nothing to be laid out against"
    );
    assert_eq!(model.rows().len(), model.len());
    assert!(model.row_at(0).is_some());
}
