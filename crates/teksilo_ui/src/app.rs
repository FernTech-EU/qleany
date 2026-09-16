//! The application body: the navigation rail and the screen it selects.

pub mod commands;
pub mod nav;

use teksilo::prelude::*;
use teksilo::widgets::{Expand, HStack, Switcher, TextWidget};

use crate::app::commands::CommandDeps;
use crate::app::nav::NavRail;

use crate::app_ids::AppIds;
use crate::home::{self, HomeViewModel};
use crate::manifest::ManifestViewModel;
use crate::project::{ProjectPage, ProjectViewModel};
use crate::session::Session;
use crate::shell::menus::MenuParts;

pub struct App {
    session: Session,
    ids: AppIds,
    parts: MenuParts,
    manifest: ManifestViewModel,
    // The screens' view-models are built once and kept, never built inside `build`.
    // A rebuild would otherwise hand each screen a fresh set of signals, and
    // anything held in one, a selection, a pending edit, a bridged combo value,
    // would reset every time anything on the window asked for a rebuild.
    home: HomeViewModel,
    project: ProjectViewModel,
    root_child: Option<WidgetId>,
}

impl App {
    pub fn new(
        session: Session,
        ids: AppIds,
        parts: MenuParts,
        manifest: ManifestViewModel,
    ) -> Self {
        let project = ProjectViewModel::new(session.single_global.clone(), ids.clone());
        Self {
            session,
            ids,
            parts,
            manifest,
            home: HomeViewModel::new(),
            project,
            root_child: None,
        }
    }
}

impl std::fmt::Debug for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("App").finish_non_exhaustive()
    }
}

impl Widget for App {
    fn build(&mut self, ctx: &mut BuildContext) -> Vec<WidgetId> {
        // Every generated handle re-subscribes on every build: a `BuildContext`
        // subscription lives exactly one build cycle, so a guard here would leave
        // the whole app deaf after its first rebuild.
        self.session.wire_all(ctx);
        self.manifest.wire(ctx);
        commands::register(
            ctx,
            &CommandDeps {
                ids: self.ids.clone(),
                parts: self.parts.clone(),
                manifest: self.manifest.clone(),
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
            .child(TextWidget::new(tr!(nav_entities())))
            .child(TextWidget::new(tr!(nav_features())))
            .child(TextWidget::new(tr!(nav_user_interface())))
            .child(TextWidget::new(tr!(nav_generate())));

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
