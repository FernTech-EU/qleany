//! English pluralization for the identifiers Qleany generates.
//!
//! Generated code names collections after the entity they hold: an entity
//! `Category` becomes a `categories` table on the store, a `categories`
//! parameter on `create_multi`, and a `m_categories` member in C++. All three
//! generators and the manifest checker must agree on that spelling, which is
//! why the rules live here rather than in each of them.
//!
//! There is one deliberate second copy: `macros_direct_access.tera` emits the
//! same tables into every generated project's proc-macro crate, because that
//! crate is standalone and cannot depend on this one. `tests/template_parity.rs`
//! fails if the two drift.

// --- BEGIN canonical pluralizer ---
//
// This block is mirrored VERBATIM between `crates/naming/src/lib.rs` (the
// source of truth) and `macros_direct_access.tera`, because every generated
// project ships its own copy inside a standalone proc-macro crate that cannot
// depend on `qleany-naming`. `crates/naming/tests/template_parity.rs` compares
// the two character by character and fails if they drift.
//
// Editing rules: change `crates/naming/src/lib.rs` first, then copy the whole
// block across. Keep it dependency-free (no `use`), and free of the three
// sequences Tera would interpret when rendering the template: a doubled brace,
// a brace-percent, and a brace-hash.

/// Words that are already plural, so pluralizing them again produces nonsense
/// (`settings` -> `settingses`).
///
/// Three kinds qualify: *pluralia tantum* that have no singular form in
/// practice (`settings`, `news`), mass nouns (`metadata`), and the plural half
/// of `IRREGULAR` -- an entity named `Children` must not become `childrens`.
const ALREADY_PLURAL: &[&str] = &[
    // pluralia tantum and mass nouns
    "settings",
    "preferences",
    "credentials",
    "permissions",
    "statistics",
    "analytics",
    "metadata",
    "news",
    // the plural half of IRREGULAR
    "children",
    "people",
    "men",
    "women",
    "mice",
    "geese",
    "feet",
    "teeth",
    "oxen",
    "data",
    "indices",
    "matrices",
    "vertices",
    "appendices",
    "criteria",
    "phenomena",
    "media",
    "curricula",
    "dice",
];

/// Nouns whose plural is not formed by suffixing.
const IRREGULAR: &[(&str, &str)] = &[
    ("child", "children"),
    ("person", "people"),
    ("man", "men"),
    ("woman", "women"),
    ("mouse", "mice"),
    ("goose", "geese"),
    ("foot", "feet"),
    ("tooth", "teeth"),
    ("ox", "oxen"),
    ("datum", "data"),
    ("index", "indices"),
    ("matrix", "matrices"),
    ("vertex", "vertices"),
    ("appendix", "appendices"),
    ("criterion", "criteria"),
    ("phenomenon", "phenomena"),
    ("medium", "media"),
    ("curriculum", "curricula"),
    ("die", "dice"),
];

/// Nouns whose plural is spelled the same as their singular.
const UNCOUNTABLE: &[&str] = &[
    "sheep",
    "fish",
    "deer",
    "species",
    "series",
    "aircraft",
    "offspring",
    "moose",
];

const FE_TO_VES: &[&str] = &["knife", "life", "wife", "midwife"];

const F_TO_VES: &[&str] = &[
    "leaf", "half", "wolf", "shelf", "self", "calf", "loaf", "thief", "sheaf", "elf", "scarf",
];

const US_TO_I: &[&str] = &[
    "focus", "radius", "fungus", "cactus", "stimulus", "syllabus", "nucleus", "alumnus",
];

const O_TO_OES: &[&str] = &[
    "hero", "potato", "tomato", "echo", "torpedo", "veto", "embargo", "volcano", "mosquito",
    "cargo",
];

/// Pluralizes a single English word carrying no word boundary of its own.
fn pluralize_single(word: &str) -> String {
    if word.is_empty() {
        return String::new();
    }

    let lower = word.to_lowercase();

    for &(singular, plural) in IRREGULAR {
        if lower == singular {
            if word.chars().next().is_some_and(char::is_uppercase) {
                let mut chars = plural.chars();
                // Every `plural` above is a non-empty literal, so this branch
                // cannot be taken with an empty iterator -- but expressing that
                // as a `match` costs nothing and keeps the helper panic-free by
                // construction rather than by reading the table above.
                match chars.next() {
                    Some(first) => {
                        return format!("{}{}", first.to_uppercase(), chars.as_str());
                    }
                    None => return plural.to_string(),
                }
            }
            return plural.to_string();
        }
    }

    if UNCOUNTABLE.contains(&lower.as_str()) || ALREADY_PLURAL.contains(&lower.as_str()) {
        return word.to_string();
    }

    // Words ending in -fe -> -ves
    if FE_TO_VES.contains(&lower.as_str()) {
        let stem = &word[..word.len() - 2];
        return format!("{}ves", stem);
    }

    // Words ending in -f -> -ves (common cases)
    if F_TO_VES.contains(&lower.as_str()) {
        let stem = &word[..word.len() - 1];
        return format!("{}ves", stem);
    }

    // Words ending in -sis or -xis -> -ses / -xes (Latin/Greek).
    // The final three bytes are ASCII, so `len - 2` is a char boundary.
    if lower.ends_with("sis") || lower.ends_with("xis") {
        return format!("{}es", &word[..word.len() - 2]);
    }

    // Words ending in -us -> -i (Latin, common cases)
    if US_TO_I.contains(&lower.as_str()) {
        return format!("{}i", &word[..word.len() - 2]);
    }

    // Words ending in -o preceded by a consonant -> -oes (common cases)
    if O_TO_OES.contains(&lower.as_str()) {
        return format!("{}es", word);
    }

    // Words ending in 'y' preceded by a consonant: change 'y' to 'ies'.
    // `word.len()` is BYTES while `chars().nth()` counts CHARS, so indexing by
    // byte length would run past the end for any word with a multi-byte
    // character. Walking backwards over `chars` is both correct and shorter.
    if word.ends_with('y') {
        let mut chars = word.chars().rev();
        chars.next(); // the trailing 'y'
        if let Some(second_last) = chars.next()
            && !"aeiou".contains(second_last)
        {
            let stem: String = word.chars().take(word.chars().count() - 1).collect();
            return format!("{stem}ies");
        }
    }

    // Words ending in 's', 'x', 'z', 'ch', 'sh': add 'es'
    if word.ends_with('s')
        || word.ends_with('x')
        || word.ends_with('z')
        || word.ends_with("ch")
        || word.ends_with("sh")
    {
        return format!("{}es", word);
    }

    // Default case: add 's'
    format!("{}s", word)
}

/// Byte index at which the final word of `word` starts.
///
/// Only the final word is pluralized, so `deleted_tag` -> `deleted_tags` rather
/// than `deleteds_tags`. Both naming conventions Qleany generates must be
/// recognised: the Rust generator passes snake_case (`project_settings`) while
/// the C++ generator passes PascalCase (`ProjectSettings`). Splitting on `_`
/// alone would leave every PascalCase compound unsegmented, so `ProjectSettings`
/// would be looked up whole, miss `ALREADY_PLURAL`, and come back as
/// `ProjectSettingses`.
fn last_word_start(word: &str) -> usize {
    // An underscore is unambiguous, so snake_case wins when it is present.
    if let Some(pos) = word.rfind('_') {
        return pos + 1;
    }

    // Otherwise take the last lowercase-or-digit followed by an uppercase.
    // Requiring the *previous* character to be lowercase keeps acronyms whole:
    // `HTTPRequest` has no such boundary and stays one word.
    let mut start = 0;
    let mut previous: Option<char> = None;
    for (index, character) in word.char_indices() {
        if let Some(p) = previous
            && character.is_uppercase()
            && (p.is_lowercase() || p.is_numeric())
        {
            start = index;
        }
        previous = Some(character);
    }
    start
}

/// Transforms an English word to its plural form, pluralizing only its final
/// word so that both `deleted_tag` -> `deleted_tags` and
/// `ProjectSettings` -> `ProjectSettings` come out right.
pub fn to_plural(word: &str) -> String {
    if word.is_empty() {
        return String::new();
    }

    let (prefix, last) = word.split_at(last_word_start(word));
    format!("{}{}", prefix, pluralize_single(last))
}
// --- END canonical pluralizer ---

#[cfg(test)]
mod tests {
    use super::to_plural;

    #[test]
    fn regular_words_take_a_trailing_s() {
        assert_eq!(to_plural("project"), "projects");
        assert_eq!(to_plural("user"), "users");
        assert_eq!(to_plural("root"), "roots");
    }

    #[test]
    fn consonant_y_becomes_ies() {
        assert_eq!(to_plural("entity"), "entities");
        assert_eq!(to_plural("category"), "categories");
        assert_eq!(to_plural("property"), "properties");
        // vowel + y keeps the y
        assert_eq!(to_plural("day"), "days");
        assert_eq!(to_plural("key"), "keys");
    }

    #[test]
    fn sibilants_take_es() {
        assert_eq!(to_plural("status"), "statuses");
        assert_eq!(to_plural("box"), "boxes");
        assert_eq!(to_plural("branch"), "branches");
        assert_eq!(to_plural("dish"), "dishes");
        assert_eq!(to_plural("class"), "classes");
    }

    #[test]
    fn irregulars_use_the_table() {
        assert_eq!(to_plural("child"), "children");
        assert_eq!(to_plural("person"), "people");
        assert_eq!(to_plural("datum"), "data");
        assert_eq!(to_plural("index"), "indices");
    }

    #[test]
    fn irregulars_preserve_leading_case() {
        assert_eq!(to_plural("Child"), "Children");
        assert_eq!(to_plural("Person"), "People");
    }

    #[test]
    fn uncountables_are_unchanged() {
        assert_eq!(to_plural("sheep"), "sheep");
        assert_eq!(to_plural("series"), "series");
        assert_eq!(to_plural("species"), "species");
    }

    #[test]
    fn already_plural_words_are_not_pluralized_twice() {
        // The motivating case: an entity named `ProjectSettings` used to
        // generate a `project_settingses` table.
        assert_eq!(to_plural("project_settings"), "project_settings");
        assert_eq!(to_plural("settings"), "settings");
        assert_eq!(to_plural("preferences"), "preferences");
        assert_eq!(to_plural("credentials"), "credentials");
        assert_eq!(to_plural("metadata"), "metadata");
        assert_eq!(to_plural("news"), "news");
    }

    #[test]
    fn the_plural_half_of_the_irregular_table_is_a_fixed_point() {
        for (_, plural) in super::IRREGULAR {
            assert_eq!(&to_plural(plural), plural, "{plural} was pluralized twice");
        }
    }

    #[test]
    fn f_and_fe_become_ves() {
        assert_eq!(to_plural("leaf"), "leaves");
        assert_eq!(to_plural("knife"), "knives");
        assert_eq!(to_plural("shelf"), "shelves");
    }

    #[test]
    fn latin_and_greek_endings() {
        assert_eq!(to_plural("analysis"), "analyses");
        assert_eq!(to_plural("basis"), "bases");
        assert_eq!(to_plural("axis"), "axes");
        assert_eq!(to_plural("radius"), "radii");
    }

    #[test]
    fn only_the_last_pascal_case_segment_is_pluralized() {
        // The C++ generator passes PascalCase entity names straight through, so
        // splitting on '_' alone left every compound unsegmented: `MenuChild`
        // came back as `MenuChilds` and `ProjectSettings` as `ProjectSettingses`.
        assert_eq!(to_plural("ProjectSettings"), "ProjectSettings");
        assert_eq!(to_plural("UserPreferences"), "UserPreferences");
        assert_eq!(to_plural("MenuChild"), "MenuChildren");
        assert_eq!(to_plural("DataSeries"), "DataSeries");
        assert_eq!(to_plural("UseCase"), "UseCases");
        assert_eq!(to_plural("DtoField"), "DtoFields");
        assert_eq!(to_plural("UserInterface"), "UserInterfaces");
        // camelCase is segmented the same way
        assert_eq!(to_plural("projectSettings"), "projectSettings");
        assert_eq!(to_plural("menuChild"), "menuChildren");
    }

    #[test]
    fn acronyms_are_not_split() {
        // A boundary needs a lowercase-or-digit before the uppercase, so a run
        // of capitals stays one word instead of pluralizing its last letter.
        assert_eq!(to_plural("HTTPRequest"), "HTTPRequests");
        assert_eq!(to_plural("XMLDto"), "XMLDtos");
        assert_eq!(to_plural("IPv4Address"), "IPv4Addresses");
    }

    #[test]
    fn the_two_casings_agree_on_which_word_is_pluralized() {
        // The Rust generator feeds snake_case and the C++ generator PascalCase.
        // Check rule C48 detects collisions in snake space only, so the two must
        // segment the same name identically or C48 would be blind to a C++-only
        // collision.
        for (pascal, snake) in [
            ("ProjectSettings", "project_settings"),
            ("MenuChild", "menu_child"),
            ("UseCase", "use_case"),
            ("DataSeries", "data_series"),
        ] {
            let from_pascal = to_plural(pascal).to_lowercase().replace('_', "");
            let from_snake = to_plural(snake).replace('_', "");
            assert_eq!(from_pascal, from_snake, "{pascal} vs {snake}");
        }
    }

    #[test]
    fn only_the_last_snake_case_segment_is_pluralized() {
        assert_eq!(to_plural("deleted_tag"), "deleted_tags");
        assert_eq!(to_plural("user_interface"), "user_interfaces");
        assert_eq!(to_plural("dto_field"), "dto_fields");
        assert_eq!(to_plural("use_case"), "use_cases");
    }

    #[test]
    fn empty_input_stays_empty() {
        assert_eq!(to_plural(""), "");
        assert_eq!(to_plural("_"), "_");
    }

    #[test]
    fn multi_byte_input_does_not_panic() {
        // `word.len()` counts bytes; these used to index past a char boundary.
        assert_eq!(to_plural("café"), "cafés");
        assert_eq!(to_plural("naïve"), "naïves");
        assert_eq!(to_plural("日本"), "日本s");
        assert_eq!(to_plural("é"), "és");
        // 'y' preceded by a multi-byte consonant-ish char
        let _ = to_plural("ény");
    }

    #[test]
    fn qleanys_own_entities_pluralize_sensibly() {
        for (singular, expected) in [
            ("root", "roots"),
            ("workspace", "workspaces"),
            ("system", "systems"),
            ("entity", "entities"),
            ("field", "fields"),
            ("feature", "features"),
            ("file", "files"),
            ("use_case", "use_cases"),
            ("dto", "dtos"),
            ("dto_field", "dto_fields"),
            ("global", "globals"),
            ("relationship", "relationships"),
            ("user_interface", "user_interfaces"),
        ] {
            assert_eq!(to_plural(singular), expected);
        }
    }
}
