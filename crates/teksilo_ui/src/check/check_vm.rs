//! What the manifest's validation says.
//!
//! The backend has one `check` use case that returns every problem it found. The
//! work here is deciding *when* to run it: after every edit would run it hundreds
//! of times during a load, and only on demand would leave the badge lying.

use std::rc::Rc;

use teksilo::prelude::*;

use frontend::AppContext;
use frontend::commands::handling_manifest_commands;
use frontend::common::event::{DirectAccessEntity, EntityEvent, Event, Origin};

use crate::shared::settle::Settle;

/// The count at which the badge stops counting.
///
/// A badge is a glance, not a report: "9+" and "14" tell a user the same thing,
/// and the panel behind it has the real list.
const BADGE_CAP: usize = 9;

/// Every entity whose creation, update or removal can change what the check says.
///
/// `Created` is in the list for the same reason it is in the manifest's dirty
/// tracking: adding an entity with no fields is exactly the kind of thing the check
/// complains about, and a badge that only noticed updates would stay green.
const WATCHED: &[DirectAccessEntity] = &[
    DirectAccessEntity::Workspace(EntityEvent::Updated),
    DirectAccessEntity::Entity(EntityEvent::Created),
    DirectAccessEntity::Entity(EntityEvent::Updated),
    DirectAccessEntity::Entity(EntityEvent::Removed),
    DirectAccessEntity::Field(EntityEvent::Created),
    DirectAccessEntity::Field(EntityEvent::Updated),
    DirectAccessEntity::Field(EntityEvent::Removed),
    DirectAccessEntity::Feature(EntityEvent::Created),
    DirectAccessEntity::Feature(EntityEvent::Updated),
    DirectAccessEntity::Feature(EntityEvent::Removed),
    DirectAccessEntity::UseCase(EntityEvent::Created),
    DirectAccessEntity::UseCase(EntityEvent::Updated),
    DirectAccessEntity::UseCase(EntityEvent::Removed),
    DirectAccessEntity::Dto(EntityEvent::Created),
    DirectAccessEntity::Dto(EntityEvent::Updated),
    DirectAccessEntity::Dto(EntityEvent::Removed),
    DirectAccessEntity::DtoField(EntityEvent::Created),
    DirectAccessEntity::DtoField(EntityEvent::Updated),
    DirectAccessEntity::DtoField(EntityEvent::Removed),
    DirectAccessEntity::Global(EntityEvent::Updated),
    DirectAccessEntity::UserInterface(EntityEvent::Updated),
];

/// What the badge shows.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum CheckStatus {
    /// Nothing is open, so there is nothing to say.
    #[default]
    None,
    /// The manifest is valid.
    Ok,
    /// It will generate, but something looks wrong.
    Warning,
    /// It will not generate.
    Critical,
}

impl CheckStatus {
    /// Which of the three counts a result carries.
    pub fn of(criticals: usize, warnings: usize) -> CheckStatus {
        if criticals > 0 {
            CheckStatus::Critical
        } else if warnings > 0 {
            CheckStatus::Warning
        } else {
            CheckStatus::Ok
        }
    }
}

/// What a badge counts down to. `9+` past the cap.
pub fn badge_text(count: usize) -> String {
    if count > BADGE_CAP {
        format!("{BADGE_CAP}+")
    } else {
        count.to_string()
    }
}

#[derive(Clone)]
pub struct CheckViewModel {
    app_ctx: Rc<AppContext>,
    /// Whether a manifest is open. Nothing is checked without one.
    open: Signal<bool>,
    status: Signal<CheckStatus>,
    criticals: Signal<Vec<String>>,
    warnings: Signal<Vec<String>>,
    /// Edits arrive in bursts; the check runs once the burst has drained.
    settle: Settle,
}

impl std::fmt::Debug for CheckViewModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CheckViewModel")
            .field("status", &self.status.get())
            .finish_non_exhaustive()
    }
}

impl CheckViewModel {
    pub fn new(app_ctx: Rc<AppContext>, open: Signal<bool>) -> Self {
        Self {
            app_ctx,
            open,
            status: Signal::new(CheckStatus::None),
            criticals: Signal::new(Vec::new()),
            warnings: Signal::new(Vec::new()),
            settle: Settle::new(),
        }
    }

    // ── state a view binds ───────────────────────────────────────────────────

    pub fn status(&self) -> Signal<CheckStatus> {
        self.status.clone()
    }

    /// Whether the manifest has a critical error. The navigation rail reads this:
    /// generating from a manifest that does not validate produces code that does
    /// not compile, so Generate is out of reach until it is fixed.
    pub fn critical(&self) -> Signal<bool> {
        self.status.map(|s| *s == CheckStatus::Critical)
    }

    pub fn criticals(&self) -> Signal<Vec<String>> {
        self.criticals.clone()
    }

    pub fn warnings(&self) -> Signal<Vec<String>> {
        self.warnings.clone()
    }

    /// What the badge says: the critical count when there is one, otherwise the
    /// warning count.
    pub fn badge(&self) -> Signal<String> {
        self.criticals
            .zip(&self.warnings)
            .map(|(criticals, warnings)| {
                if criticals.is_empty() {
                    badge_text(warnings.len())
                } else {
                    badge_text(criticals.len())
                }
            })
    }

    /// The problems, criticals first: a warning is worth reading after the thing
    /// that stops the build, not before it.
    pub fn problems(&self) -> Signal<Vec<(bool, String)>> {
        self.criticals
            .zip(&self.warnings)
            .map(|(criticals, warnings)| {
                criticals
                    .iter()
                    .map(|m| (true, m.clone()))
                    .chain(warnings.iter().map(|m| (false, m.clone())))
                    .collect()
            })
    }

    // ── commands ─────────────────────────────────────────────────────────────

    /// Run the check now, whatever the edits have been doing.
    pub fn run(&self) {
        if !self.open.get() {
            self.clear();
            return;
        }
        match handling_manifest_commands::check(&self.app_ctx) {
            Ok(result) => {
                self.status.set_if_changed(CheckStatus::of(
                    result.critical_errors.len(),
                    result.warnings.len(),
                ));
                self.criticals.set_if_changed(result.critical_errors);
                self.warnings.set_if_changed(result.warnings);
            }
            Err(e) => log::error!("the manifest check failed to run: {e}"),
        }
    }

    /// Forget everything. Closing a manifest leaves nothing to be valid about.
    pub fn clear(&self) {
        self.status.set_if_changed(CheckStatus::None);
        self.criticals.set_if_changed(Vec::new());
        self.warnings.set_if_changed(Vec::new());
    }

    // ── wiring ───────────────────────────────────────────────────────────────

    pub fn wire(&self, ctx: &mut BuildContext) {
        let wake = ctx.wake_at_handle();
        for entity in WATCHED {
            let settle = self.settle.clone();
            let wake = wake.clone();
            ctx.subscribe_event(Origin::DirectAccess(entity.clone()), move |_e: &Event| {
                settle.touch();
                wake.set(Some(std::time::Instant::now()));
            });
        }

        let me = self.clone();
        let tick = ctx.frame_tick();
        ctx.effect(&tick, move |_| {
            if me.settle.tick() {
                me.run();
            }
        });

        // A manifest that opens is checked; one that closes clears the badge.
        let me = self.clone();
        let open = self.open.clone();
        ctx.effect(&open, move |open| {
            if *open {
                me.settle.touch();
            } else {
                me.settle.forget();
                me.clear();
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// US-CHK-01: a critical error outranks a warning, because it is the one that
    /// stops the build.
    #[test]
    fn a_critical_error_outranks_a_warning() {
        assert_eq!(CheckStatus::of(0, 0), CheckStatus::Ok);
        assert_eq!(CheckStatus::of(0, 3), CheckStatus::Warning);
        assert_eq!(CheckStatus::of(1, 0), CheckStatus::Critical);
        assert_eq!(CheckStatus::of(1, 3), CheckStatus::Critical);
    }

    /// US-CHK-01: the badge caps. A badge is a glance, and "14" says nothing "9+"
    /// does not.
    #[test]
    fn the_badge_caps_at_nine() {
        assert_eq!(badge_text(0), "0");
        assert_eq!(badge_text(9), "9");
        assert_eq!(badge_text(10), "9+");
        assert_eq!(badge_text(400), "9+");
    }

    /// US-CHK-02: criticals first, then warnings, each the backend's own sentence.
    #[test]
    fn the_panel_lists_criticals_before_warnings() {
        let ctx = Rc::new(AppContext::new());
        let vm = CheckViewModel::new(ctx, Signal::new(true));
        vm.criticals.set(vec!["no entities".to_string()]);
        vm.warnings
            .set(vec!["no features".to_string(), "no ui".to_string()]);

        let problems = vm.problems().get();
        assert_eq!(
            problems,
            vec![
                (true, "no entities".to_string()),
                (false, "no features".to_string()),
                (false, "no ui".to_string()),
            ]
        );
    }

    /// The badge counts the criticals while there are any, and the warnings once
    /// there are not.
    #[test]
    fn the_badge_counts_what_matters_most() {
        let ctx = Rc::new(AppContext::new());
        let vm = CheckViewModel::new(ctx, Signal::new(true));
        let badge = vm.badge();

        vm.warnings.set(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(badge.get(), "2");

        vm.criticals.set(vec!["boom".to_string()]);
        assert_eq!(badge.get(), "1", "a critical error takes the badge");
    }

    /// US-CHK-04: closing the manifest clears the badge and the panel with it.
    #[test]
    fn closing_clears_everything() {
        let ctx = Rc::new(AppContext::new());
        let vm = CheckViewModel::new(ctx, Signal::new(true));
        vm.criticals.set(vec!["boom".to_string()]);
        vm.status.set(CheckStatus::Critical);

        vm.clear();
        assert_eq!(vm.status().get(), CheckStatus::None);
        assert!(vm.criticals().get().is_empty());
        assert!(vm.warnings().get().is_empty());
    }

    /// US-CHK-03: a burst of edits runs the check once, after it.
    #[test]
    fn a_burst_of_edits_checks_once() {
        let ctx = Rc::new(AppContext::new());
        let vm = CheckViewModel::new(ctx, Signal::new(true));

        for _ in 0..8 {
            vm.settle.touch();
            assert!(!vm.settle.tick(), "still editing");
        }
        assert!(vm.settle.tick(), "the quiet frame runs the check");
        assert!(!vm.settle.tick(), "and only once");
    }
}
