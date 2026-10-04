//! Doing something once a burst of changes has gone quiet.
//!
//! Some editors have no moment that means "finished". A single-line field has Enter
//! and blur, and every text field in this app commits on those. A multi-line editor
//! has neither: it reports a document version that bumps on each keystroke, and
//! nothing else.
//!
//! Committing on each of those would work and would put one undo entry on the stack
//! per character typed, so a user who typed a six-line enum and pressed Ctrl+Z would
//! get one letter back. [`Settle`] is the alternative: mark each change, and act on
//! the first frame that brings none.

use std::cell::Cell;
use std::rc::Rc;

/// A two-flag state machine: something changed, and the changes have stopped.
///
/// Cloning shares the state, so the widget that reports changes and the frame effect
/// that acts on them can each hold one.
#[derive(Clone, Default)]
pub struct Settle {
    /// A change arrived since the last frame.
    saw_change: Rc<Cell<bool>>,
    /// There is a burst in progress that has not been acted on yet.
    pending: Rc<Cell<bool>>,
}

impl std::fmt::Debug for Settle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Settle")
            .field("pending", &self.pending.get())
            .finish_non_exhaustive()
    }
}

impl Settle {
    pub fn new() -> Self {
        Self::default()
    }

    /// A change arrived. Call from whatever reports them.
    pub fn touch(&self) {
        self.saw_change.set(true);
        self.pending.set(true);
    }

    /// One frame. `true` exactly once per burst, on the first quiet frame after it.
    ///
    /// Call from a frame-tick effect. Teksilo pumps a frame only when something asks
    /// for one, so the caller also has to keep the window awake while a burst is in
    /// flight; `BuildContext::wake_at_handle` is how the rest of this app does that.
    pub fn tick(&self) -> bool {
        if self.saw_change.replace(false) {
            // Still going.
            return false;
        }
        // A quiet frame. If a burst was in flight, it has just ended.
        self.pending.replace(false)
    }

    /// Whether a burst is waiting to be acted on.
    pub fn is_pending(&self) -> bool {
        self.pending.get()
    }

    /// Give up on the burst without acting, for a caller that is about to throw the
    /// edit away anyway, such as one switching to a different row.
    pub fn forget(&self) {
        self.saw_change.set(false);
        self.pending.set(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole point: a burst of changes produces exactly one action, after it.
    #[test]
    fn a_burst_of_changes_settles_once() {
        let settle = Settle::new();

        for _ in 0..5 {
            settle.touch();
            assert!(!settle.tick(), "a frame with a change in it is not quiet");
        }

        assert!(settle.tick(), "the first quiet frame ends the burst");
        assert!(!settle.tick(), "and it only ends once");
    }

    /// A frame with nothing in it, with no burst in flight, does nothing. Without
    /// this the frame effect would fire on every frame of an idle window.
    #[test]
    fn a_quiet_frame_with_no_burst_is_inert() {
        let settle = Settle::new();
        assert!(!settle.tick());
        assert!(!settle.tick());
    }

    /// Two bursts are two actions: a user who types, pauses, and types again gets
    /// two undo entries, which is what they would expect.
    #[test]
    fn two_bursts_settle_twice() {
        let settle = Settle::new();

        settle.touch();
        assert!(!settle.tick());
        assert!(settle.tick());

        settle.touch();
        assert!(!settle.tick());
        assert!(settle.tick());
    }

    #[test]
    fn forgetting_a_burst_leaves_nothing_to_settle() {
        let settle = Settle::new();
        settle.touch();
        assert!(settle.is_pending());

        settle.forget();
        assert!(!settle.is_pending());
        assert!(!settle.tick());
        assert!(!settle.tick());
    }
}
