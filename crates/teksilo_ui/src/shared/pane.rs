//! A column of a fixed width that still fills the height it is given.
//!
//! Every master-detail screen here has one: a list down the side, a form beside it.
//! `FixedSize::width` is not it. Its doc is explicit that an unbound dimension falls
//! back to the child's *natural* size, so a column built that way is as tall as its
//! own header and the list under it spills off the bottom of the window, unclipped.
//! The symptom is a list that is in the accessibility tree, and in no pixel on
//! screen, which is a confusing half hour for whoever meets it first.
//!
//! This is the navigation rail's own shape, generalised: pin the width, take the
//! height from the proposal.

use teksilo::prelude::*;

pub struct FixedWidth {
    width: f32,
    pending: Option<Box<dyn Widget>>,
    child_id: Option<WidgetId>,
}

impl std::fmt::Debug for FixedWidth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FixedWidth")
            .field("width", &self.width)
            .finish_non_exhaustive()
    }
}

impl FixedWidth {
    pub fn new(width: f32, child: impl Widget + 'static) -> Self {
        Self {
            width,
            pending: Some(Box::new(child)),
            child_id: None,
        }
    }
}

impl Widget for FixedWidth {
    fn build(&mut self, ctx: &mut BuildContext) -> Vec<WidgetId> {
        // Taken rather than cloned: a `Box<dyn Widget>` is not `Clone`, and a rebuild
        // of this widget rebuilds its parent, which hands over a fresh child.
        let Some(child) = self.pending.take() else {
            return self.child_id.into_iter().collect();
        };
        let id = ctx.add_boxed(child);
        self.child_id = Some(id);
        vec![id]
    }

    fn layout_response(&self, proposal: SizeProposal, ctx: &LayoutContext) -> LayoutResponse {
        // The child is measured against the width it will be placed at, so anything
        // that wraps or elides inside it computes against the real constraint rather
        // than against an unbounded one.
        let child_proposal = SizeProposal {
            width: Some(self.width),
            height: proposal.height,
        };
        let natural = self
            .child_id
            .and_then(|id| ctx.child_size(id, child_proposal))
            .map(|size| size.height)
            .unwrap_or(0.0);
        // The proposal wins on height: that is the whole difference from `FixedSize`.
        Size::new(self.width, proposal.height.unwrap_or(natural)).into()
    }
}

/// A row of a fixed height that still fills the width it is given.
///
/// The transpose of [`FixedWidth`], and needed for the same reason: a list that has
/// to stop at a certain height inside a scrolling form cannot use
/// `FixedSize::height`, whose unbound width falls back to the child's natural one.
/// Wrapping that in an `Expand::horizontal` does not rescue it either, because the
/// expand is then handed an unspecified width to expand into.
pub struct FixedHeight {
    height: f32,
    pending: Option<Box<dyn Widget>>,
    child_id: Option<WidgetId>,
}

impl std::fmt::Debug for FixedHeight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FixedHeight")
            .field("height", &self.height)
            .finish_non_exhaustive()
    }
}

impl FixedHeight {
    pub fn new(height: f32, child: impl Widget + 'static) -> Self {
        Self {
            height,
            pending: Some(Box::new(child)),
            child_id: None,
        }
    }
}

impl Widget for FixedHeight {
    fn build(&mut self, ctx: &mut BuildContext) -> Vec<WidgetId> {
        let Some(child) = self.pending.take() else {
            return self.child_id.into_iter().collect();
        };
        let id = ctx.add_boxed(child);
        self.child_id = Some(id);
        vec![id]
    }

    fn layout_response(&self, proposal: SizeProposal, ctx: &LayoutContext) -> LayoutResponse {
        let child_proposal = SizeProposal {
            width: proposal.width,
            height: Some(self.height),
        };
        let natural = self
            .child_id
            .and_then(|id| ctx.child_size(id, child_proposal))
            .map(|size| size.width)
            .unwrap_or(0.0);
        Size::new(proposal.width.unwrap_or(natural), self.height).into()
    }
}
