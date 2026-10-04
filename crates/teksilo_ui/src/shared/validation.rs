//! What a name has to look like, and how a form says so.
//!
//! Qleany turns the names in a manifest into Rust type names, file names and
//! module paths, so a name that is not in the expected case does not produce odd
//! output, it produces output that does not compile. The rules are the generator's
//! own, checked here as the user types rather than at generation time, when the
//! manifest is already written and the user has moved on.

use teksilo::prelude::*;
use teksilo::widgets::ValidationState;

/// Whether a name is already PascalCase.
///
/// Expressed as a round trip through `heck` rather than as a hand-written character
/// rule, so the UI's verdict cannot drift from what the generator will actually do
/// with the name. This is the Slint UI's rule verbatim, and keeping it identical is
/// the point: a manifest that passed there must pass here.
pub fn is_pascal_case(name: &str) -> bool {
    name == heck::AsPascalCase(name).to_string()
}

/// Whether a name is already snake_case. See [`is_pascal_case`].
pub fn is_snake_case(name: &str) -> bool {
    name == heck::AsSnakeCase(name).to_string()
}

/// A required text field's validation state, derived from its own value.
///
/// Whitespace counts as empty: a name of three spaces passes a bare `is_empty` and
/// then generates a crate whose name begins with a space.
pub fn required(value: &Signal<String>, message: LocalizedString) -> Signal<ValidationState> {
    value.map(move |v| {
        if v.trim().is_empty() {
            ValidationState::Error(message.clone())
        } else {
            ValidationState::None
        }
    })
}

/// A required field that must also be in a given case.
///
/// Two messages, not one: "Entity name is required" and "Entity name must be in
/// PascalCase" are different problems with different fixes, and a field that
/// answered both with one message would be telling a user who typed `my entity`
/// that they typed nothing.
pub fn required_cased(
    value: &Signal<String>,
    empty_message: LocalizedString,
    case_message: LocalizedString,
    is_well_cased: fn(&str) -> bool,
) -> Signal<ValidationState> {
    value.map(move |v| {
        if v.trim().is_empty() {
            ValidationState::Error(empty_message.clone())
        } else if !is_well_cased(v) {
            ValidationState::Error(case_message.clone())
        } else {
            ValidationState::None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pascal_case_accepts_what_the_generator_would_leave_alone() {
        for name in ["Entity", "UseCase", "DtoField", "A"] {
            assert!(is_pascal_case(name), "{name}");
        }
        for name in ["entity", "use_case", "Use Case", "useCase", "Use-Case"] {
            assert!(!is_pascal_case(name), "{name}");
        }
    }

    #[test]
    fn snake_case_accepts_what_the_generator_would_leave_alone() {
        for name in ["name", "created_at", "only_for_heritage", "a"] {
            assert!(is_snake_case(name), "{name}");
        }
        for name in ["Name", "createdAt", "created at", "created-at"] {
            assert!(!is_snake_case(name), "{name}");
        }
    }

    /// An empty name and a badly cased one are different problems, and the field
    /// has to say which one it is.
    #[test]
    fn the_two_messages_are_told_apart() {
        let value = Signal::new(String::new());
        let state = required_cased(
            &value,
            lit!("required"),
            lit!("must be PascalCase"),
            is_pascal_case,
        );

        assert!(matches!(&state.get(), ValidationState::Error(m) if m.resolve_now() == "required"));

        value.set("my entity".to_string());
        assert!(
            matches!(&state.get(), ValidationState::Error(m) if m.resolve_now() == "must be PascalCase")
        );

        value.set("MyEntity".to_string());
        assert!(matches!(state.get(), ValidationState::None));
    }

    #[test]
    fn a_required_field_rejects_whitespace() {
        let value = Signal::new("   ".to_string());
        let state = required(&value, lit!("required"));
        assert!(matches!(state.get(), ValidationState::Error(_)));

        value.set("Demo".to_string());
        assert!(matches!(state.get(), ValidationState::None));
    }
}
