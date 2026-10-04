//! The application body: the navigation rail and the screen it selects.

pub mod commands;
pub mod nav;

use teksilo::prelude::*;
use teksilo::widgets::{Expand, HStack, Switcher};

use crate::app::commands::CommandDeps;
use crate::app::nav::NavRail;

use crate::app_ids::{AppIds, Screen};
use crate::check::CheckViewModel;
use crate::demo::DemoViewModel;
use crate::edit::UndoViewModel;
use crate::entities::{EntitiesPage, EntitiesViewModel, FieldViewModel};
use crate::features::{DtoSide, DtoViewModel, FeaturesPage, FeaturesViewModel, UseCaseViewModel};
use crate::generate::{GeneratePage, GenerateViewModel};
use crate::home::{self, HomeViewModel};
use crate::manifest::ManifestViewModel;
use crate::new_manifest::NewManifestViewModel;
use crate::project::{ProjectPage, ProjectViewModel};
use crate::session::Session;
use crate::shell::menus::MenuParts;
use crate::user_interface::{UserInterfacePage, UserInterfaceViewModel};

pub struct App {
    session: Session,
    ids: AppIds,
    parts: MenuParts,
    manifest: ManifestViewModel,
    check: CheckViewModel,
    undo: UndoViewModel,
    /// Read by the window's close guard, which is built before this view-model
    /// exists and cannot reach it. One signal, written here and read there.
    unsaved: Signal<bool>,
    // The screens' view-models are built once and kept, never built inside `build`.
    // A rebuild would otherwise hand each screen a fresh set of signals, and
    // anything held in one, a selection, a pending edit, a bridged combo value,
    // would reset every time anything on the window asked for a rebuild.
    home: HomeViewModel,
    project: ProjectViewModel,
    entities: EntitiesViewModel,
    fields: FieldViewModel,
    user_interface: UserInterfaceViewModel,
    features: FeaturesViewModel,
    use_cases: UseCaseViewModel,
    dto_in: DtoViewModel,
    dto_out: DtoViewModel,
    generate: GenerateViewModel,
    new_manifest: NewManifestViewModel,
    demo: DemoViewModel,
    root_child: Option<WidgetId>,
}

impl App {
    pub fn new(
        session: Session,
        ids: AppIds,
        parts: MenuParts,
        manifest: ManifestViewModel,
        check: CheckViewModel,
        undo: UndoViewModel,
        unsaved: Signal<bool>,
    ) -> Self {
        let project = ProjectViewModel::new(
            session.app_ctx.clone(),
            session.single_global.clone(),
            ids.clone(),
        );
        let entities = EntitiesViewModel::new(
            session.app_ctx.clone(),
            ids.clone(),
            session.workspace_entities.clone(),
            session.single_entity.clone(),
        );
        // The field list's owner is the selected entity, which is the one piece of
        // state the two view-models share. Handed over as a signal rather than as a
        // reference to the other view-model, so neither holds the other.
        let fields = FieldViewModel::new(
            session.app_ctx.clone(),
            ids.clone(),
            session.entity_fields.clone(),
            session.single_field.clone(),
            entities.selected(),
        );
        // The language lives on the Project screen; the targets on offer here follow
        // it, so the signal is shared rather than mirrored.
        let user_interface = UserInterfaceViewModel::new(
            session.app_ctx.clone(),
            session.single_user_interface.clone(),
            ids.clone(),
            project.language(),
        );
        let features = FeaturesViewModel::new(
            session.app_ctx.clone(),
            ids.clone(),
            session.workspace_features.clone(),
            session.single_feature.clone(),
        );
        let use_cases = UseCaseViewModel::new(
            session.app_ctx.clone(),
            ids.clone(),
            session.feature_use_cases.clone(),
            session.single_use_case.clone(),
            features.selected(),
            session.workspace_entities.clone(),
        );
        // Two panes, each with its own handles. The session mints one `SingleDto`,
        // one `DtoFieldsListModel` and one `SingleDtoField` and wires those; a
        // second pane sharing them would show one DTO twice, and a second pane
        // taking fresh ones that nobody wired would be deaf and merely look empty.
        // `DtoViewModel` therefore builds its own and wires them itself.
        let dto_in = DtoViewModel::new(
            session.app_ctx.clone(),
            ids.clone(),
            DtoSide::In,
            use_cases.selected(),
            use_cases.name(),
        );
        let dto_out = DtoViewModel::new(
            session.app_ctx.clone(),
            ids.clone(),
            DtoSide::Out,
            use_cases.selected(),
            use_cases.name(),
        );
        // Text editors retain one undoable edit until Enter/blur. Their buffers
        // must still enable Save immediately, and Ctrl+S must commit them first.
        let pending = [
            session.single_global.dirty(),
            session.single_entity.dirty(),
            session.single_field.dirty(),
            session.single_feature.dirty(),
            session.single_use_case.dirty(),
            session.single_user_interface.dirty(),
            dto_in.pending_dirty(),
            dto_out.pending_dirty(),
        ]
        .into_iter()
        .fold(Signal::new(false), |any, dirty| {
            any.zip(&dirty).map(|(any, dirty)| *any || *dirty)
        });
        let editors = (
            project.clone(),
            entities.clone(),
            fields.clone(),
            features.clone(),
            use_cases.clone(),
            dto_in.clone(),
            dto_out.clone(),
        );
        manifest.track_pending_edits(pending, move || {
            editors.0.commit();
            editors.1.commit();
            editors.2.flush_pending_edits();
            editors.3.commit();
            editors.4.commit();
            editors.5.flush_pending_edits();
            editors.6.flush_pending_edits();
        });
        let app_ctx_for_wizard = session.app_ctx.clone();
        // The demo loads a manifest of its own, so it goes through the same
        // view-model every other open does rather than writing to the store behind
        // the application's back.
        let demo = DemoViewModel::new(session.app_ctx.clone(), manifest.clone());
        let generate = GenerateViewModel::new(
            session.app_ctx.clone(),
            ids.clone(),
            session.system_files.clone(),
        );
        Self {
            session,
            ids,
            parts,
            manifest,
            check,
            undo,
            unsaved,
            home: HomeViewModel::new(),
            project,
            entities,
            fields,
            user_interface,
            features,
            use_cases,
            dto_in,
            dto_out,
            generate,
            new_manifest: NewManifestViewModel::new(app_ctx_for_wizard),
            demo,
            root_child: None,
        }
    }
}

impl std::fmt::Debug for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("App").finish_non_exhaustive()
    }
}

impl App {
    /// Point the lists that belong to the open manifest rather than to a screen.
    ///
    /// `workspace.entities` and `workspace.features` are read by more than one
    /// screen: the Features screen lists entities so a use case can name the ones it
    /// touches, and both DTO combos read them too. A list pointed by whichever
    /// screen happens to be mounted is empty on every other one, which is a bug that
    /// only shows up on the screen that did not do the pointing.
    ///
    /// The lists whose owner is a *selection* stay with the screen that owns the
    /// selection.
    fn point_workspace_lists(&self, ctx: &mut BuildContext) {
        let session = self.session.clone();
        let workspace = self.ids.workspace_id.clone();
        ctx.effect(&workspace, move |id| {
            session.workspace_entities_owner.set(*id);
            session.workspace_features_owner.set(*id);
        });
        self.session.workspace_entities_owner.set(workspace.get());
        self.session.workspace_features_owner.set(workspace.get());
    }
}

impl App {
    /// Tell the Generate screen when it is entered and when it is left.
    ///
    /// Here rather than in the screen's own `build`, because a screen that is not
    /// mounted does not build: the moment worth acting on is exactly the one where
    /// the screen stops existing. Leaving cancels the long operation that renders
    /// every file in the manifest, and releases the rendered bodies, which are the
    /// largest thing this app holds.
    fn follow_generate_screen(&self, ctx: &mut BuildContext) {
        let generate = self.generate.clone();
        let screen = self.ids.screen.clone();
        ctx.effect(&screen, move |screen| {
            if *screen == Screen::Generate {
                generate.enter();
            } else {
                generate.leave();
            }
        });
        if screen.get() == Screen::Generate {
            self.generate.enter();
        }
    }
}

impl Widget for App {
    fn build(&mut self, ctx: &mut BuildContext) -> Vec<WidgetId> {
        // Every generated handle re-subscribes on every build: a `BuildContext`
        // subscription lives exactly one build cycle, so a guard here would leave
        // the whole app deaf after its first rebuild.
        self.session.wire_all(ctx);
        self.point_workspace_lists(ctx);
        // Language is shared with User Interface and must follow the manifest
        // before the lazy Project page has ever been mounted.
        self.project.wire(ctx);
        self.follow_generate_screen(ctx);
        self.manifest.wire(ctx);
        self.check.wire(ctx);
        self.undo.wire(ctx);
        self.demo.wire(ctx);

        // Keep the close guard's answer current.
        let unsaved = self.unsaved.clone();
        let can_save = self.manifest.can_save();
        ctx.effect(&can_save, move |dirty| {
            unsaved.set_if_changed(*dirty);
        });
        self.unsaved.set_if_changed(can_save.get());
        commands::register(
            ctx,
            &CommandDeps {
                ids: self.ids.clone(),
                parts: self.parts.clone(),
                manifest: self.manifest.clone(),
                entities: self.entities.clone(),
                undo: self.undo.clone(),
                new_manifest: self.new_manifest.clone(),
                demo: self.demo.clone(),
            },
        );

        let screen = self.ids.screen.clone();
        let selected = screen.map(|s| s.index());

        let rail = NavRail::new(
            self.ids.clone(),
            self.parts.manifest_open.clone(),
            self.parts.check_critical.clone(),
        );

        let pages = Switcher::new(selected)
            .child(home::page::page(self.home.clone(), self.manifest.clone()))
            .child(ProjectPage::new(self.project.clone()))
            .child(EntitiesPage::new(
                self.entities.clone(),
                self.fields.clone(),
            ))
            .child(FeaturesPage::new(
                self.features.clone(),
                self.use_cases.clone(),
                self.dto_in.clone(),
                self.dto_out.clone(),
            ))
            .child(UserInterfacePage::new(self.user_interface.clone()))
            .child(GeneratePage::new(self.generate.clone()));

        let id = ctx.add(teksu!(
            HStack {
                spacing: 0.0
                child: rail
                Expand {
                    child: pages
                }
            }
        ));
        self.root_child = Some(id);
        vec![id]
    }

    fn layout_response(&self, proposal: SizeProposal, ctx: &LayoutContext) -> LayoutResponse {
        self.root_child
            .and_then(|id| ctx.child_size(id, proposal))
            .map(LayoutResponse::from)
            .unwrap_or_else(|| proposal.resolve(0.0, 0.0).into())
    }
}

#[cfg(all(test, not(feature = "mocks")))]
mod tests {
    use super::*;
    use crate::{event_source::QleanyEventSource, project::Language, test_support::Fixture};
    use frontend::{EventHubClient, handling_manifest::dtos::CreateLanguage};
    use teksilo::core::{
        WidgetTree,
        event_source::{AppEventPoster, EventSourceAdapter, SubscriptionId, TreeAppContext},
    };

    struct NoEvents;
    impl AppEventPoster for NoEvents {
        fn post_subscription_event(&self, _: SubscriptionId, _: Box<dyn std::any::Any + Send>) {}
    }

    #[test]
    fn cpp_targets_are_ready_before_project_is_visited() {
        let f = Fixture::new(CreateLanguage::CppQt);
        let session = Session::new(f.ctx.clone());
        let check = CheckViewModel::new(f.ctx.clone(), f.manifest.is_open());
        let undo = UndoViewModel::new(f.ctx.clone(), f.ids.clone());
        let parts = MenuParts {
            manifest_open: f.manifest.is_open(),
            can_save: f.manifest.can_save(),
            can_undo: undo.can_undo(),
            can_redo: undo.can_redo(),
            undo_label: undo.undo_label(),
            redo_label: undo.redo_label(),
            dark: Signal::new(false),
            check_critical: check.critical(),
        };
        let app = App::new(
            session,
            f.ids.clone(),
            parts,
            f.manifest.clone(),
            check,
            undo,
            Signal::new(false),
        );
        let ui = app.user_interface.clone();
        f.ids.screen.set(Screen::UserInterface);
        let source = QleanyEventSource::new(EventHubClient::new(&f.ctx.event_hub));
        let mut tree = WidgetTree::new();
        tree.set_app_context(std::rc::Rc::new(TreeAppContext::with_source_and_poster(
            EventSourceAdapter::new(source),
            std::sync::Arc::new(NoEvents),
        )));
        tree.add(app);
        tree.layout(SizeProposal::exact(1280.0, 820.0));
        assert_eq!(ui.language().get(), Some(Language::CppQt));
        assert_eq!(
            ui.targets(),
            &[
                crate::user_interface::Target::CppQtWidgets,
                crate::user_interface::Target::CppQtQuick
            ]
        );
    }
}
