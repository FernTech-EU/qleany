//! An empty list that says so.
//!
//! `ListView` has no empty state of its own: an empty model renders as nothing at
//! all, which on a screen whose whole left half is a list reads as a broken window
//! rather than as "there is nothing here yet". Every list in this app therefore sits
//! inside a [`Switcher`] that shows a line of text instead when the model is empty.
//!
//! Written once because there are nine of them, and because the part that is easy to
//! get wrong is not the `Switcher`: it is the signal that drives it. A list model's
//! emptiness is not a signal, it is a method, so the index has to be derived from the
//! model's version and re-read on every bump.

use teksilo::prelude::*;
use teksilo::widgets::{Padding, Switcher, TextWidget, VStack};

/// Whether a list is empty, as a signal.
///
/// `version` is the generated model's `version_signal`, which bumps on every reload.
/// The derived signal recomputes on every read, so the value is never stale; what the
/// version buys is the **notification**, which is what makes a bound `Switcher`
/// actually swap. Reading a fresh value nobody is told about would leave the empty
/// state on screen over a list that has since been filled.
///
/// A plain `Signal<bool>` written by each refresh would be the alternative, and it is
/// the one that goes wrong the first time a reload changes the row count and nobody
/// remembers to write the flag.
pub fn emptiness(version: &Signal<u64>, is_empty: impl Fn() -> bool + 'static) -> Signal<bool> {
    version.map(move |_| is_empty())
}

/// The list, or a line of text when there is nothing in it.
///
/// The list is the `Switcher`'s first child so that the common case is index 0, and
/// so an `emptiness` signal that somehow arrives stale shows the list rather than
/// claiming a populated list is empty.
pub fn list_or_empty(
    is_empty: Signal<bool>,
    list: impl teksilo::core::IntoTeksiChild,
    empty: impl teksilo::core::IntoTeksiChild,
) -> Switcher {
    Switcher::new(is_empty.map(|empty| usize::from(*empty)))
        .child(list)
        .child(empty)
}

/// The line of text itself: what there is none of, and how to make one.
///
/// Two lines rather than one, because "No entities yet" on its own tells a user what
/// they can already see. The second line is the part that helps.
pub fn empty_state(message: LocalizedString, hint: LocalizedString) -> impl Widget {
    Padding::symmetric(24.0, 40.0).child(teksu!(
        VStack {
            spacing: 4.0
            TextWidget::new(message) {
                style: TextStyleRole::BodyBold
                color: TextRole::Secondary
            }
            TextWidget::new(hint) {
                style: TextStyleRole::Small
                color: TextRole::Secondary
            }
        }
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    /// The value is never stale: a derived signal recomputes on every read, so the
    /// model is the single source of truth rather than a flag beside it.
    #[test]
    fn emptiness_reads_the_model_rather_than_a_remembered_flag() {
        let version = Signal::new(0u64);
        let rows = Rc::new(Cell::new(3usize));
        let count = rows.clone();
        let empty = emptiness(&version, move || count.get() == 0);

        assert!(!empty.get());
        rows.set(0);
        assert!(empty.get());
    }

    /// And the version is what tells anything bound to it to look again. Without the
    /// notification the value would be right and the window would still show the old
    /// one, because nothing asked.
    #[test]
    fn a_version_bump_notifies_whatever_is_bound_to_it() {
        let version = Signal::new(0u64);
        let rows = Rc::new(Cell::new(0usize));
        let count = rows.clone();
        let empty = emptiness(&version, move || count.get() == 0);

        let fired = Rc::new(Cell::new(0usize));
        let seen = fired.clone();
        let _handle = empty.observe(move |_| seen.set(seen.get() + 1));

        rows.set(2);
        version.set(1);
        assert!(fired.get() > 0, "a reload has to reach the Switcher");
    }

    /// Index 0 is the list. A `Switcher` seeded from a signal that has not been
    /// refreshed yet then shows the list, which is wrong for one frame, rather than
    /// an empty state over a populated list, which is wrong and alarming.
    #[test]
    fn the_list_is_the_first_child() {
        let empty = Signal::new(false);
        let index = empty.map(|e| usize::from(*e));
        assert_eq!(index.get(), 0);
        empty.set(true);
        assert_eq!(index.get(), 1);
    }
}
