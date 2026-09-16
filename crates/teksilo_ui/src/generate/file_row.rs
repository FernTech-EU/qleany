//! What the file list holds, and what it hides.
//!
//! The generated `SystemFilesRow` carries `generated_code`, which is the whole body
//! of every file Qleany would write: a few megabytes for a manifest of any size, and
//! cloned on every refresh. The list does not need it, so this module projects the
//! rows it does need and the filters that decide which of those are shown.
//!
//! Everything here is a plain value over plain values. The filtering rules are the
//! part a user notices when they are wrong, and they are checkable without a window.

use frontend::EntityId;
use frontend::common::entities::{FileNature, FileStatus};

use crate::models::SystemFilesRow;

/// Where the elision starts. Past this many characters a path is shown from its
/// right-hand end, because the end is the part that identifies the file.
const MAX_PATH_LENGTH: usize = 50;

/// The synthetic first row of the group list.
///
/// Not a group in the manifest: a group is a directory Qleany writes into, and
/// "everything" is not one of those. It is first, and selected at launch.
pub const ALL_GROUPS: &str = "All";

/// One row of the file list, without the file's contents.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct FileRow {
    pub id: EntityId,
    pub name: String,
    pub relative_path: String,
    pub group: String,
    pub status: FileStatus,
    pub nature: FileNature,
}

impl FileRow {
    /// Project a generated row, leaving its body behind.
    pub fn of(row: &SystemFilesRow) -> Self {
        Self {
            id: row.id,
            name: row.name.clone(),
            relative_path: row.relative_path.clone(),
            group: row.group.clone(),
            status: row.status.clone(),
            nature: row.nature.clone(),
        }
    }

    /// The path as it appears in the project.
    ///
    /// `relative_path` always carries a trailing slash, which is what the
    /// generator's own file matching relies on, so this is a join rather than a
    /// `Path::join`.
    pub fn full_path(&self) -> String {
        format!("{}{}", self.relative_path, self.name)
    }

    /// The path split for display: everything up to and including the last slash,
    /// then the file name.
    ///
    /// The file name is what a user scans for, so it is the part shown in bold; the
    /// directory is context. Past fifty characters the *left* is dropped, because
    /// generated paths share long prefixes and differ at their ends: eliding the
    /// right would turn a screenful of distinct files into a screenful of
    /// `crates/common/src/direct_ac…`.
    pub fn display_parts(&self) -> (String, String) {
        elide_path(&self.full_path())
    }
}

/// See [`FileRow::display_parts`].
pub fn elide_path(path: &str) -> (String, String) {
    let shown = if path.chars().count() > MAX_PATH_LENGTH {
        let tail: String = path
            .chars()
            .skip(path.chars().count() - MAX_PATH_LENGTH)
            .collect();
        format!("…{tail}")
    } else {
        path.to_string()
    };

    match shown.rfind('/') {
        Some(slash) => (shown[..=slash].to_string(), shown[slash + 1..].to_string()),
        None => (String::new(), shown),
    }
}

/// What the file list is showing.
///
/// The defaults are the ones the Slint UI started every launch with, and they are
/// the useful ones: a generation run is almost always about what changed, and the
/// unchanged files are the bulk of the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Filters {
    pub modified: bool,
    pub new: bool,
    pub unchanged: bool,
    pub infrastructure: bool,
    pub aggregate: bool,
    pub scaffold: bool,
    /// Matched case-insensitively against the path and the name together.
    pub text: String,
    /// `None` for every group.
    pub group: Option<String>,
}

impl Default for Filters {
    fn default() -> Self {
        Self {
            modified: true,
            new: true,
            // Off: an unchanged file is one there is nothing to do about, and on a
            // real manifest they outnumber the rest several times over.
            unchanged: false,
            infrastructure: true,
            aggregate: true,
            scaffold: true,
            text: String::new(),
            group: None,
        }
    }
}

impl Filters {
    /// Whether a row is shown. All four kinds of filter combine.
    pub fn shows(&self, row: &FileRow) -> bool {
        self.shows_status(&row.status)
            && self.shows_nature(&row.nature)
            && self.shows_group(&row.group)
            && self.shows_text(row)
    }

    fn shows_status(&self, status: &FileStatus) -> bool {
        match status {
            FileStatus::Modified => self.modified,
            FileStatus::New => self.new,
            FileStatus::Unchanged => self.unchanged,
            // A file whose status has not been computed yet is always shown: hiding
            // it would make the list shrink and grow again as the status pass runs,
            // which reads as files appearing from nowhere.
            FileStatus::Unknown => true,
        }
    }

    fn shows_nature(&self, nature: &FileNature) -> bool {
        match nature {
            FileNature::Infrastructure => self.infrastructure,
            FileNature::Aggregate => self.aggregate,
            FileNature::Scaffold => self.scaffold,
        }
    }

    fn shows_group(&self, group: &str) -> bool {
        match &self.group {
            None => true,
            Some(wanted) => wanted == ALL_GROUPS || wanted == group,
        }
    }

    fn shows_text(&self, row: &FileRow) -> bool {
        if self.text.trim().is_empty() {
            return true;
        }
        let needle = self.text.to_lowercase();
        row.full_path().to_lowercase().contains(&needle)
    }
}

/// Every group in the list, "All" first and the rest alphabetically.
///
/// Sorted rather than left in manifest order: a group is a directory, the list is
/// read to find one, and manifest order is the order the generator happened to
/// enumerate templates in.
pub fn groups_of(rows: &[FileRow]) -> Vec<String> {
    let mut groups: Vec<String> = rows.iter().map(|row| row.group.clone()).collect();
    groups.sort_unstable();
    groups.dedup();
    std::iter::once(ALL_GROUPS.to_string())
        .chain(groups)
        .collect()
}

/// The colour of a row's status stripe, as a text role.
///
/// A role rather than a colour so the stripe follows the theme; the status meanings
/// are the same three the rest of the app uses for "needs attention", "new" and
/// "nothing to do".
pub fn status_role(status: &FileStatus) -> teksilo::prelude::TextRole {
    use teksilo::prelude::TextRole;
    match status {
        FileStatus::Modified => TextRole::Warning,
        FileStatus::New => TextRole::Success,
        FileStatus::Unchanged => TextRole::Disabled,
        FileStatus::Unknown => TextRole::Secondary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(name: &str, path: &str, group: &str, status: FileStatus, nature: FileNature) -> FileRow {
        FileRow {
            id: 1,
            name: name.to_string(),
            relative_path: path.to_string(),
            group: group.to_string(),
            status,
            nature,
        }
    }

    fn sample() -> FileRow {
        row(
            "entity_repository.rs",
            "crates/common/src/direct_access/entity/",
            "common",
            FileStatus::Modified,
            FileNature::Infrastructure,
        )
    }

    /// US-GEN-04: the defaults, which are what every launch starts from.
    #[test]
    fn the_defaults_show_what_there_is_something_to_do_about() {
        let filters = Filters::default();
        assert!(filters.modified);
        assert!(filters.new);
        assert!(
            !filters.unchanged,
            "unchanged files are the bulk of the list"
        );
        assert!(filters.infrastructure && filters.aggregate && filters.scaffold);
        assert!(filters.text.is_empty());
        assert_eq!(filters.group, None);
    }

    #[test]
    fn a_status_filter_hides_only_that_status() {
        let filters = Filters::default();
        assert!(filters.shows(&sample()));
        assert!(!filters.shows(&row(
            "a",
            "b/",
            "g",
            FileStatus::Unchanged,
            FileNature::Infrastructure
        )));
        assert!(filters.shows(&row(
            "a",
            "b/",
            "g",
            FileStatus::New,
            FileNature::Infrastructure
        )));
    }

    /// A file whose status has not been computed is shown whatever the filters say,
    /// or the list shrinks and grows again while the status pass runs.
    #[test]
    fn an_uncomputed_status_is_always_shown() {
        let filters = Filters {
            modified: false,
            new: false,
            unchanged: false,
            ..Filters::default()
        };
        assert!(filters.shows(&row(
            "a",
            "b/",
            "g",
            FileStatus::Unknown,
            FileNature::Infrastructure
        )));
    }

    #[test]
    fn a_nature_filter_hides_only_that_nature() {
        let filters = Filters {
            infrastructure: false,
            ..Filters::default()
        };
        assert!(!filters.shows(&sample()));
        assert!(filters.shows(&row("a", "b/", "g", FileStatus::New, FileNature::Aggregate)));
    }

    /// US-GEN-04: case-insensitive, over the path and the name together.
    #[test]
    fn the_text_filter_matches_the_whole_path() {
        let with = |text: &str| Filters {
            text: text.to_string(),
            ..Filters::default()
        };
        assert!(
            with("DIRECT_ACCESS").shows(&sample()),
            "matching is case-insensitive"
        );
        assert!(with("repository").shows(&sample()), "the name counts too");
        assert!(!with("nothing like it").shows(&sample()));
        assert!(with("   ").shows(&sample()), "whitespace is not a filter");
    }

    /// US-GEN-02: exactly one group at a time, and "All" is not a group.
    #[test]
    fn the_group_filter_is_one_at_a_time() {
        let with = |group: &str| Filters {
            group: Some(group.to_string()),
            ..Filters::default()
        };
        assert!(with("common").shows(&sample()));
        assert!(!with("cli").shows(&sample()));
        assert!(with(ALL_GROUPS).shows(&sample()), "All is every group");
    }

    #[test]
    fn all_four_filters_combine() {
        let filters = Filters {
            text: "repository".to_string(),
            group: Some("common".to_string()),
            ..Filters::default()
        };
        assert!(filters.shows(&sample()));

        let narrowed = Filters {
            modified: false,
            ..filters
        };
        assert!(!narrowed.shows(&sample()), "one no is enough");
    }

    /// US-GEN-02: "All" first, then the groups alphabetically and once each.
    #[test]
    fn the_group_list_starts_with_all_and_is_sorted() {
        let rows = vec![
            row(
                "a",
                "x/",
                "cli",
                FileStatus::New,
                FileNature::Infrastructure,
            ),
            row(
                "b",
                "x/",
                "common",
                FileStatus::New,
                FileNature::Infrastructure,
            ),
            row(
                "c",
                "x/",
                "cli",
                FileStatus::New,
                FileNature::Infrastructure,
            ),
        ];
        assert_eq!(groups_of(&rows), vec!["All", "cli", "common"]);
    }

    #[test]
    fn the_group_list_of_nothing_still_offers_all() {
        assert_eq!(groups_of(&[]), vec!["All"]);
    }

    /// US-GEN-03: the name is bold, the directory is context.
    #[test]
    fn a_short_path_is_split_at_its_last_slash() {
        let (prefix, name) = elide_path("crates/cli/src/main.rs");
        assert_eq!(prefix, "crates/cli/src/");
        assert_eq!(name, "main.rs");
    }

    #[test]
    fn a_path_with_no_slash_is_all_name() {
        let (prefix, name) = elide_path("Cargo.toml");
        assert_eq!(prefix, "");
        assert_eq!(name, "Cargo.toml");
    }

    /// US-GEN-03: elided from the **left**. Generated paths share long prefixes and
    /// differ at their ends, so eliding the right would make a screenful of distinct
    /// files look identical.
    #[test]
    fn a_long_path_keeps_its_end() {
        let path = "crates/common/src/direct_access/entity/entity_repository_and_more.rs";
        let (prefix, name) = elide_path(path);
        assert!(
            prefix.starts_with('…'),
            "the elision is on the left: {prefix:?}"
        );
        assert_eq!(name, "entity_repository_and_more.rs");
        assert!(
            format!("{prefix}{name}").chars().count() <= MAX_PATH_LENGTH + 1,
            "the shown path fits"
        );
    }

    #[test]
    fn the_full_path_joins_without_a_second_slash() {
        assert_eq!(
            sample().full_path(),
            "crates/common/src/direct_access/entity/entity_repository.rs"
        );
    }
}
