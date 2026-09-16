//! Which frontends the manifest asks Qleany to scaffold.
//!
//! Seven flags on one row, shown as two sets that never appear together: the
//! targets that exist for Rust, and the ones that exist for C++/Qt. Which set is
//! shown follows the project's language, so this view-model reads the Project
//! screen's rather than keeping a second copy of it.

use teksilo::prelude::*;

use frontend::EntityId;

use crate::app_ids::AppIds;
use crate::project::Language;
use crate::singles::SingleUserInterface;

/// One frontend Qleany can scaffold.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Target {
    RustCli,
    RustTeksilo,
    RustSlint,
    RustIos,
    RustAndroid,
    CppQtWidgets,
    CppQtQuick,
}

impl Target {
    /// The targets a language offers, in the order the screen lists them.
    pub fn for_language(language: Language) -> &'static [Target] {
        match language {
            Language::Rust => &[
                Target::RustCli,
                Target::RustTeksilo,
                Target::RustSlint,
                Target::RustIos,
                Target::RustAndroid,
            ],
            Language::CppQt => &[Target::CppQtWidgets, Target::CppQtQuick],
        }
    }

    pub fn label(self) -> LocalizedString {
        match self {
            Target::RustCli => tr!(ui_target_rust_cli()),
            Target::RustTeksilo => tr!(ui_target_rust_teksilo()),
            Target::RustSlint => tr!(ui_target_rust_slint()),
            Target::RustIos => tr!(ui_target_rust_ios()),
            Target::RustAndroid => tr!(ui_target_rust_android()),
            Target::CppQtWidgets => tr!(ui_target_cpp_qt_widgets()),
            Target::CppQtQuick => tr!(ui_target_cpp_qt_quick()),
        }
    }
}

#[derive(Clone)]
pub struct UserInterfaceViewModel {
    ids: AppIds,
    single: SingleUserInterface,
    /// The project's language, read rather than owned: the two screens must agree
    /// about which targets exist, and a second copy is a second thing to keep in
    /// step.
    language: Signal<Option<Language>>,
}

impl std::fmt::Debug for UserInterfaceViewModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UserInterfaceViewModel")
            .field("row", &self.single.id())
            .finish_non_exhaustive()
    }
}

impl UserInterfaceViewModel {
    pub fn new(
        single: SingleUserInterface,
        ids: AppIds,
        language: Signal<Option<Language>>,
    ) -> Self {
        Self {
            ids,
            single,
            language,
        }
    }

    // ── state a view binds ───────────────────────────────────────────────────

    pub fn language(&self) -> Signal<Option<Language>> {
        self.language.clone()
    }

    /// The targets on offer right now.
    pub fn targets(&self) -> &'static [Target] {
        Target::for_language(self.language.get().unwrap_or_default())
    }

    /// The signal behind one target's checkbox.
    ///
    /// The handle's own field signal, not a copy: a checkbox bound to it follows an
    /// undo, a reload and a manifest change without anything being mirrored.
    pub fn flag(&self, target: Target) -> Signal<bool> {
        match target {
            Target::RustCli => self.single.rust_cli(),
            Target::RustTeksilo => self.single.rust_teksilo(),
            Target::RustSlint => self.single.rust_slint(),
            Target::RustIos => self.single.rust_ios(),
            Target::RustAndroid => self.single.rust_android(),
            Target::CppQtWidgets => self.single.cpp_qt_qtwidgets(),
            Target::CppQtQuick => self.single.cpp_qt_qtquick(),
        }
    }

    // ── commands ─────────────────────────────────────────────────────────────

    /// Turn one target on or off, and write it.
    ///
    /// Through the handle's setter rather than through the bound signal: the signal
    /// is what the checkbox already wrote, and `save` needs the handle to know it is
    /// dirty. Setting the same value twice is what `set_if_changed` is for and costs
    /// a comparison.
    pub fn set(&self, target: Target, value: bool) {
        match target {
            Target::RustCli => self.single.set_rust_cli(value),
            Target::RustTeksilo => self.single.set_rust_teksilo(value),
            Target::RustSlint => self.single.set_rust_slint(value),
            Target::RustIos => self.single.set_rust_ios(value),
            Target::RustAndroid => self.single.set_rust_android(value),
            Target::CppQtWidgets => self.single.set_cpp_qt_qtwidgets(value),
            Target::CppQtQuick => self.single.set_cpp_qt_qtquick(value),
        }
        self.single.save(self.ids.user_interface_stack.get());
    }

    // ── wiring ───────────────────────────────────────────────────────────────

    pub fn wire(&self, ctx: &mut BuildContext) {
        let me = self.clone();
        let row = self.ids.user_interface_id.clone();
        ctx.effect(&row, move |id| me.point_at(*id));
        self.point_at(row.get());
    }

    fn point_at(&self, id: Option<EntityId>) {
        if self.single.id() == id {
            return;
        }
        self.single.set_id(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// US-UIT-01: each language offers its own targets and nothing else, so a Rust
    /// manifest never shows a Qt checkbox that would write a flag the generator
    /// ignores.
    #[test]
    fn each_language_offers_only_its_own_targets() {
        let rust = Target::for_language(Language::Rust);
        assert_eq!(rust.len(), 5);
        assert!(rust.contains(&Target::RustTeksilo));
        assert!(!rust.contains(&Target::CppQtQuick));

        let cpp = Target::for_language(Language::CppQt);
        assert_eq!(cpp, &[Target::CppQtWidgets, Target::CppQtQuick]);
    }

    /// Every target has its own label. A copy-and-paste slip here would put two
    /// identical checkboxes on the screen, which reads as a duplicate rather than
    /// as a mistake.
    #[test]
    fn every_target_is_labelled_distinctly() {
        let mut labels: Vec<String> = [Language::Rust, Language::CppQt]
            .into_iter()
            .flat_map(|l| Target::for_language(l).iter().copied())
            .map(|t| t.label().resolve_now())
            .collect();
        assert_eq!(labels.len(), 7);
        labels.sort();
        labels.dedup();
        assert_eq!(labels.len(), 7, "two targets share a label");
    }
}
