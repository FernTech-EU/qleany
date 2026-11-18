#![cfg(test)]
#![allow(dead_code)]
#![allow(unused_imports)]

use super::{
    DtoVM, EntityVM, FeatureVM, FieldVM, FileVM, GenerationSnapshot, GlobalVM, SystemVM, UseCaseVM,
    UserInterfaceVM, get_rust_tera,
};
use common::entities::{
    Entity, Field, FieldType, File, FileStatus, Global, Relationship, System, UserInterface,
};
use indexmap::IndexMap;
use tera::Context;

#[test]
fn render_direct_access_lib_lists_entities() {
    // Build a snapshot with all_entities: true and two entities in the map
    let file = File {
        id: 1,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "lib".into(),
        relative_path: "crates/direct_access/src/lib.rs".into(),
        group: "entities".into(),
        template_name: "direct_access_lib".into(),
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

    let global = Global {
        id: 50,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        language: "".to_string(),
        application_name: "".to_string(),
        organisation_name: "".to_string(),
        organisation_domain: "".to_string(),
        prefix_path: "".to_string(),
    };

    let e1 = Entity {
        id: 10,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "Feature".into(),
        only_for_heritage: false,
        inherits_from: None,
        single_model: true,

        fields: vec![],
        relationships: vec![],
        undoable: false,
    };
    let e2 = Entity {
        id: 11,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        name: "Field".into(),
        only_for_heritage: false,
        single_model: true,
        inherits_from: None,

        fields: vec![],
        relationships: vec![],
        undoable: false,
    };

    let mut entities = IndexMap::new();
    entities.insert(
        10,
        EntityVM {
            inner: e1,
            relationships: IndexMap::new(),
            forward_relationships: IndexMap::new(),
            backward_relationships: IndexMap::new(),
            snake_name: "feature".into(),
            pascal_name: "Feature".into(),
            fields: vec![],
            normal_fields: vec![],
            owner: None,
            owner_pascal_name: None,
            owner_snake_name: None,
            owner_relationship_field_pascal_name: None,
            owner_relationship_field_snake_name: None,
            owner_relationship_type: None,
            ownership_chain: vec![],
        },
    );
    entities.insert(
        11,
        EntityVM {
            inner: e2,
            relationships: IndexMap::new(),
            forward_relationships: IndexMap::new(),
            backward_relationships: IndexMap::new(),
            snake_name: "field".into(),
            pascal_name: "Field".into(),
            fields: vec![],
            normal_fields: vec![],
            owner: None,
            owner_pascal_name: None,
            owner_snake_name: None,
            owner_relationship_field_pascal_name: None,
            owner_relationship_field_snake_name: None,
            owner_relationship_type: None,
            ownership_chain: vec![],
        },
    );

    let snapshot = GenerationSnapshot {
        file: FileVM { inner: file },
        global: GlobalVM {
            inner: global,
            application_kebab_name: "".to_string(),
            application_snake_name: "".to_string(),
            prefix: "".to_string(),
        },
        ui: UserInterfaceVM {
            inner: UserInterface::default(),
        },
        system: SystemVM {
            inner: System::default(),
        },
        entities,
        features: IndexMap::new(),
        use_cases: IndexMap::new(),
        dtos: IndexMap::new(),
        entity_enum_owners: IndexMap::new(),
    };

    let tera = get_rust_tera();
    let mut context = Context::new();
    context.insert("s", &snapshot);
    let code = tera.render("direct_access_lib", &context).unwrap();

    assert!(code.contains("pub mod feature;"));
    assert!(code.contains("pub mod field;"));
    assert!(code.contains("pub use feature::feature_controller;"));
    assert!(code.contains("pub use field::field_controller;"));
}
