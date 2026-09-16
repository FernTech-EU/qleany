use std::cmp::Ordering;
use std::collections::BTreeMap;

/// Represents a node in the file tree structure.
/// Uses BTreeMap to maintain alphabetical ordering of children.
#[derive(Default)]
struct TreeNode {
    children: BTreeMap<String, TreeNode>,
    is_file: bool,
}

impl TreeNode {
    fn new() -> Self {
        Self {
            children: BTreeMap::new(),
            is_file: false,
        }
    }

    /// Inserts a path into the tree, creating intermediate nodes as needed.
    fn insert(&mut self, path: &str) {
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        self.insert_parts(&parts, 0);
    }

    fn insert_parts(&mut self, parts: &[&str], index: usize) {
        if index >= parts.len() {
            return;
        }

        let part = parts[index];
        let child = self.children.entry(part.to_string()).or_default();

        if index == parts.len() - 1 {
            child.is_file = true;
        } else {
            child.insert_parts(parts, index + 1);
        }
    }
}

/// Renders a list of file paths as an ASCII tree.
///
/// # Arguments
///
/// * `files` - A slice of file path strings (e.g., `["src/main.rs", "src/lib.rs"]`)
///
/// # Returns
///
/// A `String` containing the formatted ASCII tree representation.
///
/// # Example
///
/// ```
/// use teksilo_ui::cli_handlers::list::file_tree::render_file_tree;
///
/// let files = ["src/main.rs", "src/cli/mod.rs", "src/cli/args.rs", "Cargo.toml"];
/// let tree = render_file_tree(&files);
/// assert_eq!(
///     tree,
///     "\
/// ├── Cargo.toml
/// └── src/
///     ├── cli/
///     │   ├── args.rs
///     │   └── mod.rs
///     └── main.rs
/// "
/// );
/// ```
///
/// Top-level entries carry a connector too, unlike `tree(1)`, which prints the
/// directory it was given as a bare root above them. There is no such root here:
/// the paths are relative to a manifest, not to a directory that was walked.
///
/// This example went four years without running. `crates/slint_ui` was a
/// binary-only crate and cargo does not collect doctests from a binary, so the
/// snippet did not compile (it called the function with no path to it) and the
/// output block below it was wrong as well. Moving the CLI into a crate with a
/// `[lib]` is what surfaced both.
pub fn render_file_tree(files: &[impl AsRef<str>]) -> String {
    let mut root = TreeNode::new();

    for file in files {
        root.insert(file.as_ref());
    }

    let mut output = String::new();
    render_children(&root, "", &mut output);
    output
}

fn render_children(node: &TreeNode, prefix: &str, output: &mut String) {
    let mut children: Vec<_> = node.children.iter().collect();
    children.sort_by(|(name_a, _), (name_b, _)| {
        let ord = name_a.to_lowercase().cmp(&name_b.to_lowercase());
        if ord == Ordering::Equal {
            name_a.cmp(name_b)
        } else {
            ord
        }
    });
    let count = children.len();

    for (index, (name, child)) in children.into_iter().enumerate() {
        let is_last = index == count - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let child_prefix = if is_last { "    " } else { "│   " };

        // Append directory indicator if this node has children
        let display_name = if child.children.is_empty() {
            name.clone()
        } else {
            format!("{}/", name)
        };

        output.push_str(prefix);
        output.push_str(connector);
        output.push_str(&display_name);
        output.push('\n');

        // Recursively render children
        if !child.children.is_empty() {
            let new_prefix = format!("{}{}", prefix, child_prefix);
            render_children(child, &new_prefix, output);
        }
    }
}

/// Renders a file tree to stdout for convenience.
pub fn print_file_tree(files: &[impl AsRef<str>]) {
    print!("{}", render_file_tree(files));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_file() {
        let files = vec!["main.rs"];
        let result = render_file_tree(&files);
        assert_eq!(result, "└── main.rs\n");
    }

    #[test]
    fn test_flat_structure() {
        let files = vec!["Cargo.toml", "README.md", "main.rs"];
        let result = render_file_tree(&files);
        let expected = "\
├── Cargo.toml
├── main.rs
└── README.md
";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_nested_structure() {
        let files = vec![
            "src/main.rs",
            "src/lib.rs",
            "src/cli/mod.rs",
            "src/cli/args.rs",
            "Cargo.toml",
        ];
        let result = render_file_tree(&files);
        let expected = "\
├── Cargo.toml
└── src/
    ├── cli/
    │   ├── args.rs
    │   └── mod.rs
    ├── lib.rs
    └── main.rs
";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_deeply_nested() {
        let files = vec!["a/b/c/d/file.txt", "a/b/c/other.txt", "a/b/sibling.txt"];
        let result = render_file_tree(&files);
        let expected = "\
└── a/
    └── b/
        ├── c/
        │   ├── d/
        │   │   └── file.txt
        │   └── other.txt
        └── sibling.txt
";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_empty_input() {
        let files: Vec<&str> = vec![];
        let result = render_file_tree(&files);
        assert_eq!(result, "");
    }

    #[test]
    fn test_string_slices() {
        let files = vec![String::from("src/main.rs"), String::from("Cargo.toml")];
        let result = render_file_tree(&files);
        assert!(result.contains("src/"));
        assert!(result.contains("main.rs"));
        assert!(result.contains("Cargo.toml"));
    }

    #[test]
    fn test_multiple_root_directories() {
        let files = vec![
            "docs/guide.md",
            "docs/api.md",
            "src/main.rs",
            "tests/test_main.rs",
        ];
        let result = render_file_tree(&files);
        let expected = "\
├── docs/
│   ├── api.md
│   └── guide.md
├── src/
│   └── main.rs
└── tests/
    └── test_main.rs
";
        assert_eq!(result, expected);
    }
}
