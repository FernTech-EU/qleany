use crate::use_cases::common::tools;
use crate::{FillRustFilesDto, FillRustFilesReturnDto};
use anyhow::Result;
use common::direct_access::feature::FeatureRelationshipField;
use common::direct_access::system::SystemRelationshipField;
use common::direct_access::workspace::WorkspaceRelationshipField;
use common::entities::UserInterface;
use common::entities::{Entity, FileNature};
use common::generator::file_list_builder::FileListBuilder;
use common::types::EntityId;
use common::{
    database::CommandUnitOfWork, entities::Feature, entities::File, entities::Global,
    entities::Relationship, entities::Root, entities::UseCase,
};

pub trait FillRustFilesUnitOfWorkFactoryTrait {
    fn create(&self) -> Box<dyn FillRustFilesUnitOfWorkTrait>;
}

#[macros::uow_action(entity = "Root", action = "GetAll")]
#[macros::uow_action(entity = "Root", action = "GetRelationship")]
#[macros::uow_action(entity = "Workspace", action = "GetRelationship")]
#[macros::uow_action(entity = "System", action = "GetRelationship")]
#[macros::uow_action(entity = "System", action = "SetRelationship")]
#[macros::uow_action(entity = "Global", action = "Get")]
#[macros::uow_action(entity = "UserInterface", action = "Get")]
#[macros::uow_action(entity = "Entity", action = "GetMulti")]
#[macros::uow_action(entity = "Entity", action = "GetRelationship")]
#[macros::uow_action(entity = "Relationship", action = "GetMulti")]
#[macros::uow_action(entity = "Feature", action = "GetMulti")]
#[macros::uow_action(entity = "Feature", action = "GetRelationship")]
#[macros::uow_action(entity = "UseCase", action = "GetMulti")]
#[macros::uow_action(entity = "File", action = "CreateOrphan")]
#[macros::uow_action(entity = "File", action = "CreateOrphanMulti")]
#[macros::uow_action(entity = "File", action = "RemoveMulti")]
pub trait FillRustFilesUnitOfWorkTrait: CommandUnitOfWork {}

pub struct FillRustFilesUseCase {
    uow_factory: Box<dyn FillRustFilesUnitOfWorkFactoryTrait>,
}

impl FillRustFilesUseCase {
    pub fn new(uow_factory: Box<dyn FillRustFilesUnitOfWorkFactoryTrait>) -> Self {
        FillRustFilesUseCase { uow_factory }
    }

    pub fn execute(&mut self, dto: &FillRustFilesDto) -> Result<FillRustFilesReturnDto> {
        let mut b = FileListBuilder::new();

        let mut uow = self.uow_factory.create();
        uow.begin_transaction()?;

        use anyhow::anyhow;
        let roots = uow.get_all_root()?;
        let root = roots
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("Root entity not found"))?;

        let all_workspace_ids = uow.get_root_relationship(
            &root.id,
            &common::direct_access::root::RootRelationshipField::Workspace,
        )?;

        let workspace_id = all_workspace_ids
            .first()
            .cloned()
            .ok_or(anyhow!("No workspace found"))?;

        let all_system_ids = uow.get_root_relationship(
            &root.id,
            &common::direct_access::root::RootRelationshipField::System,
        )?;

        let system_id = all_system_ids
            .first()
            .cloned()
            .ok_or(anyhow!("No system found"))?;

        // Get global
        let globals =
            uow.get_workspace_relationship(&workspace_id, &WorkspaceRelationshipField::Global)?;
        let global_id = globals.first().ok_or(anyhow!("No global found"))?;
        let global = uow.get_global(global_id)?;
        let global = global.ok_or(anyhow!("Global not found"))?;
        if global.language != "rust" {
            return Err(anyhow!("Global language is not rust"));
        };

        // get prefix path
        let prefix = global.prefix_path.clone();
        // strip it from leading and trailing "/" or "\"
        let prefix = if prefix.trim().is_empty() {
            "crates".to_string()
        } else {
            tools::strip_leading_and_trailing_slashes(&prefix)
        };

        // ui
        let user_interfaces = uow.get_workspace_relationship(
            &workspace_id,
            &WorkspaceRelationshipField::UserInterface,
        )?;
        let ui_id = user_interfaces
            .first()
            .ok_or(anyhow!("No user interface found"))?;
        let ui = uow
            .get_user_interface(ui_id)?
            .ok_or(anyhow!("User interface not found"))?;

        // remove all files from system
        let all_previous_files =
            uow.get_system_relationship(&root.id, &SystemRelationshipField::Files)?;
        if !all_previous_files.is_empty() {
            uow.remove_file_multi(&all_previous_files)?;
        }

        b.add(
            "Cargo.toml",
            "",
            "base",
            "root_cargo",
            FileNature::Aggregate,
        )
        .all_features = true;

        b.add(
            "Cargo.toml",
            format!("{}/common/", prefix),
            "base",
            "common_cargo",
            FileNature::Infrastructure,
        );

        b.add(
            "lib.rs",
            format!("{}/common/src/", prefix),
            "base",
            "common_lib",
            FileNature::Aggregate,
        );

        b.add(
            "undo_redo.rs",
            format!("{}/common/src/", prefix),
            "base",
            "undo_redo",
            FileNature::Infrastructure,
        );

        b.add(
            "long_operation.rs",
            format!("{}/common/src/", prefix),
            "base",
            "long_operation",
            FileNature::Infrastructure,
        );

        b.add(
            "error.rs",
            format!("{}/common/src/", prefix),
            "base",
            "error",
            FileNature::Infrastructure,
        );

        b.add(
            "repository_factory.rs",
            format!("{}/common/src/direct_access/", prefix),
            "base",
            "repository_factory",
            FileNature::Aggregate,
        )
        .all_entities = true;

        b.add(
            "setup.rs",
            format!("{}/common/src/direct_access/", prefix),
            "base",
            "common_setup",
            FileNature::Aggregate,
        )
        .all_entities = true;

        b.add(
            "types.rs",
            format!("{}/common/src/", prefix),
            "base",
            "types",
            FileNature::Infrastructure,
        );

        b.add(
            "database.rs",
            format!("{}/common/src/", prefix),
            "base",
            "database",
            FileNature::Infrastructure,
        );

        b.add(
            "db_context.rs",
            format!("{}/common/src/database/", prefix),
            "base",
            "db_context",
            FileNature::Infrastructure,
        );

        b.add(
            "db_helpers.rs",
            format!("{}/common/src/database/", prefix),
            "base",
            "db_helpers",
            FileNature::Infrastructure,
        );

        b.add(
            "hashmap_store.rs",
            format!("{}/common/src/database/", prefix),
            "base",
            "hashmap_store",
            FileNature::Infrastructure,
        )
        .all_entities = true;

        b.add(
            "transactions.rs",
            format!("{}/common/src/database/", prefix),
            "base",
            "transactions",
            FileNature::Infrastructure,
        );

        b.add(
            "write_guard.rs",
            format!("{}/common/src/database/", prefix),
            "base",
            "write_guard",
            FileNature::Infrastructure,
        );

        b.add(
            "snapshot.rs",
            format!("{}/common/src/", prefix),
            "base",
            "snapshot",
            FileNature::Infrastructure,
        );

        b.add(
            "transaction_tests.rs",
            format!("{}/common/tests/", prefix),
            "base",
            "transaction_tests",
            FileNature::Infrastructure,
        );

        b.add(
            "undo_redo_tests.rs",
            format!("{}/common/tests/", prefix),
            "base",
            "undo_redo_tests",
            FileNature::Infrastructure,
        );

        // direct access entities

        b.add(
            "entities.rs",
            format!("{}/common/src/", prefix),
            "entities",
            "common_entities",
            FileNature::Aggregate,
        )
        .all_entities = true;

        {
            let f = b.add(
                "event.rs",
                format!("{}/common/src/", prefix),
                "base",
                "common_event",
                FileNature::Aggregate,
            );
            f.all_features = true;
            f.all_entities = true;
        }

        b.add(
            "direct_access.rs",
            format!("{}/common/src/", prefix),
            "entities",
            "common_direct_access_mod",
            FileNature::Aggregate,
        )
        .all_entities = true;

        b.add(
            "Cargo.toml",
            format!("{}/direct_access/", prefix),
            "entities",
            "direct_access_cargo",
            FileNature::Infrastructure,
        );

        b.add(
            "lib.rs",
            format!("{}/direct_access/src/", prefix),
            "entities",
            "direct_access_lib",
            FileNature::Aggregate,
        )
        .all_entities = true;

        b.add(
            "use_cases.rs",
            format!("{}/common/src/direct_access/", prefix),
            "entities",
            "common_da_use_cases_mod",
            FileNature::Aggregate,
        );

        let uc_relative_path = format!("{}/common/src/direct_access/use_cases/", prefix);

        b.add_batch(
            &uc_relative_path,
            "entities",
            FileNature::Infrastructure,
            &[
                ("traits.rs", "common_da_use_cases_traits"),
                ("get.rs", "common_da_use_cases_get"),
                ("create_orphan.rs", "common_da_use_cases_create_orphan"),
                ("create.rs", "common_da_use_cases_create"),
                ("update.rs", "common_da_use_cases_update"),
                ("remove.rs", "common_da_use_cases_remove"),
                (
                    "get_relationship.rs",
                    "common_da_use_cases_get_relationship",
                ),
                (
                    "get_relationship_many.rs",
                    "common_da_use_cases_get_relationship_many",
                ),
                (
                    "get_relationship_count.rs",
                    "common_da_use_cases_get_relationship_count",
                ),
                (
                    "get_relationship_in_range.rs",
                    "common_da_use_cases_get_relationship_in_range",
                ),
                (
                    "set_relationship.rs",
                    "common_da_use_cases_set_relationship",
                ),
                (
                    "move_relationship.rs",
                    "common_da_use_cases_move_relationship",
                ),
            ],
        );

        // Get entities
        let entities =
            uow.get_workspace_relationship(&workspace_id, &WorkspaceRelationshipField::Entities)?;
        let entities = uow.get_entity_multi(&entities)?;

        for entity in &entities {
            let entity = entity.as_ref().ok_or(anyhow!("Entity not found"))?;

            // continue if entity is "heritage"
            if entity.only_for_heritage {
                continue;
            }

            // for crates/direct_access/src/

            b.add(
                format!("{}.rs", heck::AsSnakeCase(&entity.name)),
                format!("{}/direct_access/src/", prefix),
                "entities",
                "entity_mod",
                FileNature::Aggregate,
            )
            .entity = Some(entity.id);

            let relative_path = format!(
                "{}/direct_access/src/{}/",
                prefix,
                heck::AsSnakeCase(&entity.name)
            );

            b.add(
                "dtos.rs",
                relative_path.clone(),
                "entities",
                "entity_dtos",
                FileNature::Infrastructure,
            )
            .entity = Some(entity.id);

            b.add(
                "units_of_work.rs",
                relative_path.clone(),
                "entities",
                "entity_units_of_work",
                FileNature::Infrastructure,
            )
            .entity = Some(entity.id);

            b.add(
                format!("{}_controller.rs", heck::AsSnakeCase(&entity.name)),
                relative_path.clone(),
                "entities",
                "entity_controller",
                FileNature::Infrastructure,
            )
            .entity = Some(entity.id);

            // for crates/common/src/direct_access/
            let relative_path = format!("{}/common/src/direct_access/", prefix);

            b.add(
                format!("{}.rs", heck::AsSnakeCase(&entity.name)),
                relative_path.to_string(),
                "entities",
                "common_entity_mod",
                FileNature::Aggregate,
            )
            .entity = Some(entity.id);

            b.add(
                format!("{}_repository.rs", heck::AsSnakeCase(&entity.name)),
                format!("{}{}/", relative_path, heck::AsSnakeCase(&entity.name)),
                "entities",
                "common_entity_repository",
                FileNature::Infrastructure,
            )
            .entity = Some(entity.id);

            b.add(
                format!("{}_table.rs", heck::AsSnakeCase(&entity.name)),
                format!("{}{}/", relative_path, heck::AsSnakeCase(&entity.name)),
                "entities",
                "common_entity_table",
                FileNature::Infrastructure,
            )
            .entity = Some(entity.id);
        }

        // features:

        let features =
            uow.get_workspace_relationship(&workspace_id, &WorkspaceRelationshipField::Features)?;

        let features = uow.get_feature_multi(&features)?;

        for feature in &features {
            let feature = feature.as_ref().ok_or(anyhow!("Feature not found"))?;

            let relative_path = format!("{}/{}/", prefix, heck::AsSnakeCase(&feature.name));

            b.add(
                "Cargo.toml",
                relative_path.clone(),
                "features",
                "feature_cargo",
                FileNature::Infrastructure,
            )
            .feature = Some(feature.id);

            let relative_path = format!("{}/{}/src/", prefix, heck::AsSnakeCase(&feature.name));

            b.add(
                "lib.rs",
                relative_path.clone(),
                "features",
                "feature_lib",
                FileNature::Aggregate,
            )
            .feature = Some(feature.id);

            b.add(
                "use_cases.rs",
                relative_path.clone(),
                "features",
                "feature_use_cases_mod",
                FileNature::Aggregate,
            )
            .feature = Some(feature.id);

            b.add(
                "dtos.rs",
                relative_path.clone(),
                "features",
                "feature_dtos",
                FileNature::Infrastructure,
            )
            .feature = Some(feature.id);

            b.add(
                "units_of_work.rs",
                relative_path.clone(),
                "features",
                "feature_units_of_work_mod",
                FileNature::Aggregate,
            )
            .feature = Some(feature.id);

            b.add(
                format!("{}_controller.rs", heck::AsSnakeCase(&feature.name)),
                relative_path.clone(),
                "features",
                "feature_controller",
                FileNature::Infrastructure,
            )
            .feature = Some(feature.id);

            // for crates/{}/src/use_cases/
            let relative_path = format!(
                "{}/{}/src/use_cases/",
                prefix,
                heck::AsSnakeCase(&feature.name)
            );

            let use_cases =
                uow.get_feature_relationship(&feature.id, &FeatureRelationshipField::UseCases)?;
            let use_cases = uow.get_use_case_multi(&use_cases)?;

            for use_case in &use_cases {
                let use_case = use_case.clone().ok_or(anyhow!("Use case not found"))?;

                let f = b.add(
                    format!("{}_uc.rs", heck::AsSnakeCase(&use_case.name)),
                    relative_path.clone(),
                    "features",
                    "feature_use_case",
                    FileNature::Scaffold,
                );
                f.feature = Some(feature.id);
                f.use_case = Some(use_case.id);
            }

            // for crates/{}/src/units_of_work/
            let relative_path = format!(
                "{}/{}/src/units_of_work/",
                prefix,
                heck::AsSnakeCase(&feature.name)
            );

            for use_case in use_cases {
                let use_case = use_case.ok_or(anyhow!("Use case not found"))?;

                let f = b.add(
                    format!("{}_uow.rs", heck::AsSnakeCase(&use_case.name)),
                    relative_path.clone(),
                    "features",
                    "feature_use_case_uow",
                    FileNature::Scaffold,
                );
                f.feature = Some(feature.id);
                f.use_case = Some(use_case.id);
            }
        }

        // macros in crates/macros/

        b.add(
            "Cargo.toml",
            format!("{}/macros/", prefix),
            "base",
            "macros_cargo",
            FileNature::Infrastructure,
        );

        b.add(
            "lib.rs",
            format!("{}/macros/src/", prefix),
            "base",
            "macros_lib",
            FileNature::Infrastructure,
        );

        b.add(
            "direct_access.rs",
            format!("{}/macros/src/", prefix),
            "base",
            "macros_direct_access",
            FileNature::Infrastructure,
        );

        // Frontend crate (always generated)
        {
            let relative_path = format!("{}/frontend/", prefix);

            b.add(
                "Cargo.toml",
                relative_path.clone(),
                "frontend",
                "frontend_cargo",
                FileNature::Aggregate,
            )
            .all_features = true;

            let relative_path_src = format!("{}/frontend/src/", prefix);

            {
                let f = b.add(
                    "lib.rs",
                    relative_path_src.clone(),
                    "frontend",
                    "frontend_lib",
                    FileNature::Aggregate,
                );
                f.all_features = true;
                f.all_entities = true;
            }

            b.add(
                "app_context.rs",
                relative_path_src.clone(),
                "frontend",
                "frontend_app_context",
                FileNature::Infrastructure,
            );

            b.add(
                "event_hub_client.rs",
                relative_path_src.clone(),
                "frontend",
                "frontend_event_hub_client",
                FileNature::Infrastructure,
            );

            {
                let f = b.add(
                    "flat_event.rs",
                    relative_path_src.clone(),
                    "frontend",
                    "frontend_flat_event",
                    FileNature::Infrastructure,
                );
                f.all_features = true;
                f.all_entities = true;
            }

            {
                let f = b.add(
                    "commands.rs",
                    relative_path_src.clone(),
                    "frontend",
                    "frontend_commands_mod",
                    FileNature::Aggregate,
                );
                f.all_features = true;
                f.all_entities = true;
            }

            // commands:
            let relative_path_commands = format!("{}/frontend/src/commands/", prefix);

            {
                let f = b.add(
                    "undo_redo_commands.rs",
                    relative_path_commands.clone(),
                    "frontend",
                    "frontend_undo_redo_commands",
                    FileNature::Infrastructure,
                );
                f.all_features = true;
                f.all_entities = true;
            }

            {
                let f = b.add(
                    "long_operation_commands.rs",
                    relative_path_commands.clone(),
                    "frontend",
                    "frontend_long_operation_commands",
                    FileNature::Infrastructure,
                );
                f.all_features = true;
                f.all_entities = true;
            }

            for entity in &entities {
                let entity = entity.as_ref().ok_or(anyhow!("Entity not found"))?;
                if entity.only_for_heritage {
                    continue;
                }

                b.add(
                    format!("{}_commands.rs", heck::AsSnakeCase(&entity.name)),
                    relative_path_commands.clone(),
                    "frontend",
                    "frontend_entity_commands",
                    FileNature::Infrastructure,
                )
                .entity = Some(entity.id);
            }

            for feature in &features {
                let feature = feature.as_ref().ok_or(anyhow!("Feature not found"))?;

                b.add(
                    format!("{}_commands.rs", heck::AsSnakeCase(&feature.name)),
                    relative_path_commands.clone(),
                    "frontend",
                    "frontend_feature_commands",
                    FileNature::Infrastructure,
                )
                .feature = Some(feature.id);
            }
        }

        if ui.rust_cli {
            b.add(
                "Cargo.toml",
                format!("{}/cli/", prefix),
                "cli",
                "cli_cargo",
                FileNature::Scaffold,
            );

            b.add(
                "main.rs",
                format!("{}/cli/src/", prefix),
                "cli",
                "cli_main",
                FileNature::Scaffold,
            );
        }

        if ui.rust_slint {
            b.add(
                "Cargo.toml",
                format!("{}/slint_ui/", prefix),
                "slint",
                "slint_cargo",
                FileNature::Scaffold,
            )
            .all_features = true;

            b.add(
                "build.rs",
                format!("{}/slint_ui/", prefix),
                "slint",
                "slint_build",
                FileNature::Scaffold,
            );

            let relative_path = format!("{}/slint_ui/src/", prefix);

            b.add(
                "main.rs",
                relative_path.clone(),
                "slint",
                "slint_main",
                FileNature::Scaffold,
            );

            b.add(
                "app.slint",
                format!("{}/slint_ui/ui/", prefix),
                "slint",
                "slint_app",
                FileNature::Scaffold,
            );

            b.add(
                "globals.slint",
                format!("{}/slint_ui/ui/", prefix),
                "slint",
                "slint_globals",
                FileNature::Scaffold,
            );
        }

        // Mobile bridge crate (generated when rust_ios or rust_android is enabled)
        if ui.rust_ios || ui.rust_android {
            let relative_path = format!("{}/mobile_bridge/", prefix);

            {
                let f = b.add(
                    "Cargo.toml",
                    relative_path.clone(),
                    "mobile_bridge",
                    "mobile_bridge_cargo",
                    FileNature::Aggregate,
                );
                f.all_features = true;
            }

            b.add(
                "uniffi.toml",
                relative_path.clone(),
                "mobile_bridge",
                "mobile_bridge_uniffi_toml",
                FileNature::Infrastructure,
            );

            b.add(
                "uniffi-bindgen.rs",
                relative_path.clone(),
                "mobile_bridge",
                "mobile_bridge_uniffi_bindgen",
                FileNature::Infrastructure,
            );

            let relative_path_src = format!("{}/mobile_bridge/src/", prefix);

            {
                let f = b.add(
                    "lib.rs",
                    relative_path_src.clone(),
                    "mobile_bridge",
                    "mobile_bridge_lib",
                    FileNature::Aggregate,
                );
                f.all_features = true;
                f.all_entities = true;
            }

            b.add(
                "backend.rs",
                relative_path_src.clone(),
                "mobile_bridge",
                "mobile_bridge_backend",
                FileNature::Infrastructure,
            );

            b.add(
                "custom_types.rs",
                relative_path_src.clone(),
                "mobile_bridge",
                "mobile_bridge_custom_types",
                FileNature::Infrastructure,
            );

            b.add(
                "errors.rs",
                relative_path_src.clone(),
                "mobile_bridge",
                "mobile_bridge_errors",
                FileNature::Infrastructure,
            );

            {
                let f = b.add(
                    "events.rs",
                    relative_path_src.clone(),
                    "mobile_bridge",
                    "mobile_bridge_events",
                    FileNature::Infrastructure,
                );
                f.all_features = true;
                f.all_entities = true;
            }

            b.add(
                "undo_redo_commands.rs",
                relative_path_src.clone(),
                "mobile_bridge",
                "mobile_bridge_undo_redo_commands",
                FileNature::Infrastructure,
            );

            // Per-entity command modules
            for entity in &entities {
                let entity = entity.as_ref().ok_or(anyhow!("Entity not found"))?;
                if entity.only_for_heritage {
                    continue;
                }

                b.add(
                    format!("{}_commands.rs", heck::AsSnakeCase(&entity.name)),
                    relative_path_src.clone(),
                    "mobile_bridge",
                    "mobile_bridge_entity_commands",
                    FileNature::Infrastructure,
                )
                .entity = Some(entity.id);
            }

            // Per-feature command modules
            for feature in &features {
                let feature = feature.as_ref().ok_or(anyhow!("Feature not found"))?;

                b.add(
                    format!("{}_commands.rs", heck::AsSnakeCase(&feature.name)),
                    relative_path_src.clone(),
                    "mobile_bridge",
                    "mobile_bridge_feature_commands",
                    FileNature::Infrastructure,
                )
                .feature = Some(feature.id);
            }

            // Integration tests
            {
                let f = b.add(
                    "integration_tests.rs",
                    format!("{}/mobile_bridge/tests/", prefix),
                    "mobile_bridge",
                    "mobile_bridge_tests",
                    FileNature::Infrastructure,
                );
                f.all_entities = true;
            }

            // Platform-native async wrappers

            // Swift async extension
            {
                let f = b.add(
                    "MobileBackend+Async.swift",
                    format!("{}/mobile_bridge/swift/", prefix),
                    "mobile_bridge",
                    "mobile_bridge_swift_async",
                    FileNature::Infrastructure,
                );
                f.all_entities = true;
                f.all_features = true;
            }

            // Swift tests
            {
                let f = b.add(
                    "MobileBridgeTests.swift",
                    format!("{}/mobile_bridge/swift/", prefix),
                    "mobile_bridge",
                    "mobile_bridge_swift_tests",
                    FileNature::Infrastructure,
                );
                f.all_entities = true;
            }

            // Kotlin async extensions
            {
                let f = b.add(
                    "MobileBackendAsync.kt",
                    format!("{}/mobile_bridge/kotlin/", prefix),
                    "mobile_bridge",
                    "mobile_bridge_kotlin_async",
                    FileNature::Infrastructure,
                );
                f.all_entities = true;
                f.all_features = true;
            }

            // Kotlin tests
            {
                let f = b.add(
                    "MobileBridgeTest.kt",
                    format!("{}/mobile_bridge/kotlin/", prefix),
                    "mobile_bridge",
                    "mobile_bridge_kotlin_tests",
                    FileNature::Infrastructure,
                );
                f.all_entities = true;
            }

            // Platform READMEs
            if ui.rust_ios {
                let f = b.add(
                    "README-iOS.md",
                    relative_path.clone(),
                    "mobile_bridge",
                    "mobile_bridge_readme_ios",
                    FileNature::Scaffold,
                );
                f.all_entities = true;
            }

            if ui.rust_android {
                let f = b.add(
                    "README-Android.md",
                    relative_path.clone(),
                    "mobile_bridge",
                    "mobile_bridge_readme_android",
                    FileNature::Scaffold,
                );
                f.all_entities = true;
            }

            // SwiftUI demo app (iOS only)
            if ui.rust_ios {
                let swift_demo_path = format!("{}/mobile_bridge/swift/demo/", prefix);
                for (name, template) in [
                    ("DemoApp.swift", "mobile_bridge_swiftui_app"),
                    (
                        "BackendWrapper.swift",
                        "mobile_bridge_swiftui_backend_wrapper",
                    ),
                    ("ContentView.swift", "mobile_bridge_swiftui_content_view"),
                    ("EntityRowView.swift", "mobile_bridge_swiftui_entity_row"),
                ] {
                    let f = b.add(
                        name,
                        swift_demo_path.clone(),
                        "mobile_bridge",
                        template,
                        FileNature::Scaffold,
                    );
                    f.all_entities = true;
                }
            }

            // Jetpack Compose demo app (Android only)
            if ui.rust_android {
                let kotlin_demo_path = format!("{}/mobile_bridge/kotlin/demo/", prefix);
                for (name, template) in [
                    ("DemoActivity.kt", "mobile_bridge_compose_activity"),
                    ("DemoViewModel.kt", "mobile_bridge_compose_viewmodel"),
                    ("MainScreen.kt", "mobile_bridge_compose_main_screen"),
                    ("EntityCard.kt", "mobile_bridge_compose_entity_card"),
                ] {
                    let f = b.add(
                        name,
                        kotlin_demo_path.clone(),
                        "mobile_bridge",
                        template,
                        FileNature::Scaffold,
                    );
                    f.all_entities = true;
                }
            }
        }

        let files = if dto.only_list_already_existing {
            b.build_filtered(|file| {
                let full_path = format!("{}{}", file.relative_path, file.name);
                std::path::Path::new(&full_path).exists()
            })
        } else {
            b.build()
        };

        // create files in db
        let created_files = uow.create_orphan_file_multi(&files)?;
        uow.set_system_relationship(
            &system_id,
            &SystemRelationshipField::Files,
            &created_files
                .iter()
                .map(|f| f.id)
                .collect::<Vec<EntityId>>(),
        )?;

        uow.commit()?;

        let mut file_ids = vec![];
        let mut file_names = vec![];
        let mut file_groups = vec![];

        for file in created_files {
            file_ids.push(file.id);
            file_names.push(format!("{}{}", file.relative_path, file.name));
            file_groups.push(file.group.clone());
        }

        Ok(FillRustFilesReturnDto {
            file_ids,
            file_names,
            file_groups,
        })
    }
}
