//! What an undo entry is called.
//!
//! The generated `direct_access` controllers record no label: `UndoRedoCommand::label`
//! is the trait default, which is `None`. So the name of an operation is the UI's to
//! mint, and it has to be minted *before* the operation runs, because the backend
//! stores it with the entry rather than deriving it afterwards.
//!
//! `UndoLabel` carries two `&'static str`s, so the set is fixed and small. That is
//! the right shape: a menu row saying "Undo add entity" is useful, and one saying
//! "Undo update Entity 47" is not.

use frontend::AppContext;
use frontend::commands::undo_redo_commands;
use frontend::common::undo_redo::UndoLabel;
use teksilo::prelude::LocalizedString;

/// Everything this app can undo, named.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UndoAction {
    EditProject,
    AddEntity,
    RemoveEntity,
    ReorderEntities,
    EditEntity,
    AddField,
    RemoveField,
    ReorderFields,
    EditField,
    AddFeature,
    RemoveFeature,
    ReorderFeatures,
    EditFeature,
    AddUseCase,
    RemoveUseCase,
    ReorderUseCases,
    EditUseCase,
    EnableDto,
    DisableDto,
    EditDto,
    AddDtoField,
    RemoveDtoField,
    ReorderDtoFields,
    EditDtoField,
    EditUserInterface,
}

impl UndoAction {
    /// Every action, for the round-trip test below.
    pub const ALL: [UndoAction; 25] = [
        UndoAction::EditProject,
        UndoAction::AddEntity,
        UndoAction::RemoveEntity,
        UndoAction::ReorderEntities,
        UndoAction::EditEntity,
        UndoAction::AddField,
        UndoAction::RemoveField,
        UndoAction::ReorderFields,
        UndoAction::EditField,
        UndoAction::AddFeature,
        UndoAction::RemoveFeature,
        UndoAction::ReorderFeatures,
        UndoAction::EditFeature,
        UndoAction::AddUseCase,
        UndoAction::RemoveUseCase,
        UndoAction::ReorderUseCases,
        UndoAction::EditUseCase,
        UndoAction::EnableDto,
        UndoAction::DisableDto,
        UndoAction::EditDto,
        UndoAction::AddDtoField,
        UndoAction::RemoveDtoField,
        UndoAction::ReorderDtoFields,
        UndoAction::EditDtoField,
        UndoAction::EditUserInterface,
    ];

    /// The pair the backend stores. Both halves are `'static`, which is why the set
    /// is an enum rather than a formatted string.
    pub fn label(self) -> UndoLabel {
        let (subject, action) = self.parts();
        UndoLabel::new(subject, action)
    }

    fn parts(self) -> (&'static str, &'static str) {
        match self {
            UndoAction::EditProject => ("project", "edit"),
            UndoAction::AddEntity => ("entity", "add"),
            UndoAction::RemoveEntity => ("entity", "remove"),
            UndoAction::ReorderEntities => ("entity", "reorder"),
            UndoAction::EditEntity => ("entity", "edit"),
            UndoAction::AddField => ("field", "add"),
            UndoAction::RemoveField => ("field", "remove"),
            UndoAction::ReorderFields => ("field", "reorder"),
            UndoAction::EditField => ("field", "edit"),
            UndoAction::AddFeature => ("feature", "add"),
            UndoAction::RemoveFeature => ("feature", "remove"),
            UndoAction::ReorderFeatures => ("feature", "reorder"),
            UndoAction::EditFeature => ("feature", "edit"),
            UndoAction::AddUseCase => ("use_case", "add"),
            UndoAction::RemoveUseCase => ("use_case", "remove"),
            UndoAction::ReorderUseCases => ("use_case", "reorder"),
            UndoAction::EditUseCase => ("use_case", "edit"),
            UndoAction::EnableDto => ("dto", "enable"),
            UndoAction::DisableDto => ("dto", "disable"),
            UndoAction::EditDto => ("dto", "edit"),
            UndoAction::AddDtoField => ("dto_field", "add"),
            UndoAction::RemoveDtoField => ("dto_field", "remove"),
            UndoAction::ReorderDtoFields => ("dto_field", "reorder"),
            UndoAction::EditDtoField => ("dto_field", "edit"),
            UndoAction::EditUserInterface => ("user_interface", "edit"),
        }
    }

    /// Read a stored label back.
    ///
    /// `None` for anything this UI did not write, which includes every entry the
    /// backend recorded before labelling existed and anything a future version adds.
    /// The menu then falls back to a plain "Undo", which is the honest answer.
    pub fn of(label: &UndoLabel) -> Option<UndoAction> {
        UndoAction::ALL
            .into_iter()
            .find(|action| action.parts() == (label.subject, label.action))
    }

    /// What a menu row or a toast says.
    pub fn describe(self) -> LocalizedString {
        use teksilo::prelude::tr;
        match self {
            UndoAction::EditProject => tr!(undo_edit_project()),
            UndoAction::AddEntity => tr!(undo_add_entity()),
            UndoAction::RemoveEntity => tr!(undo_remove_entity()),
            UndoAction::ReorderEntities => tr!(undo_reorder_entities()),
            UndoAction::EditEntity => tr!(undo_edit_entity()),
            UndoAction::AddField => tr!(undo_add_field()),
            UndoAction::RemoveField => tr!(undo_remove_field()),
            UndoAction::ReorderFields => tr!(undo_reorder_fields()),
            UndoAction::EditField => tr!(undo_edit_field()),
            UndoAction::AddFeature => tr!(undo_add_feature()),
            UndoAction::RemoveFeature => tr!(undo_remove_feature()),
            UndoAction::ReorderFeatures => tr!(undo_reorder_features()),
            UndoAction::EditFeature => tr!(undo_edit_feature()),
            UndoAction::AddUseCase => tr!(undo_add_use_case()),
            UndoAction::RemoveUseCase => tr!(undo_remove_use_case()),
            UndoAction::ReorderUseCases => tr!(undo_reorder_use_cases()),
            UndoAction::EditUseCase => tr!(undo_edit_use_case()),
            UndoAction::EnableDto => tr!(undo_enable_dto()),
            UndoAction::DisableDto => tr!(undo_disable_dto()),
            UndoAction::EditDto => tr!(undo_edit_dto()),
            UndoAction::AddDtoField => tr!(undo_add_dto_field()),
            UndoAction::RemoveDtoField => tr!(undo_remove_dto_field()),
            UndoAction::ReorderDtoFields => tr!(undo_reorder_dto_fields()),
            UndoAction::EditDtoField => tr!(undo_edit_dto_field()),
            UndoAction::EditUserInterface => tr!(undo_edit_user_interface()),
        }
    }
}

/// Run one or more backend writes as a single named undo entry.
///
/// Every mutation in this app goes through here, for two reasons. A write that is
/// two commands has to be one entry or undo takes back half of it; and an entry has
/// to be named before it runs, because the backend stores the name with it.
///
/// The composite is ended rather than cancelled whatever `body` returns: the backend
/// records nothing for a command that failed, so an empty group is an empty group.
pub fn labeled<T>(
    ctx: &AppContext,
    stack: Option<u64>,
    action: UndoAction,
    body: impl FnOnce() -> T,
) -> T {
    if let Err(e) = undo_redo_commands::begin_composite_labeled(ctx, stack, Some(action.label())) {
        // Not fatal: the write is what matters, and an unnamed entry is better than
        // no entry at all.
        log::warn!("could not name the undo entry for {action:?}: {e}");
        return body();
    }
    let outcome = body();
    undo_redo_commands::end_composite(ctx);
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every action round-trips through the pair the backend stores, which is what
    /// lets the menu say what Undo would do.
    #[test]
    fn every_action_survives_the_backend() {
        for action in UndoAction::ALL {
            let stored = action.label();
            assert_eq!(
                UndoAction::of(&stored),
                Some(action),
                "{action:?} did not round-trip"
            );
        }
    }

    /// Two actions sharing a pair would make one of them unreadable: the menu would
    /// name the wrong operation, which is worse than naming none.
    #[test]
    fn no_two_actions_share_a_label() {
        let mut pairs: Vec<(&str, &str)> = UndoAction::ALL.iter().map(|a| a.parts()).collect();
        let before = pairs.len();
        pairs.sort_unstable();
        pairs.dedup();
        assert_eq!(before, pairs.len(), "two actions share a stored label");
    }

    /// And no two read the same on screen, or the menu would say the same thing for
    /// two different operations.
    #[test]
    fn no_two_actions_read_the_same() {
        let mut described: Vec<String> = UndoAction::ALL
            .iter()
            .map(|a| a.describe().resolve_now())
            .collect();
        let before = described.len();
        described.sort();
        described.dedup();
        assert_eq!(before, described.len(), "two actions describe the same");
    }

    /// An entry this UI did not write reads as unnamed rather than as the wrong
    /// thing. That covers every entry the backend recorded before labelling, and
    /// anything a later version adds.
    #[test]
    fn an_unknown_label_is_not_guessed_at() {
        assert_eq!(UndoAction::of(&UndoLabel::new("something", "else")), None);
        assert_eq!(UndoAction::of(&UndoLabel::new("entity", "")), None);
    }
}
