#![cfg(test)]
#![allow(dead_code)]
#![allow(unused_imports)]

use super::{GenerationOps, GenerationReadOps, GenerationSnapshot, SnapshotBuilder};
use anyhow::Result;
use common::database::QueryUnitOfWork;
use common::direct_access::root::RootRelationshipField;
use common::direct_access::workspace::WorkspaceRelationshipField;
use common::entities::{
    Cardinality, Direction, Dto, DtoField, Entity, Feature, Field, FieldRelationshipType,
    FieldType, File, FileStatus, Global, Relationship, RelationshipType, Root, Strength, System,
    UseCase, UserInterface, Workspace,
};
use common::types::EntityId;
use std::collections::HashMap;

// DummyGenerationReadOps that allows setting return values
struct DummyGenerationReadOps {
    roots: HashMap<EntityId, Root>,
    workspaces: HashMap<EntityId, Workspace>,
    systems: HashMap<EntityId, System>,
    files: HashMap<EntityId, File>,
    globals: HashMap<EntityId, Global>,
    features: HashMap<EntityId, Feature>,
    use_cases: HashMap<EntityId, UseCase>,
    entities: HashMap<EntityId, Entity>,
    dtos: HashMap<EntityId, Dto>,
    dto_fields: HashMap<EntityId, DtoField>,
    fields: HashMap<EntityId, Field>,
    relationships: HashMap<EntityId, Relationship>,
    workspace_entities: HashMap<EntityId, Vec<EntityId>>,
    workspace_features: HashMap<EntityId, Vec<EntityId>>,
    system_files: HashMap<EntityId, Vec<EntityId>>,
    user_interfaces: HashMap<EntityId, UserInterface>,
}

impl DummyGenerationReadOps {
    fn new() -> Self {
        Self {
            roots: HashMap::new(),
            workspaces: HashMap::new(),
            systems: HashMap::new(),
            files: HashMap::new(),
            globals: HashMap::new(),
            features: HashMap::new(),
            use_cases: HashMap::new(),
            entities: HashMap::new(),
            dtos: HashMap::new(),
            dto_fields: HashMap::new(),
            fields: HashMap::new(),
            relationships: HashMap::new(),
            workspace_entities: HashMap::new(),
            workspace_features: HashMap::new(),
            system_files: HashMap::new(),
            user_interfaces: HashMap::new(),
        }
    }
}

// Implement minimal QueryUnitOfWork
impl QueryUnitOfWork for DummyGenerationReadOps {
    fn begin_transaction(&self) -> Result<()> {
        Ok(())
    }
    fn end_transaction(&self) -> Result<()> {
        Ok(())
    }
}

impl GenerationOps for DummyGenerationReadOps {
    fn get_root_relationship(
        &self,
        id: &EntityId,
        field: &RootRelationshipField,
    ) -> Result<Vec<EntityId>> {
        let root = self
            .roots
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("Root not found"))?;
        match field {
            RootRelationshipField::Workspace => {
                Ok(root.workspace.map(|id| vec![id]).unwrap_or_default())
            }
            RootRelationshipField::System => Ok(root.system.map(|id| vec![id]).unwrap_or_default()),
        }
    }

    fn get_all_root(&self) -> Result<Vec<Root>> {
        Ok(self.roots.values().cloned().collect())
    }

    fn get_system(&self, id: &EntityId) -> Result<Option<System>> {
        Ok(self.systems.get(id).cloned())
    }

    fn get_workspace(&self, id: &EntityId) -> Result<Option<Workspace>> {
        Ok(self.workspaces.get(id).cloned())
    }

    fn get_workspace_relationship(
        &self,
        id: &EntityId,
        field: &WorkspaceRelationshipField,
    ) -> Result<Vec<EntityId>> {
        match field {
            WorkspaceRelationshipField::Entities => {
                Ok(self.workspace_entities.get(id).cloned().unwrap_or_default())
            }
            WorkspaceRelationshipField::Features => {
                Ok(self.workspace_features.get(id).cloned().unwrap_or_default())
            }
            WorkspaceRelationshipField::Global => Ok(self
                .workspaces
                .get(id)
                .map(|w| vec![w.global])
                .unwrap_or_default()),
            WorkspaceRelationshipField::UserInterface => Ok(self
                .workspaces
                .get(id)
                .map(|w| vec![w.user_interface])
                .unwrap_or_default()),
        }
    }

    fn get_user_interface(&self, id: &EntityId) -> Result<Option<UserInterface>> {
        Ok(self.user_interfaces.get(id).cloned())
    }

    fn get_file(&self, id: &EntityId) -> Result<Option<File>> {
        Ok(self.files.get(id).cloned())
    }

    fn get_global(&self, id: &EntityId) -> Result<Option<Global>> {
        Ok(self.globals.get(id).cloned())
    }

    fn get_feature(&self, id: &EntityId) -> Result<Option<Feature>> {
        Ok(self.features.get(id).cloned())
    }

    fn get_feature_multi(&self, ids: &[EntityId]) -> Result<Vec<Option<Feature>>> {
        Ok(ids.iter().map(|i| self.features.get(i).cloned()).collect())
    }

    fn get_use_case(&self, id: &EntityId) -> Result<Option<UseCase>> {
        Ok(self.use_cases.get(id).cloned())
    }

    fn get_use_case_multi(&self, ids: &[EntityId]) -> Result<Vec<Option<UseCase>>> {
        Ok(ids.iter().map(|i| self.use_cases.get(i).cloned()).collect())
    }

    fn get_dto(&self, id: &EntityId) -> Result<Option<Dto>> {
        Ok(self.dtos.get(id).cloned())
    }

    fn get_dto_field_multi(&self, ids: &[EntityId]) -> Result<Vec<Option<DtoField>>> {
        Ok(ids
            .iter()
            .map(|i| self.dto_fields.get(i).cloned())
            .collect())
    }

    fn get_entity(&self, id: &EntityId) -> Result<Option<Entity>> {
        Ok(self.entities.get(id).cloned())
    }

    fn get_entity_multi(&self, ids: &[EntityId]) -> Result<Vec<Option<Entity>>> {
        Ok(ids.iter().map(|i| self.entities.get(i).cloned()).collect())
    }

    fn get_field_multi(&self, ids: &[EntityId]) -> Result<Vec<Option<Field>>> {
        Ok(ids.iter().map(|i| self.fields.get(i).cloned()).collect())
    }

    fn get_relationship_multi(&self, ids: &[EntityId]) -> Result<Vec<Option<Relationship>>> {
        Ok(ids
            .iter()
            .map(|i| self.relationships.get(i).cloned())
            .collect())
    }
}

// Implement GenerationReadOps methods (declared via macros on the trait)
impl GenerationReadOps for DummyGenerationReadOps {
    fn get_root_relationship(
        &self,
        _id: &EntityId,
        _field: &common::direct_access::root::RootRelationshipField,
    ) -> Result<Vec<EntityId>> {
        todo!()
    }
    fn get_workspace(&self, _id: &EntityId) -> Result<Option<Workspace>> {
        todo!()
    }
    fn get_system(&self, _id: &EntityId) -> Result<Option<System>> {
        todo!()
    }
    fn get_workspace_relationship(
        &self,
        _id: &EntityId,
        _field: &common::direct_access::workspace::WorkspaceRelationshipField,
    ) -> Result<Vec<EntityId>> {
        todo!()
    }
    fn get_user_interface(&self, _id: &EntityId) -> Result<Option<UserInterface>> {
        todo!()
    }
    fn get_file(&self, _id: &EntityId) -> Result<Option<File>> {
        todo!()
    }
    fn get_global(&self, _id: &EntityId) -> Result<Option<Global>> {
        todo!()
    }
    fn get_feature(&self, _id: &EntityId) -> Result<Option<Feature>> {
        todo!()
    }
    fn get_feature_multi(&self, _ids: &[EntityId]) -> Result<Vec<Option<Feature>>> {
        todo!()
    }
    fn get_use_case(&self, _id: &EntityId) -> Result<Option<UseCase>> {
        todo!()
    }
    fn get_use_case_multi(&self, _ids: &[EntityId]) -> Result<Vec<Option<UseCase>>> {
        todo!()
    }
    fn get_dto(&self, _id: &EntityId) -> Result<Option<Dto>> {
        todo!()
    }
    fn get_dto_field_multi(&self, _ids: &[EntityId]) -> Result<Vec<Option<DtoField>>> {
        todo!()
    }
    fn get_entity(&self, _id: &EntityId) -> Result<Option<Entity>> {
        todo!()
    }
    fn get_entity_multi(&self, _ids: &[EntityId]) -> Result<Vec<Option<Entity>>> {
        todo!()
    }
    fn get_field_multi(&self, _ids: &[EntityId]) -> Result<Vec<Option<Field>>> {
        todo!()
    }
    fn get_relationship_multi(&self, _ids: &[EntityId]) -> Result<Vec<Option<Relationship>>> {
        todo!()
    }
    fn get_all_root(&self) -> Result<Vec<Root>> {
        todo!()
    }
}

#[test]
fn for_file_returns_err_when_file_missing() {
    let uow = DummyGenerationReadOps::new();
    let res = SnapshotBuilder::for_file_id(&uow, 1, &Vec::new());
    assert!(res.is_err());
}

#[test]
fn for_file_feature_without_use_cases_errors() {
    let mut uow = DummyGenerationReadOps::new();
    let file = File {
        id: 1,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "f".into(),
        relative_path: "p".into(),
        group: "g".into(),
        template_name: "feature_lib".into(),
        generated_code: None,
        status: FileStatus::New,
        nature: Default::default(),
        feature: Some(10),
        all_features: false,
        entity: None,
        all_entities: false,
        use_case: None,
        all_use_cases: false,
        field: None,
    };
    uow.files.insert(1, file);
    let feature = Feature {
        id: 10,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "Feat".into(),
        use_cases: vec![],
    };
    uow.features.insert(10, feature);
    let global = Global {
        id: 3,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        application_name: "App".into(),
        language: "rust".into(),
        organisation_name: "Org".into(),
        organisation_domain: "org.com".into(),
        prefix_path: "".into(),
    };
    uow.globals.insert(3, global);
    let user_interface = UserInterface {
        id: 1,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        rust_cli: false,
        rust_slint: false,
        cpp_qt_qtwidgets: false,
        cpp_qt_qtquick: false,
        rust_ios: false,
        rust_android: false,
    };
    uow.user_interfaces.insert(1, user_interface);
    let workspace = Workspace {
        id: 2,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        manifest_absolute_path: "".into(),
        global: 3,
        entities: vec![],
        features: vec![10],
        user_interface: 1,
    };
    uow.workspaces.insert(2, workspace);
    uow.workspace_features.insert(2, vec![10]);
    let root = Root {
        id: 1,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        workspace: Some(2),
        system: None,
    };
    uow.roots.insert(1, root);
    let res = SnapshotBuilder::for_file_id(&uow, 1, &Vec::new());
    assert!(res.is_err());
}

#[test]
fn for_file_happy_path_feature_with_use_case_and_dtos() {
    let mut uow = DummyGenerationReadOps::new();
    // File bound to feature
    let file = File {
        id: 1,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "f".into(),
        relative_path: "p".into(),
        group: "g".into(),
        template_name: "feature_lib".into(),
        generated_code: None,
        status: FileStatus::New,
        nature: Default::default(),
        feature: Some(10),
        all_features: false,
        entity: None,
        all_entities: false,
        use_case: None,
        all_use_cases: false,
        field: None,
    };
    uow.files.insert(1, file);
    // Feature with use case 100
    let uc = UseCase {
        id: 100,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "UC".into(),
        entities: vec![300],
        undoable: false,
        read_only: false,
        long_operation: false,
        dto_in: Some(200),
        dto_out: Some(201),
    };
    let feature = Feature {
        id: 10,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "Feat".into(),
        use_cases: vec![100],
    };
    uow.features.insert(10, feature);
    uow.use_cases.insert(100, uc.clone());
    // Entity and fields
    let ent = Entity {
        id: 300,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "User".into(),
        only_for_heritage: false,
        inherits_from: None,
        single_model: true,

        fields: vec![400],
        relationships: vec![],
        undoable: false,
    };
    let field = Field {
        id: 400,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "name".into(),
        field_type: FieldType::String,
        entity: Some(300),
        relationship: FieldRelationshipType::OneToOne,
        optional: true,
        is_list: false,
        strong: true,
        list_model: false,
        list_model_displayed_field: None,
        enum_name: None,
        enum_values: vec![],
    };
    uow.entities.insert(300, ent);
    uow.fields.insert(400, field);
    // DTOs and fields
    let dto_in = Dto {
        id: 200,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "In".into(),
        fields: vec![500],
    };
    let dto_out = Dto {
        id: 201,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "Out".into(),
        fields: vec![501],
    };
    uow.dtos.insert(200, dto_in);
    uow.dtos.insert(201, dto_out);
    let df_in = DtoField {
        id: 500,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "a".into(),
        field_type: common::entities::DtoFieldType::String,
        optional: false,
        is_list: false,
        enum_name: None,
        enum_values: vec![],
    };
    let df_out = DtoField {
        id: 501,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "b".into(),
        field_type: common::entities::DtoFieldType::Integer,
        optional: true,
        is_list: false,
        enum_name: None,
        enum_values: vec![],
    };
    uow.dto_fields.insert(500, df_in);
    uow.dto_fields.insert(501, df_out);
    let global = Global {
        id: 3,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        application_name: "App".into(),
        language: "rust".into(),
        organisation_name: "Org".into(),
        organisation_domain: "org.com".into(),
        prefix_path: "".into(),
    };
    uow.globals.insert(3, global);
    let user_interface = UserInterface {
        id: 1,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        rust_cli: false,
        rust_slint: false,
        cpp_qt_qtwidgets: false,
        cpp_qt_qtquick: false,
        rust_ios: false,
        rust_android: false,
    };
    uow.user_interfaces.insert(1, user_interface);
    let workspace = Workspace {
        id: 2,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        manifest_absolute_path: "".into(),
        global: 3,
        entities: vec![],
        features: vec![10],
        user_interface: 1,
    };
    uow.workspaces.insert(2, workspace);
    uow.workspace_features.insert(2, vec![10]);
    let system = System {
        id: 4,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        version: "1.0.0".into(),
        files: vec![],
    };
    uow.systems.insert(4, system);
    let root = Root {
        id: 1,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        workspace: Some(2),
        system: Some(4),
    };
    uow.roots.insert(1, root);

    let (snap, _from_cache) = SnapshotBuilder::for_file_id(&uow, 1, &Vec::new()).expect("snapshot");
    assert!(snap.features.contains_key(&10));
    assert!(snap.use_cases.contains_key(&100));
    assert!(snap.entities.contains_key(&300));
    assert!(snap.dtos.contains_key(&200) && snap.dtos.contains_key(&201));
}

#[test]
fn for_file_various_combinations_generate_expected_items() {
    // Prepare uow with feature, use_case, entities, dtos
    let mut uow = DummyGenerationReadOps::new();

    // Common entities
    let ent_a = Entity {
        id: 1,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "A".into(),
        only_for_heritage: false,
        inherits_from: None,
        single_model: true,

        fields: vec![],
        relationships: vec![],
        undoable: false,
    };
    let ent_b = Entity {
        id: 2,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "B".into(),
        only_for_heritage: false,
        inherits_from: None,
        single_model: true,

        fields: vec![],
        relationships: vec![],
        undoable: false,
    };
    uow.entities.insert(1, ent_a.clone());
    uow.entities.insert(2, ent_b.clone());
    // Workspace contains both entities (for all_entities: true)
    uow.workspace_entities.insert(2, vec![1, 2]);

    // DTOs for UC
    let dto_in = Dto {
        id: 10,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "In".into(),
        fields: vec![],
    };
    let dto_out = Dto {
        id: 11,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "Out".into(),
        fields: vec![],
    };
    uow.dtos.insert(10, dto_in);
    uow.dtos.insert(11, dto_out);

    // Use case referencing ent_a and ent_b
    let uc = UseCase {
        id: 100,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "UC".into(),
        entities: vec![1, 2],
        undoable: false,
        read_only: false,
        long_operation: false,
        dto_in: Some(10),
        dto_out: Some(11),
    };
    uow.use_cases.insert(100, uc.clone());

    // Feature with the UC
    let feat = Feature {
        id: 200,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "Feat".into(),
        use_cases: vec![100],
    };
    uow.features.insert(200, feat.clone());
    let global = Global {
        id: 3,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        application_name: "App".into(),
        language: "rust".into(),
        organisation_name: "Org".into(),
        organisation_domain: "org.com".into(),
        prefix_path: "".into(),
    };
    uow.globals.insert(3, global);
    let user_interface = UserInterface {
        id: 1,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        rust_cli: false,
        rust_slint: false,
        cpp_qt_qtwidgets: false,
        cpp_qt_qtquick: false,
        rust_ios: false,
        rust_android: false,
    };
    uow.user_interfaces.insert(1, user_interface);
    let workspace = Workspace {
        id: 2,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        manifest_absolute_path: "".into(),
        global: 3,
        entities: vec![1, 2],
        features: vec![200],
        user_interface: 1,
    };
    uow.workspaces.insert(2, workspace);
    uow.workspace_features.insert(2, vec![200]);
    uow.workspace_entities.insert(2, vec![1, 2]);
    let system = System {
        id: 4,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        version: "1.0.0".into(),
        files: vec![],
    };
    uow.systems.insert(4, system);
    let root = Root {
        id: 1,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        workspace: Some(2),
        system: Some(4),
    };
    uow.roots.insert(1, root);

    // 1) File with only feature
    let file_feature_only = File {
        id: 1000,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "f1".into(),
        relative_path: "p".into(),
        group: "g".into(),
        template_name: "feature_lib".into(),
        generated_code: None,
        status: FileStatus::New,
        nature: Default::default(),
        feature: Some(200),
        all_features: false,
        entity: None,
        all_entities: false,
        use_case: None,
        all_use_cases: false,
        field: None,
    };
    uow.files.insert(1000, file_feature_only);
    let (snap, _from_cache) =
        SnapshotBuilder::for_file_id(&uow, 1000, &Vec::new()).expect("snapshot");
    assert!(snap.features.contains_key(&200));
    assert!(snap.use_cases.contains_key(&100));
    assert!(snap.entities.contains_key(&1) && snap.entities.contains_key(&2));
    assert!(snap.dtos.contains_key(&10) && snap.dtos.contains_key(&11));

    // 2) File with only use_case
    let file_uc_only = File {
        id: 1001,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "f2".into(),
        relative_path: "p".into(),
        group: "g".into(),
        template_name: "feature_use_case".into(),
        generated_code: None,
        status: FileStatus::New,
        nature: Default::default(),
        feature: None,
        all_features: false,
        entity: None,
        all_entities: false,
        use_case: Some(100),
        all_use_cases: false,
        field: None,
    };
    uow.files.insert(1001, file_uc_only);
    let (snap, _from_cache) =
        SnapshotBuilder::for_file_id(&uow, 1001, &Vec::new()).expect("snapshot");
    assert!(snap.features.is_empty());
    assert!(snap.use_cases.contains_key(&100));
    assert!(snap.entities.contains_key(&1) && snap.entities.contains_key(&2));
    assert!(snap.dtos.contains_key(&10) && snap.dtos.contains_key(&11));

    // 3) File with only entity
    let file_ent_only = File {
        id: 1002,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "f3".into(),
        relative_path: "p".into(),
        group: "g".into(),
        template_name: "entity_mod".into(),
        generated_code: None,
        status: FileStatus::New,
        nature: Default::default(),
        feature: None,
        all_features: false,
        entity: Some(1),
        all_entities: false,
        use_case: None,
        all_use_cases: false,
        field: None,
    };
    uow.files.insert(1002, file_ent_only);
    let (snap, _from_cache) =
        SnapshotBuilder::for_file_id(&uow, 1002, &Vec::new()).expect("snapshot");
    assert!(snap.features.is_empty());
    assert!(snap.use_cases.is_empty());
    assert!(snap.entities.contains_key(&1));

    // 4) File with all_entities: true -> loads all entities from root
    let file_all_ent = File {
        id: 1003,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "f4".into(),
        relative_path: "p".into(),
        group: "g".into(),
        template_name: "entity_mod".into(),
        generated_code: None,
        status: FileStatus::New,
        nature: Default::default(),
        feature: None,
        all_features: false,
        entity: None,
        all_entities: true,
        use_case: None,
        all_use_cases: false,
        field: None,
    };
    uow.files.insert(1003, file_all_ent);
    let (snap, _from_cache) =
        SnapshotBuilder::for_file_id(&uow, 1003, &Vec::new()).expect("snapshot");
    assert!(snap.entities.contains_key(&1) && snap.entities.contains_key(&2));

    // 5) File with feature + entity: ensure both feature scope (UCs, dtos, uc entities) and explicit entity are included
    let file_feat_ent = File {
        id: 1004,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "f5".into(),
        relative_path: "p".into(),
        group: "g".into(),
        template_name: "feature_lib".into(),
        generated_code: None,
        status: FileStatus::New,
        nature: Default::default(),
        feature: Some(200),
        all_features: false,
        entity: Some(1),
        all_entities: false,
        use_case: None,
        all_use_cases: false,
        field: None,
    };
    uow.files.insert(1004, file_feat_ent);
    let (snap, _from_cache) =
        SnapshotBuilder::for_file_id(&uow, 1004, &Vec::new()).expect("snapshot");
    assert!(snap.features.contains_key(&200));
    assert!(snap.use_cases.contains_key(&100));
    // must include entity 1 (explicit) and UC entities
    assert!(snap.entities.contains_key(&1) && snap.entities.contains_key(&2));

    // 6) File with use_case + entity
    let file_uc_ent = File {
        id: 1005,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "f6".into(),
        relative_path: "p".into(),
        group: "g".into(),
        template_name: "entity_use_cases_mod".into(),
        generated_code: None,
        status: FileStatus::New,
        nature: Default::default(),
        feature: None,
        all_features: false,
        entity: Some(2),
        all_entities: false,
        use_case: Some(100),
        all_use_cases: false,
        field: None,
    };
    uow.files.insert(1005, file_uc_ent);
    let (snap, _from_cache) =
        SnapshotBuilder::for_file_id(&uow, 1005, &Vec::new()).expect("snapshot");
    assert!(snap.use_cases.contains_key(&100));
    // entities from UC plus explicitly provided entity
    assert!(snap.entities.contains_key(&1) && snap.entities.contains_key(&2));
}

// ── WriteTransactionGuard generation ────────────────────────────────────────
//
// The following tests exercise the real Tera rendering pipeline (not just
// snapshot assembly) for the templates that wire the generated
// `write_guard` module into every write-transaction call site. They assert
// both on the presence of the guard wiring and on the syntactic validity of
// the rendered Rust (via `rustfmt`, which fails to parse malformed code).

/// Pipe `code` through `rustfmt` (stdin -> stdout) purely to validate that it
/// parses as syntactically correct Rust. Panics with rustfmt's stderr if it
/// doesn't.
fn assert_is_valid_rust(code: &str, label: &str) {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new("rustfmt")
        .arg("--edition")
        .arg("2024")
        .arg("--emit")
        .arg("stdout")
        .arg("--quiet")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn rustfmt — required to validate generated template output");

    child
        .stdin
        .take()
        .unwrap()
        .write_all(code.as_bytes())
        .expect("failed to write to rustfmt stdin");

    let output = child.wait_with_output().expect("failed to wait on rustfmt");
    assert!(
        output.status.success(),
        "{label}: rendered template output is not valid Rust:\n{}\n\n--- rendered code ---\n{code}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Collect the brace-matched body of every `fn {name}(...)` in `code` that
/// actually has one. Trait *declarations* (`fn commit(&mut self) -> Result<()>;`)
/// are skipped: their signature is closed by `;` before any `{`, and following
/// the next `{` would otherwise splice in an unrelated function's body.
fn fn_bodies<'a>(code: &'a str, name: &str) -> Vec<&'a str> {
    let sig = format!("fn {name}(");
    let mut bodies = Vec::new();
    let mut from = 0;

    while let Some(rel) = code[from..].find(&sig) {
        let start = from + rel;
        from = start + sig.len();

        let open = match code[start..].find(['{', ';']) {
            Some(off) if code.as_bytes()[start + off] == b'{' => start + off,
            _ => continue, // declaration, not a definition
        };

        let mut depth = 0usize;
        for (i, c) in code[open..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        bodies.push(&code[open..open + i + 1]);
                        break;
                    }
                }
                _ => {}
            }
        }
    }

    bodies
}

/// Assert that every generated `commit`/`rollback` releases the write guard
/// only *after* the transaction itself is finished.
///
/// This is deliberately an ordering assertion, not another `contains` check.
/// The `contains` assertions in the tests below would pass just as happily if
/// `write_guard = None` were moved *above* the `commit()?`/`rollback()?` call
/// — which would hand the store's slot to a second writer while the first
/// transaction's whole-store savepoint is still live, reopening precisely the
/// silent-rollback window `common::database::write_guard` exists to close.
/// Since the guard cannot detect its own misuse from inside the generated
/// app, the ordering has to be pinned here, at the point of generation.
fn assert_guard_outlives_the_transaction(code: &str, release_stmt: &str, label: &str) {
    let mut checked = 0;

    for (fn_name, terminator) in [("commit", ".commit()?"), ("rollback", ".rollback()?")] {
        for body in fn_bodies(code, fn_name) {
            if !body.contains(release_stmt) {
                continue;
            }

            let release_at = body.find(release_stmt).unwrap();
            let terminator_at = body.find(terminator).unwrap_or_else(|| {
                panic!("{label}: `{fn_name}` releases the write guard but never calls `{terminator}`:\n{body}")
            });

            assert!(
                terminator_at < release_at,
                "{label}: `{fn_name}` releases the write guard BEFORE finishing the \
                 transaction. The guard must outlive the transaction's whole-store \
                 savepoint — releasing it first lets a second writer in while that \
                 savepoint is still live, which is the silent-rollback bug the guard \
                 exists to prevent.\n{body}"
            );
            checked += 1;
        }
    }

    assert_eq!(
        checked, 2,
        "{label}: expected the write guard to be released in exactly one `commit` and one \
         `rollback`, found {checked} such fn(s) — the guard wiring moved, and this test no \
         longer checks what it claims to"
    );
}

fn base_global() -> Global {
    Global {
        id: 3,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        application_name: "App".into(),
        language: "rust".into(),
        organisation_name: "Org".into(),
        organisation_domain: "org.com".into(),
        prefix_path: "".into(),
    }
}

fn base_user_interface() -> UserInterface {
    UserInterface {
        id: 1,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        rust_cli: false,
        rust_slint: false,
        cpp_qt_qtwidgets: false,
        cpp_qt_qtquick: false,
        rust_ios: false,
        rust_android: false,
    }
}

#[test]
fn write_guard_template_renders_valid_rust() {
    let mut uow = DummyGenerationReadOps::new();
    let file = File {
        id: 1,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "write_guard.rs".into(),
        relative_path: "common/src/database/".into(),
        group: "base".into(),
        template_name: "write_guard".into(),
        generated_code: None,
        status: FileStatus::New,
        nature: Default::default(),
        feature: None,
        all_features: false,
        entity: None,
        all_entities: false,
        use_case: None,
        all_use_cases: false,
        field: None,
    };
    uow.files.insert(1, file);
    uow.globals.insert(3, base_global());
    uow.user_interfaces.insert(1, base_user_interface());
    let workspace = Workspace {
        id: 2,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        manifest_absolute_path: "".into(),
        global: 3,
        entities: vec![],
        features: vec![],
        user_interface: 1,
    };
    uow.workspaces.insert(2, workspace);
    let system = System {
        id: 4,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        version: "1.0.0".into(),
        files: vec![],
    };
    uow.systems.insert(4, system);
    let root = Root {
        id: 1,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        workspace: Some(2),
        system: Some(4),
    };
    uow.roots.insert(1, root);

    let (snap, _) = SnapshotBuilder::for_file_id(&uow, 1, &Vec::new()).expect("snapshot");
    let code = super::generate_code_with_snapshot(&snap).expect("render write_guard");
    assert!(code.contains("pub struct WriteTransactionGuard"));
    assert!(code.contains("pub fn acquire("));
    assert_is_valid_rust(&code, "write_guard");
}

/// Build a minimal snapshot for a feature use-case UoW file (`feature_use_case_uow`
/// template), toggling `read_only`/`long_operation` to hit every branch.
fn feature_use_case_uow_snapshot(read_only: bool, long_operation: bool) -> GenerationSnapshot {
    let mut uow = DummyGenerationReadOps::new();

    let ent = Entity {
        id: 300,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "Thing".into(),
        only_for_heritage: false,
        inherits_from: None,
        single_model: true,
        fields: vec![],
        relationships: vec![],
        undoable: true,
    };
    uow.entities.insert(300, ent);

    let uc = UseCase {
        id: 100,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "DoThing".into(),
        entities: vec![300],
        undoable: !read_only,
        read_only,
        long_operation,
        dto_in: None,
        dto_out: None,
    };
    uow.use_cases.insert(100, uc);

    let feature = Feature {
        id: 10,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "Doing".into(),
        use_cases: vec![100],
    };
    uow.features.insert(10, feature);

    let file = File {
        id: 1,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "do_thing_uow.rs".into(),
        relative_path: "src/units_of_work/".into(),
        group: "feature".into(),
        template_name: "feature_use_case_uow".into(),
        generated_code: None,
        status: FileStatus::New,
        nature: Default::default(),
        feature: Some(10),
        all_features: false,
        entity: None,
        all_entities: false,
        use_case: Some(100),
        all_use_cases: false,
        field: None,
    };
    uow.files.insert(1, file);

    uow.globals.insert(3, base_global());
    uow.user_interfaces.insert(1, base_user_interface());
    let workspace = Workspace {
        id: 2,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        manifest_absolute_path: "".into(),
        global: 3,
        entities: vec![300],
        features: vec![10],
        user_interface: 1,
    };
    uow.workspaces.insert(2, workspace);
    uow.workspace_features.insert(2, vec![10]);
    uow.workspace_entities.insert(2, vec![300]);
    let system = System {
        id: 4,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        version: "1.0.0".into(),
        files: vec![],
    };
    uow.systems.insert(4, system);
    let root = Root {
        id: 1,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        workspace: Some(2),
        system: Some(4),
    };
    uow.roots.insert(1, root);

    SnapshotBuilder::for_file_id(&uow, 1, &Vec::new())
        .expect("snapshot")
        .0
}

#[test]
fn feature_use_case_uow_write_template_wires_the_guard_and_renders_valid_rust() {
    let snap = feature_use_case_uow_snapshot(false, false);
    let code = super::generate_code_with_snapshot(&snap).expect("render feature_use_case_uow");
    assert!(code.contains("write_guard: Option<WriteTransactionGuard>"));
    assert!(code.contains(r#"WriteTransactionGuard::acquire(&self.context, "do_thing")"#));
    assert!(code.contains("self.write_guard = None;"));
    assert_guard_outlives_the_transaction(
        &code,
        "self.write_guard = None;",
        "feature_use_case_uow (write, non-long-operation)",
    );
    assert_is_valid_rust(&code, "feature_use_case_uow (write, non-long-operation)");
}

#[test]
fn feature_use_case_uow_read_only_template_has_no_guard_and_renders_valid_rust() {
    let snap = feature_use_case_uow_snapshot(true, false);
    let code = super::generate_code_with_snapshot(&snap).expect("render feature_use_case_uow");
    assert!(!code.contains("WriteTransactionGuard"));
    assert_is_valid_rust(
        &code,
        "feature_use_case_uow (read-only, non-long-operation)",
    );
}

#[test]
fn feature_use_case_uow_long_operation_write_template_wires_the_guard_and_renders_valid_rust() {
    let snap = feature_use_case_uow_snapshot(false, true);
    let code = super::generate_code_with_snapshot(&snap).expect("render feature_use_case_uow");
    assert!(code.contains("write_guard: Mutex<Option<WriteTransactionGuard>>"));
    assert!(code.contains(r#"WriteTransactionGuard::acquire(&self.context, "do_thing")"#));
    // The thread-safe arm releases the guard through `lock_or_recover`, not a
    // plain `.lock().unwrap()`: a panic inside a long operation is caught and
    // reported as `Failed`, and a poisoned guard mutex would otherwise turn that
    // reported failure into a second, fatal panic on the next use case to run.
    assert!(code.contains("*common::long_operation::lock_or_recover(&self.write_guard) = None;"));
    assert_guard_outlives_the_transaction(
        &code,
        "*common::long_operation::lock_or_recover(&self.write_guard) = None;",
        "feature_use_case_uow (write, long-operation)",
    );
    assert_is_valid_rust(&code, "feature_use_case_uow (write, long-operation)");
}

#[test]
fn feature_use_case_uow_long_operation_read_only_template_has_no_guard_and_renders_valid_rust() {
    let snap = feature_use_case_uow_snapshot(true, true);
    let code = super::generate_code_with_snapshot(&snap).expect("render feature_use_case_uow");
    assert!(!code.contains("WriteTransactionGuard"));
    assert_is_valid_rust(&code, "feature_use_case_uow (read-only, long-operation)");
}

/// Build a minimal snapshot for an entity's `entity_units_of_work` file, with
/// an owner (exercises `OwnedWriteUoW`) and a forward relationship (exercises
/// `WriteRelUoW`) so the write-guard wiring is checked alongside the richest
/// generated shape.
fn entity_units_of_work_snapshot() -> GenerationSnapshot {
    let mut uow = DummyGenerationReadOps::new();

    let owner = Entity {
        id: 1,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "Folder".into(),
        only_for_heritage: false,
        inherits_from: None,
        single_model: false,
        fields: vec![],
        relationships: vec![900],
        undoable: true,
    };
    let child = Entity {
        id: 2,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "Thing".into(),
        only_for_heritage: false,
        inherits_from: None,
        single_model: false,
        fields: vec![],
        relationships: vec![900, 901],
        undoable: true,
    };
    let tag = Entity {
        id: 3,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "Tag".into(),
        only_for_heritage: false,
        inherits_from: None,
        single_model: false,
        fields: vec![],
        relationships: vec![],
        undoable: true,
    };
    uow.entities.insert(1, owner);
    uow.entities.insert(2, child);
    uow.entities.insert(3, tag);

    // Owner relationship: Folder strongly owns Thing.
    let rel_owner = Relationship {
        id: 900,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        left_entity: Some(1),
        right_entity: Some(2),
        field_name: "Things".into(),
        relationship_type: RelationshipType::OneToMany,
        strength: Strength::Strong,
        direction: Direction::Forward,
        cardinality: Cardinality::ZeroOrMore,
        order: None,
    };
    uow.relationships.insert(900, rel_owner);

    // Thing's own forward relationship (weak, many-to-many to Tag) so
    // `forward_relationships` is non-empty for Thing itself, exercising
    // `WriteRelUoW`.
    let rel_forward = Relationship {
        id: 901,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        left_entity: Some(2),
        right_entity: Some(3),
        field_name: "Tags".into(),
        relationship_type: RelationshipType::ManyToMany,
        strength: Strength::Weak,
        direction: Direction::Forward,
        cardinality: Cardinality::ZeroOrMore,
        order: None,
    };
    uow.relationships.insert(901, rel_forward);

    let file = File {
        id: 1,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "units_of_work.rs".into(),
        relative_path: "src/thing/".into(),
        group: "entities".into(),
        template_name: "entity_units_of_work".into(),
        generated_code: None,
        status: FileStatus::New,
        nature: Default::default(),
        feature: None,
        all_features: false,
        entity: Some(2),
        all_entities: false,
        use_case: None,
        all_use_cases: false,
        field: None,
    };
    uow.files.insert(1, file);

    uow.globals.insert(3, base_global());
    uow.user_interfaces.insert(1, base_user_interface());
    let workspace = Workspace {
        id: 2,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        manifest_absolute_path: "".into(),
        global: 3,
        entities: vec![1, 2],
        features: vec![],
        user_interface: 1,
    };
    uow.workspaces.insert(2, workspace);
    uow.workspace_entities.insert(2, vec![1, 2]);
    let system = System {
        id: 4,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        version: "1.0.0".into(),
        files: vec![],
    };
    uow.systems.insert(4, system);
    let root = Root {
        id: 1,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        workspace: Some(2),
        system: Some(4),
    };
    uow.roots.insert(1, root);

    SnapshotBuilder::for_file_id(&uow, 1, &Vec::new())
        .expect("snapshot")
        .0
}

#[test]
fn entity_units_of_work_template_wires_the_guard_and_renders_valid_rust() {
    let snap = entity_units_of_work_snapshot();
    let code = super::generate_code_with_snapshot(&snap).expect("render entity_units_of_work");
    assert!(code.contains("write_guard: Option<WriteTransactionGuard>"));
    assert!(
        code.contains(r#"WriteTransactionGuard::acquire(&self.context, "thing_direct_write")"#)
    );
    assert!(code.contains("self.write_guard = None;"));
    // Sanity: the richer trait impls (owned + relationship) are still present
    // alongside the guard wiring.
    assert!(code.contains("impl use_cases::OwnedWriteUoW for ThingWriteUoW"));
    assert!(code.contains("impl use_cases::WriteRelUoW<ThingRelationshipField> for ThingWriteUoW"));
    assert_guard_outlives_the_transaction(
        &code,
        "self.write_guard = None;",
        "entity_units_of_work",
    );
    assert_is_valid_rust(&code, "entity_units_of_work");
}
