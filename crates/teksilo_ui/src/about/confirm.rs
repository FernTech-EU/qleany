//! Asking before a delete that takes more than the row with it.
//!
//! Not every delete asks. A field or a DTO field is one row and Undo covers it, so
//! a dialog there is a speed bump rather than a safeguard. An entity, a feature or
//! a use case is the root of a subtree the backend cascades through, and the user
//! cannot see from the row how much goes with it.

use teksilo::prelude::*;
use teksilo::widgets::{MessageBox, MessageBoxButtons, StandardButton};

/// What a delete would take with it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cascade {
    /// An entity: its fields and its relationships.
    Entity,
    /// A feature: its use cases, and their DTOs.
    Feature,
    /// A use case: its input and output DTOs.
    UseCase,
}

impl Cascade {
    /// The sentence that names what goes.
    ///
    /// US-SAFE-02: the object by its own name, never "this item", and the button
    /// says what it will do. A dialog that says "Are you sure?" over a row the user
    /// can no longer see is a dialog they learn to dismiss without reading.
    fn message(self, name: &str) -> LocalizedString {
        let name = name.to_string();
        match self {
            Cascade::Entity => tr!(confirm_delete_entity(name = name)),
            Cascade::Feature => tr!(confirm_delete_feature(name = name)),
            Cascade::UseCase => tr!(confirm_delete_use_case(name = name)),
        }
    }

    fn title(self) -> LocalizedString {
        match self {
            Cascade::Entity => tr!(confirm_delete_entity_title()),
            Cascade::Feature => tr!(confirm_delete_feature_title()),
            Cascade::UseCase => tr!(confirm_delete_use_case_title()),
        }
    }
}

/// Ask, then delete.
///
/// `YesNo`, which teksilo defaults to **No** and makes No the escape: Enter takes
/// the safe answer to a destructive question, which is the whole reason the preset
/// exists.
pub fn confirm_delete(
    ctx: &mut EventContext,
    cascade: Cascade,
    name: &str,
    delete: impl Fn() + 'static,
) {
    MessageBox::question(cascade.title())
        .text(cascade.message(name))
        .buttons(MessageBoxButtons::YesNo)
        .on_result(move |result, _c| {
            if result.button == StandardButton::Yes {
                delete();
            }
        })
        .present(ctx);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// US-SAFE-02: the object by its own name, and what goes with it.
    #[test]
    fn a_confirmation_names_the_row_and_the_cascade() {
        let entity = Cascade::Entity.message("Workspace").resolve_now();
        assert!(entity.contains("Workspace"), "{entity}");
        assert!(entity.contains("fields"), "{entity}");
        assert!(!entity.contains("this item"));

        let feature = Cascade::Feature.message("handling_manifest").resolve_now();
        assert!(feature.contains("handling_manifest"), "{feature}");
        assert!(feature.contains("use cases"), "{feature}");

        let use_case = Cascade::UseCase.message("load").resolve_now();
        assert!(use_case.contains("load"), "{use_case}");
        assert!(use_case.contains("DTO"), "{use_case}");
    }

    /// Three kinds, three sentences: a shared one would have to say "this item".
    #[test]
    fn each_cascade_reads_differently() {
        let mut said: Vec<String> = [Cascade::Entity, Cascade::Feature, Cascade::UseCase]
            .iter()
            .map(|c| c.message("Thing").resolve_now())
            .collect();
        said.sort();
        said.dedup();
        assert_eq!(said.len(), 3);
    }
}
