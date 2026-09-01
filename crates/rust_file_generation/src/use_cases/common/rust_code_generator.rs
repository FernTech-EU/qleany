mod direct_access_lib_tests;
mod rust_code_generator_tests;

use crate::use_cases::common::tools;
use anyhow::{Result, anyhow, bail};
use common::database::{CommandUnitOfWork, QueryUnitOfWork};
use common::entities::{
    Dto, DtoField, DtoFieldType, Entity, Feature, Field, FieldRelationshipType, FieldType, File,
    Global, Relationship, RelationshipType, Root, Strength, System, UseCase, UserInterface,
    Workspace,
};
use common::enum_variant_parser;
use common::types::EntityId;
use include_dir::{Dir, include_dir};
use indexmap::IndexMap;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::OnceLock;
use tera::{Context, Tera};

// Shared read-API for snapshot building across code and files generation
#[macros::uow_action(entity = "Root", action = "GetRelationshipRO")]
#[macros::uow_action(entity = "Root", action = "GetAllRO")]
#[macros::uow_action(entity = "System", action = "GetRO")]
#[macros::uow_action(entity = "Workspace", action = "GetRO")]
#[macros::uow_action(entity = "Workspace", action = "GetRelationshipRO")]
#[macros::uow_action(entity = "UserInterface", action = "GetRO")]
#[macros::uow_action(entity = "File", action = "GetRO")]
#[macros::uow_action(entity = "Global", action = "GetRO")]
#[macros::uow_action(entity = "Feature", action = "GetRO")]
#[macros::uow_action(entity = "Feature", action = "GetMultiRO")]
#[macros::uow_action(entity = "UseCase", action = "GetRO")]
#[macros::uow_action(entity = "UseCase", action = "GetMultiRO")]
#[macros::uow_action(entity = "Dto", action = "GetRO")]
#[macros::uow_action(entity = "DtoField", action = "GetMultiRO")]
#[macros::uow_action(entity = "Entity", action = "GetRO")]
#[macros::uow_action(entity = "Entity", action = "GetMultiRO")]
#[macros::uow_action(entity = "Field", action = "GetMultiRO")]
#[macros::uow_action(entity = "Relationship", action = "GetMultiRO")]
pub(crate) trait GenerationReadOps: QueryUnitOfWork + GenerationOps {}

#[macros::uow_action(entity = "Root", action = "GetRelationship")]
#[macros::uow_action(entity = "Root", action = "GetAll")]
#[macros::uow_action(entity = "System", action = "Get")]
#[macros::uow_action(entity = "Workspace", action = "Get")]
#[macros::uow_action(entity = "Workspace", action = "GetRelationship")]
#[macros::uow_action(entity = "UserInterface", action = "Get")]
#[macros::uow_action(entity = "File", action = "Get")]
#[macros::uow_action(entity = "Global", action = "Get")]
#[macros::uow_action(entity = "Feature", action = "Get")]
#[macros::uow_action(entity = "Feature", action = "GetMulti")]
#[macros::uow_action(entity = "UseCase", action = "Get")]
#[macros::uow_action(entity = "UseCase", action = "GetMulti")]
#[macros::uow_action(entity = "Dto", action = "Get")]
#[macros::uow_action(entity = "DtoField", action = "GetMulti")]
#[macros::uow_action(entity = "Entity", action = "Get")]
#[macros::uow_action(entity = "Entity", action = "GetMulti")]
#[macros::uow_action(entity = "Field", action = "GetMulti")]
#[macros::uow_action(entity = "Relationship", action = "GetMulti")]
#[allow(dead_code)]
pub(crate) trait GenerationWriteOps: CommandUnitOfWork + GenerationOps {}

#[macros::uow_action(entity = "Root", action = "GetRelationship")]
#[macros::uow_action(entity = "Root", action = "GetAll")]
#[macros::uow_action(entity = "System", action = "Get")]
#[macros::uow_action(entity = "Workspace", action = "Get")]
#[macros::uow_action(entity = "Workspace", action = "GetRelationship")]
#[macros::uow_action(entity = "UserInterface", action = "Get")]
#[macros::uow_action(entity = "File", action = "Get")]
#[macros::uow_action(entity = "Global", action = "Get")]
#[macros::uow_action(entity = "Feature", action = "Get")]
#[macros::uow_action(entity = "Feature", action = "GetMulti")]
#[macros::uow_action(entity = "UseCase", action = "Get")]
#[macros::uow_action(entity = "UseCase", action = "GetMulti")]
#[macros::uow_action(entity = "Dto", action = "Get")]
#[macros::uow_action(entity = "DtoField", action = "GetMulti")]
#[macros::uow_action(entity = "Entity", action = "Get")]
#[macros::uow_action(entity = "Entity", action = "GetMulti")]
#[macros::uow_action(entity = "Field", action = "GetMulti")]
#[macros::uow_action(entity = "Relationship", action = "GetMulti")]
pub(crate) trait GenerationOps {}

#[derive(Debug, Serialize, Clone)]
pub(crate) struct GenerationSnapshot {
    file: FileVM,
    global: GlobalVM,
    ui: UserInterfaceVM,
    system: SystemVM,
    entities: IndexMap<EntityId, EntityVM>,
    features: IndexMap<EntityId, FeatureVM>,
    use_cases: IndexMap<EntityId, UseCaseVM>,
    dtos: IndexMap<EntityId, DtoVM>,
}

#[derive(Debug, Serialize, Clone)]
struct FileVM {
    pub inner: File,
}

#[derive(Debug, Serialize, Clone)]
struct SystemVM {
    inner: System,
}

#[derive(Debug, Serialize, Clone)]
struct GlobalVM {
    pub inner: Global,
    pub application_kebab_name: String,
    pub application_snake_name: String,
    pub prefix: String,
}

#[derive(Debug, Serialize, Clone)]
struct UserInterfaceVM {
    pub inner: UserInterface,
}

/// An ancestor in an entity's ownership chain, used by the entity controller
/// test template to generate setup code that creates the full owner hierarchy
/// (e.g. Root → Workspace → Entity) before testing an owned entity.
/// Stored Root-first, excluding the entity itself.
#[derive(Debug, Serialize, Clone)]
struct OwnerChainEntry {
    pub snake_name: String,
    pub pascal_name: String,
    pub undoable: bool,
}

#[derive(Debug, Serialize, Clone)]
struct EntityVM {
    pub inner: Entity,
    pub relationships: IndexMap<EntityId, RelationshipVM>,
    pub forward_relationships: IndexMap<EntityId, RelationshipVM>,
    pub backward_relationships: IndexMap<EntityId, RelationshipVM>,
    pub snake_name: String,
    pub pascal_name: String,
    pub fields: Vec<FieldVM>,
    pub normal_fields: Vec<FieldVM>,
    pub owner: Option<EntityId>,
    pub owner_pascal_name: Option<String>,
    pub owner_snake_name: Option<String>,
    pub owner_relationship_field_pascal_name: Option<String>,
    pub owner_relationship_field_snake_name: Option<String>,
    pub owner_relationship_type: Option<String>,
    pub ownership_chain: Vec<OwnerChainEntry>,
}

#[derive(Debug, Serialize, Clone)]
struct FeatureVM {
    pub inner: Feature,
    pub use_cases: IndexMap<EntityId, UseCaseVM>,
    pub snake_name: String,
    pub pascal_name: String,
}

#[derive(Debug, Serialize, Clone)]
struct UseCaseVM {
    pub inner: UseCase,
    pub entities: IndexMap<EntityId, EntityVM>,
    pub dto_in: Option<DtoVM>,
    pub dto_out: Option<DtoVM>,
    pub snake_name: String,
    pub pascal_name: String,
}

#[derive(Debug, Serialize, Clone)]
struct DtoVM {
    pub inner: Dto,
    pub fields: Vec<DtoFieldVM>,
    pub pascal_name: String,
}

#[derive(Debug, Serialize, Clone)]
struct ParsedVariantVM {
    pub name: String,
    pub is_simple: bool,
    pub is_tuple: bool,
    pub is_struct: bool,
    /// Full Rust variant definition line, e.g. `"Text(String)"` or `"Image { name: String, width: i64 }"`
    pub rust_line: String,
    /// Full mobile variant definition line (with mobile types)
    pub mobile_line: String,
    /// Destructuring match pattern, e.g. `"Text(v0)"` or `"Image { name, width }"`
    pub match_pattern: String,
    /// Construction expression for Mobile-to-Core From impl body
    pub mobile_to_core_construct: String,
    /// Construction expression for Core-to-Mobile From impl body
    pub core_to_mobile_construct: String,
}

#[derive(Debug, Serialize, Clone)]
struct FieldVM {
    pub inner: Field,
    pub pascal_name: String,
    pub snake_name: String,
    pub relationship: String,
    pub optional: bool,
    pub is_list: bool,
    pub rust_base_type: String,
    pub rust_type: String,
    /// Pre-resolved Rust enum variant lines (only populated when field_type == Enum)
    pub rust_enum_variants: Vec<String>,
    /// Structured parsed variants for templates that need more control (mobile bridge)
    pub parsed_variants: Vec<ParsedVariantVM>,
    /// Whether any complex variant uses uuid::Uuid
    pub enum_needs_uuid: bool,
    /// Whether any complex variant uses chrono::DateTime
    pub enum_needs_chrono: bool,
    /// Whether any complex variant uses EntityId
    pub enum_needs_entity_id: bool,
    /// Whether any complex variant uses f32/f64
    pub enum_needs_float: bool,
}

#[derive(Debug, Serialize, Clone)]
struct DtoFieldVM {
    pub inner: DtoField,
    pub pascal_name: String,
    pub snake_name: String,
    pub rust_base_type: String,
    pub rust_type: String,
    /// Pre-resolved Rust enum variant lines (only populated when field_type == Enum)
    pub rust_enum_variants: Vec<String>,
    /// Structured parsed variants for templates that need more control
    pub parsed_variants: Vec<ParsedVariantVM>,
    /// Whether any complex variant uses uuid::Uuid
    pub enum_needs_uuid: bool,
    /// Whether any complex variant uses chrono::DateTime
    pub enum_needs_chrono: bool,
    /// Whether any complex variant uses EntityId
    pub enum_needs_entity_id: bool,
    /// Whether any complex variant uses f32/f64
    pub enum_needs_float: bool,
    /// External enum types referenced in complex variants (e.g. TaskMetadata in ProjectSummary)
    pub external_enum_references: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
struct RelationshipVM {
    pub inner: Relationship,
    pub field_snake_name: String,
    pub field_pascal_name: String,
}

static RUST_TEMPLATES_DIR: Dir<'_> =
    include_dir!("$CARGO_MANIFEST_DIR/src/use_cases/common/templates");

static RUST_TERA: OnceLock<Tera> = OnceLock::new();

/// Tera filter that converts a snake_case string to lowerCamelCase.
/// Example: "import_data" → "importData"
fn camel_case_filter(
    value: &tera::Value,
    _args: &std::collections::HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    let s = value
        .as_str()
        .ok_or_else(|| tera::Error::msg("camelCase filter expects a string"))?;
    let camel = heck::AsLowerCamelCase(s).to_string();
    Ok(tera::Value::String(camel))
}

fn get_rust_tera() -> &'static Tera {
    RUST_TERA.get_or_init(|| {
        let mut tera = Tera::default();
        load_templates_from_dir(&mut tera, &RUST_TEMPLATES_DIR);
        tera.register_filter("camelCase", camel_case_filter);
        tera
    })
}

fn load_templates_from_dir(tera: &mut Tera, dir: &Dir) {
    fn add_raw_template_in_dir(tera: &mut Tera, dir: &Dir) {
        for file in dir.files() {
            if let Some(Some(file_stem)) = file.path().file_stem().map(|s| s.to_str()) {
                // remove last ".*" from the file stem
                let file_stem = file_stem
                    .rsplit_once('.')
                    .map_or(file_stem, |(stem, _)| stem);

                // Add the template to Tera
                // The file contents are expected to be UTF-8 encoded
                let content = file.contents_utf8().expect("Invalid UTF-8 in template");
                tera.add_raw_template(file_stem, content)
                    .expect("Failed to add template");
            }
        }
        tera.build_inheritance_chains()
            .expect("Failed to build inheritance");
    }

    add_raw_template_in_dir(tera, dir);
    for subdir in dir.dirs() {
        add_raw_template_in_dir(tera, subdir);
    }
}

pub(crate) fn generate_code_with_snapshot(snapshot: &GenerationSnapshot) -> Result<String> {
    let tera = get_rust_tera();
    let mut context = Context::new();
    context.insert("s", snapshot);

    let template_name = snapshot.file.inner.template_name.as_str();

    let code = if tera.get_template(template_name).is_ok() {
        tera.render(template_name, &context)?
    } else {
        return Err(anyhow::anyhow!(
            "Unknown template name: {}",
            snapshot.file.inner.template_name
        ));
    };

    Ok(code)
}

/// Build parsed variant VMs from raw enum_values strings.
fn build_parsed_variants(
    enum_values: &[String],
) -> (
    Vec<String>,
    Vec<ParsedVariantVM>,
    bool,
    bool,
    bool,
    bool,
    Vec<String>,
) {
    let mut rust_lines = Vec::new();
    let mut parsed = Vec::new();
    let mut needs_uuid = false;
    let mut needs_chrono = false;
    let mut needs_entity_id = false;
    let mut needs_float = false;
    let mut external_refs: Vec<String> = Vec::new();

    for val in enum_values {
        match enum_variant_parser::parse_enum_variant(val) {
            Ok(variant) => {
                let rust_line = enum_variant_parser::variant_to_rust_line(&variant);
                let mobile_line = enum_variant_parser::variant_to_mobile_line(&variant);
                let match_pattern = enum_variant_parser::variant_match_pattern(&variant);
                let m2c = enum_variant_parser::variant_mobile_to_core_construct(&variant);
                let c2m = enum_variant_parser::variant_core_to_mobile_construct(&variant);
                let is_simple =
                    matches!(variant.kind, enum_variant_parser::EnumVariantKind::Simple);
                let is_tuple =
                    matches!(variant.kind, enum_variant_parser::EnumVariantKind::Tuple(_));
                let is_struct = matches!(
                    variant.kind,
                    enum_variant_parser::EnumVariantKind::Struct(_)
                );

                if enum_variant_parser::variant_needs_uuid(&variant) {
                    needs_uuid = true;
                }
                if enum_variant_parser::variant_needs_chrono(&variant) {
                    needs_chrono = true;
                }
                if enum_variant_parser::variant_needs_entity_id(&variant) {
                    needs_entity_id = true;
                }
                if enum_variant_parser::variant_needs_float(&variant) {
                    needs_float = true;
                }

                // Collect external enum references from complex variants
                for r in enum_variant_parser::collect_references(&variant) {
                    if !external_refs.contains(&r) {
                        external_refs.push(r);
                    }
                }

                rust_lines.push(rust_line.clone());
                parsed.push(ParsedVariantVM {
                    name: variant.name,
                    is_simple,
                    is_tuple,
                    is_struct,
                    rust_line,
                    mobile_line,
                    match_pattern,
                    mobile_to_core_construct: m2c,
                    core_to_mobile_construct: c2m,
                });
            }
            Err(_) => {
                // Fallback: treat as simple variant (validation in check_uc will catch errors)
                rust_lines.push(val.clone());
                parsed.push(ParsedVariantVM {
                    name: val.clone(),
                    is_simple: true,
                    is_tuple: false,
                    is_struct: false,
                    rust_line: val.clone(),
                    mobile_line: val.clone(),
                    match_pattern: val.clone(),
                    mobile_to_core_construct: val.clone(),
                    core_to_mobile_construct: val.clone(),
                });
            }
        }
    }

    (
        rust_lines,
        parsed,
        needs_uuid,
        needs_chrono,
        needs_entity_id,
        needs_float,
        external_refs,
    )
}

// Snapshot builder to compose consistent data for templates
pub(crate) struct SnapshotBuilder;

impl SnapshotBuilder {
    fn get_entity_vm(uow: &dyn GenerationOps, entity_id: &EntityId) -> anyhow::Result<EntityVM> {
        use anyhow::anyhow;

        // Load the entity
        let entity = uow
            .get_entity(entity_id)?
            .ok_or_else(|| anyhow!("Entity not found"))?;

        let mut fields_vec: Vec<Field> = Vec::new();

        // Load fields from the inherited entity first, if any
        if let Some(parent_entity_id) = entity.inherits_from {
            let parent_entity_vm = Self::get_entity_vm(uow, &parent_entity_id)?;
            for parent_field_vm in &parent_entity_vm.fields {
                fields_vec.push(parent_field_vm.inner.clone());
            }
        }

        // Load fields belonging to the entity
        if !entity.fields.is_empty() {
            fields_vec.extend(uow.get_field_multi(&entity.fields)?.into_iter().flatten());
        }

        // Build FieldVMs with the same Rust type mapping as used elsewhere
        let mut fields_vm_vec: Vec<FieldVM> = Vec::new();
        for f in &fields_vec {
            if f.name == "id" {
                continue; // skip id field
            }

            let rust_base_type = match f.field_type {
                FieldType::Boolean => "bool".to_string(),
                FieldType::Integer => "i64".to_string(),
                FieldType::UInteger => "u64".to_string(),
                FieldType::Float => "f64".to_string(),
                FieldType::String => "String".to_string(),
                FieldType::Uuid => "uuid::Uuid".to_string(),
                FieldType::DateTime => "chrono::DateTime<chrono::Utc>".to_string(),
                FieldType::Entity => "EntityId".to_string(),
                FieldType::Enum => f
                    .enum_name
                    .clone()
                    .unwrap_or("enum_name not set".to_string()),
            };
            //
            let relationship = match f.relationship {
                FieldRelationshipType::OneToOne => "OneToOne".to_string(),
                FieldRelationshipType::OrderedOneToMany => "OrderedOneToMany".to_string(),
                FieldRelationshipType::OneToMany => "OneToMany".to_string(),
                FieldRelationshipType::ManyToOne => "ManyToOne".to_string(),
                FieldRelationshipType::ManyToMany => "ManyToMany".to_string(),
            };

            let is_relationship_list = matches!(
                f.relationship,
                FieldRelationshipType::OneToMany
                    | FieldRelationshipType::OrderedOneToMany
                    | FieldRelationshipType::ManyToMany
            );
            let is_list = f.is_list || is_relationship_list;

            let rust_type = if is_list {
                format!("Vec<{}>", rust_base_type)
            } else if f.optional {
                format!("Option<{}>", rust_base_type)
            } else {
                rust_base_type.clone()
            };

            // Build parsed variant data for enum fields
            let (
                rust_enum_variants,
                parsed_variants,
                enum_needs_uuid,
                enum_needs_chrono,
                enum_needs_entity_id,
                enum_needs_float,
                _external_refs,
            ) = if f.field_type == FieldType::Enum {
                build_parsed_variants(&f.enum_values)
            } else {
                (vec![], vec![], false, false, false, false, vec![])
            };

            fields_vm_vec.push(FieldVM {
                inner: f.clone(),
                pascal_name: heck::AsPascalCase(&f.name).to_string(),
                snake_name: heck::AsSnakeCase(&f.name).to_string(),
                relationship,
                optional: f.optional,
                is_list,
                rust_base_type,
                rust_type,
                rust_enum_variants,
                parsed_variants,
                enum_needs_uuid,
                enum_needs_chrono,
                enum_needs_entity_id,
                enum_needs_float,
            });
        }

        // Build relationships maps.
        // We want to include both forward and backward relationships where this entity is involved.
        use std::collections::HashSet;
        let mut relationships_map: IndexMap<EntityId, Relationship> = IndexMap::new();

        // Start with relationships explicitly listed on this entity
        let rel_ids_direct = entity.relationships.clone();
        for rel in uow
            .get_relationship_multi(&rel_ids_direct)?
            .into_iter()
            .flatten()
        {
            relationships_map.insert(rel.id, rel);
        }

        // Additionally, scan all entities from workspace to discover relationships that reference this
        // entity as the right side (backward relationships) but may not be listed on this entity.
        // This mirrors broader snapshot building behavior where backward rels are gathered from peers.
        let all_entity_ids = uow.get_workspace_relationship(
            &tools::get_workspace_id(uow)?,
            &common::direct_access::workspace::WorkspaceRelationshipField::Entities,
        )?;

        if !all_entity_ids.is_empty() {
            let all_entities = uow.get_entity_multi(&all_entity_ids)?;
            let mut extra_rel_ids: Vec<EntityId> = Vec::new();
            for e_opt in all_entities.into_iter().flatten() {
                for rid in e_opt.relationships {
                    if !extra_rel_ids.contains(&rid) {
                        extra_rel_ids.push(rid);
                    }
                }
            }
            extra_rel_ids.sort();
            if !extra_rel_ids.is_empty() {
                let extra_rels = uow.get_relationship_multi(&extra_rel_ids)?;
                for rel_opt in extra_rels.into_iter().flatten() {
                    let rel_left_entity_id = match rel_opt.left_entity {
                        Some(id) => id,
                        None => bail!("Relationship {} is missing left_entity", rel_opt.id),
                    };
                    let rel_right_entity_id = match rel_opt.right_entity {
                        Some(id) => id,
                        None => bail!("Relationship {} is missing right_entity", rel_opt.id),
                    };
                    if rel_left_entity_id == entity.id || rel_right_entity_id == entity.id {
                        relationships_map.entry(rel_opt.id).or_insert(rel_opt);
                    }
                }
            }
        }

        // Sort relationships by (field_name, left_entity_name) for deterministic output
        // (IDs may vary between runs, and multiple relationships can share the same field_name)
        let mut entity_names_cache: std::collections::HashMap<EntityId, String> =
            std::collections::HashMap::new();
        for rel in relationships_map.values() {
            for eid in [rel.left_entity, rel.right_entity] {
                let eid = match eid {
                    Some(id) => id,
                    None => bail!("Relationship is missing an entity reference"),
                };
                entity_names_cache.entry(eid).or_insert_with(|| {
                    uow.get_entity(&eid)
                        .ok()
                        .flatten()
                        .map(|e| e.name.clone())
                        .unwrap_or_default()
                });
            }
        }
        relationships_map.sort_by(|_ka, va, _kb, vb| {
            let a_left = entity_names_cache
                .get(&va.left_entity.expect("Relationship missing left_entity"))
                .map(|s| s.as_str())
                .unwrap_or("");
            let b_left = entity_names_cache
                .get(&vb.left_entity.expect("Relationship missing left_entity"))
                .map(|s| s.as_str())
                .unwrap_or("");
            (&va.field_name, a_left).cmp(&(&vb.field_name, b_left))
        });

        // Now split into all/forward/backward with deduplication strategy used in for_file
        let mut rel_all: IndexMap<EntityId, RelationshipVM> = IndexMap::new();
        let mut rel_fwd: IndexMap<EntityId, RelationshipVM> = IndexMap::new();
        let mut rel_bwd: IndexMap<EntityId, RelationshipVM> = IndexMap::new();
        let mut fwd_seen: HashSet<String> = HashSet::new();
        let mut bwd_seen: HashSet<(EntityId, String)> = HashSet::new();

        for (rid, rel) in &relationships_map {
            let left_entity_id = match rel.left_entity {
                Some(id) => id,
                None => bail!("Relationship {} is missing left_entity", rid),
            };
            let right_entity_id = match rel.right_entity {
                Some(id) => id,
                None => bail!("Relationship {} is missing right_entity", rid),
            };

            if left_entity_id == entity.id || right_entity_id == entity.id {
                rel_all.entry(*rid).or_insert_with(|| RelationshipVM {
                    inner: rel.clone(),
                    field_snake_name: heck::AsSnakeCase(rel.field_name.clone()).to_string(),
                    field_pascal_name: heck::AsPascalCase(rel.field_name.clone()).to_string(),
                });
                if left_entity_id == entity.id && fwd_seen.insert(rel.field_name.clone()) {
                    rel_fwd.entry(*rid).or_insert_with(|| RelationshipVM {
                        inner: rel.clone(),
                        field_snake_name: heck::AsSnakeCase(rel.field_name.clone()).to_string(),
                        field_pascal_name: heck::AsPascalCase(rel.field_name.clone()).to_string(),
                    });
                }
                if rel.right_entity == Some(entity.id) {
                    let key = (left_entity_id, rel.field_name.clone());
                    if bwd_seen.insert(key) {
                        rel_bwd.entry(*rid).or_insert_with(|| RelationshipVM {
                            inner: rel.clone(),
                            field_snake_name: heck::AsSnakeCase(rel.field_name.clone()).to_string(),
                            field_pascal_name: heck::AsPascalCase(rel.field_name.clone())
                                .to_string(),
                        });
                    }
                }
            }
        }

        let owner = Self::get_entity_owner(uow, entity_id);
        let owner_entity: Option<Entity> =
            owner.and_then(|owner_id| uow.get_entity(&owner_id).ok().flatten());

        // Build ownership chain: walk from owner up to root
        let mut ownership_chain = Vec::new();
        {
            let mut cursor = owner;
            while let Some(oid) = cursor {
                if let Ok(Some(ancestor)) = uow.get_entity(&oid) {
                    ownership_chain.push(OwnerChainEntry {
                        snake_name: heck::AsSnakeCase(&ancestor.name).to_string(),
                        pascal_name: heck::AsPascalCase(&ancestor.name).to_string(),
                        undoable: ancestor.undoable,
                    });
                    cursor = Self::get_entity_owner(uow, &oid);
                } else {
                    break;
                }
            }
            ownership_chain.reverse(); // Root first
        }

        Ok(EntityVM {
            inner: entity.clone(),
            normal_fields: fields_vm_vec
                .iter()
                .filter(|f| f.inner.field_type != FieldType::Entity)
                .cloned()
                .collect(),
            fields: fields_vm_vec,
            relationships: rel_all,
            forward_relationships: rel_fwd,
            backward_relationships: rel_bwd,
            snake_name: heck::AsSnakeCase(&entity.name).to_string(),
            pascal_name: heck::AsPascalCase(&entity.name).to_string(),
            owner,
            owner_pascal_name: owner_entity
                .as_ref()
                .map(|e| heck::AsPascalCase(&e.name).to_string()),
            owner_snake_name: owner_entity
                .as_ref()
                .map(|e| heck::AsSnakeCase(&e.name).to_string()),
            owner_relationship_field_pascal_name:
                Self::get_entity_owner_relationship_field_pascal_name(uow, entity_id),
            owner_relationship_field_snake_name:
                Self::get_entity_owner_relationship_field_snake_name(uow, entity_id),
            owner_relationship_type: Self::get_entity_owner_relationship_type(uow, entity_id)
                .map(|rt| format!("{:?}", rt)),
            ownership_chain,
        })
    }

    fn get_dto_field_rust_type(dto_field: &DtoField) -> String {
        let base_type = Self::get_dto_field_rust_base_type(dto_field);
        if dto_field.optional {
            format!("Option<{}>", base_type)
        } else if dto_field.is_list {
            format!("Vec<{}>", base_type)
        } else {
            base_type
        }
    }
    fn get_dto_field_rust_base_type(dto_field: &DtoField) -> String {
        match dto_field.field_type {
            DtoFieldType::Boolean => "bool".to_string(),
            DtoFieldType::Integer => "i64".to_string(),
            DtoFieldType::UInteger => "u64".to_string(),
            DtoFieldType::Float => "f64".to_string(),
            DtoFieldType::String => "String".to_string(),
            DtoFieldType::Uuid => "uuid::Uuid".to_string(),
            DtoFieldType::DateTime => "chrono::DateTime<chrono::Utc>".to_string(),
            DtoFieldType::Enum => dto_field
                .enum_name
                .clone()
                .unwrap_or("enum_name not set".to_string()),
        }
    }

    fn build_dto_field_vm(df: &DtoField) -> DtoFieldVM {
        let (
            rust_enum_variants,
            parsed_variants,
            enum_needs_uuid,
            enum_needs_chrono,
            enum_needs_entity_id,
            enum_needs_float,
            external_enum_references,
        ) = if df.field_type == DtoFieldType::Enum {
            build_parsed_variants(&df.enum_values)
        } else {
            (vec![], vec![], false, false, false, false, vec![])
        };
        DtoFieldVM {
            inner: df.clone(),
            pascal_name: heck::AsPascalCase(&df.name).to_string(),
            snake_name: heck::AsSnakeCase(&df.name).to_string(),
            rust_base_type: Self::get_dto_field_rust_base_type(df),
            rust_type: Self::get_dto_field_rust_type(df),
            rust_enum_variants,
            parsed_variants,
            enum_needs_uuid,
            enum_needs_chrono,
            enum_needs_entity_id,
            enum_needs_float,
            external_enum_references,
        }
    }

    fn get_entity_owner(uow: &dyn GenerationOps, entity_id: &EntityId) -> Option<EntityId> {
        let entity: Option<Entity> = uow.get_entity(entity_id).ok().flatten();
        if let Some(entity) = entity {
            let relationships = uow
                .get_relationship_multi(entity.relationships.as_slice())
                .ok()?;
            for rel in relationships.into_iter().flatten() {
                if rel.right_entity == Some(*entity_id) && rel.strength == Strength::Strong {
                    return rel.left_entity;
                }
            }
        }
        None
    }

    fn get_entity_owner_relationship_field_pascal_name(
        uow: &dyn GenerationOps,
        entity_id: &EntityId,
    ) -> Option<String> {
        let entity: Option<Entity> = uow.get_entity(entity_id).ok().flatten();
        if let Some(entity) = entity {
            let relationships = uow
                .get_relationship_multi(entity.relationships.as_slice())
                .ok()?;
            for rel in relationships.into_iter().flatten() {
                if rel.right_entity == Some(*entity_id) && rel.strength == Strength::Strong {
                    return Some(heck::AsPascalCase(rel.field_name.clone()).to_string());
                }
            }
        }
        None
    }

    fn get_entity_owner_relationship_field_snake_name(
        uow: &dyn GenerationOps,
        entity_id: &EntityId,
    ) -> Option<String> {
        let entity: Option<Entity> = uow.get_entity(entity_id).ok().flatten();
        if let Some(entity) = entity {
            let relationships = uow
                .get_relationship_multi(entity.relationships.as_slice())
                .ok()?;
            for rel in relationships.into_iter().flatten() {
                if rel.right_entity == Some(*entity_id) && rel.strength == Strength::Strong {
                    return Some(heck::AsSnakeCase(rel.field_name.clone()).to_string());
                }
            }
        }
        None
    }

    fn get_entity_owner_relationship_type(
        uow: &dyn GenerationOps,
        entity_id: &EntityId,
    ) -> Option<RelationshipType> {
        let entity: Option<Entity> = uow.get_entity(entity_id).ok().flatten();
        if let Some(entity) = entity {
            let relationships = uow
                .get_relationship_multi(entity.relationships.as_slice())
                .ok()?;
            for rel in relationships.into_iter().flatten() {
                if rel.right_entity == Some(*entity_id) && rel.strength == Strength::Strong {
                    return Some(rel.relationship_type);
                }
            }
        }
        None
    }

    pub(crate) fn for_file_id(
        uow: &dyn GenerationOps,
        file_id: EntityId,
        generation_snapshot_cache: &Vec<GenerationSnapshot>,
    ) -> anyhow::Result<(GenerationSnapshot, bool)> {
        // Load file
        let file = uow
            .get_file(&file_id)?
            .ok_or_else(|| anyhow!("File not found"))?;

        Self::for_file(uow, &file, generation_snapshot_cache)
    }

    pub(crate) fn for_file(
        uow: &dyn GenerationOps,
        file: &File,
        generation_snapshot_cache: &Vec<GenerationSnapshot>,
    ) -> anyhow::Result<(GenerationSnapshot, bool)> {
        // compare with cache
        for cached_snapshot in generation_snapshot_cache {
            let cached_file_vm = &cached_snapshot.file;
            if file.entity == cached_file_vm.inner.entity
                && file.feature == cached_file_vm.inner.feature
                && file.use_case == cached_file_vm.inner.use_case
                && file.all_features == cached_file_vm.inner.all_features
                && file.all_entities == cached_file_vm.inner.all_entities
                && file.all_use_cases == cached_file_vm.inner.all_use_cases
            {
                // cache hit

                let new_file_vm = FileVM {
                    inner: file.clone(),
                };

                let new_snapshot = GenerationSnapshot {
                    file: new_file_vm,
                    global: cached_snapshot.global.clone(),
                    ui: cached_snapshot.ui.clone(),
                    system: cached_snapshot.system.clone(),
                    entities: cached_snapshot.entities.clone(),
                    features: cached_snapshot.features.clone(),
                    use_cases: cached_snapshot.use_cases.clone(),
                    dtos: cached_snapshot.dtos.clone(),
                };

                log::debug!("Snapshot cache hit for file id {}", file.id);

                return Ok((new_snapshot, true));
            }
        }

        // system

        let system_id = tools::get_system_id(uow);

        let system = uow
            .get_system(&system_id?)?
            .ok_or_else(|| anyhow!("System not found"))?;

        let system_vm = SystemVM {
            inner: system.clone(),
        };

        // global

        let workspace_id = tools::get_workspace_id(uow)?;

        let global_ids = uow.get_workspace_relationship(
            &workspace_id,
            &common::direct_access::workspace::WorkspaceRelationshipField::Global,
        )?;

        let global = uow
            .get_global(
                global_ids
                    .first()
                    .expect("Workspace must have a global entity"),
            )?
            .expect("Workspace must have a global entity");

        let global_vm = GlobalVM {
            inner: global.clone(),
            application_kebab_name: heck::AsKebabCase(&global.application_name).to_string(),
            application_snake_name: heck::AsSnakeCase(&global.application_name).to_string(),
            prefix: if global.prefix_path.trim().is_empty() {
                "crates".to_string()
            } else {
                tools::strip_leading_and_trailing_slashes(&global.prefix_path)
            },
        };

        let ui_ids = uow.get_workspace_relationship(
            &workspace_id,
            &common::direct_access::workspace::WorkspaceRelationshipField::UserInterface,
        )?;

        let ui = uow
            .get_user_interface(ui_ids.first().expect("Workspace must have a UI entity"))?
            .expect("Workspace must have a UI entity");

        let ui_vm = UserInterfaceVM { inner: ui };

        // Working flat maps, then wrap into VMs
        let mut dto_fields: HashMap<EntityId, DtoField> = HashMap::new();
        let mut dtos: HashMap<EntityId, Dto> = HashMap::new();
        let mut use_cases: HashMap<EntityId, UseCase> = HashMap::new();
        let mut entities: HashMap<EntityId, Entity> = HashMap::new();
        let mut fields: HashMap<EntityId, Field> = HashMap::new();
        let mut features: HashMap<EntityId, Feature> = HashMap::new();

        // Feature scope
        if file.all_features {
            // Load all features via Root -> Features
            let feature_ids = uow.get_workspace_relationship(
                &workspace_id,
                &common::direct_access::workspace::WorkspaceRelationshipField::Features,
            )?;
            let feats = uow.get_feature_multi(&feature_ids)?;
            for feat_opt in feats.into_iter().flatten() {
                if feat_opt.use_cases.is_empty() {
                    continue;
                }
                let feature_use_cases: Vec<UseCase> = uow
                    .get_use_case_multi(&feat_opt.use_cases)?
                    .into_iter()
                    .flatten()
                    .collect();

                features.insert(feat_opt.id, feat_opt);

                for use_case in feature_use_cases {
                    // Entities for use case
                    let use_case_entities: Vec<Entity> = uow
                        .get_entity_multi(&use_case.entities)?
                        .into_iter()
                        .flatten()
                        .collect();
                    for e in use_case_entities {
                        entities.insert(e.id, e);
                    }

                    // DTOs
                    if let Some(dto_in) = use_case.dto_in {
                        let dto = uow
                            .get_dto(&dto_in)?
                            .ok_or_else(|| anyhow!("DTO missing for use case"))?;
                        let fields_vec: Vec<DtoField> = uow
                            .get_dto_field_multi(&dto.fields)?
                            .into_iter()
                            .flatten()
                            .collect();
                        for f in fields_vec {
                            dto_fields.insert(f.id, f);
                        }
                        dtos.insert(dto.id, dto);
                    }
                    if let Some(dto_out) = use_case.dto_out {
                        let dto = uow
                            .get_dto(&dto_out)?
                            .ok_or_else(|| anyhow!("DTO missing for use case"))?;
                        let fields_vec: Vec<DtoField> = uow
                            .get_dto_field_multi(&dto.fields)?
                            .into_iter()
                            .flatten()
                            .collect();
                        for f in fields_vec {
                            dto_fields.insert(f.id, f);
                        }
                        dtos.insert(dto.id, dto);
                    }

                    use_cases.insert(use_case.id, use_case);
                }
            }
        } else if let Some(feature_id) = file.feature {
            let feature = uow
                .get_feature(&feature_id)?
                .ok_or_else(|| anyhow!("Feature not found"))?;
            if feature.use_cases.is_empty() {
                return Err(anyhow!("Feature does not have an associated use case"));
            }
            let feature_use_cases: Vec<UseCase> = uow
                .get_use_case_multi(&feature.use_cases)?
                .into_iter()
                .flatten()
                .collect();

            features.insert(feature.id, feature);

            for use_case in feature_use_cases {
                if !use_case.entities.is_empty() {
                    // Entities for use case
                    let use_case_entities: Vec<Entity> = uow
                        .get_entity_multi(&use_case.entities)?
                        .into_iter()
                        .flatten()
                        .collect();
                    for e in use_case_entities {
                        entities.insert(e.id, e);
                    }
                }

                // DTOs
                if let Some(dto_in) = use_case.dto_in {
                    let dto = uow
                        .get_dto(&dto_in)?
                        .ok_or_else(|| anyhow!("DTO missing for use case"))?;
                    let fields_vec: Vec<DtoField> = uow
                        .get_dto_field_multi(&dto.fields)?
                        .into_iter()
                        .flatten()
                        .collect();
                    for f in fields_vec {
                        dto_fields.insert(f.id, f);
                    }
                    dtos.insert(dto.id, dto);
                }
                if let Some(dto_out) = use_case.dto_out {
                    let dto = uow
                        .get_dto(&dto_out)?
                        .ok_or_else(|| anyhow!("DTO missing for use case"))?;
                    let fields_vec: Vec<DtoField> = uow
                        .get_dto_field_multi(&dto.fields)?
                        .into_iter()
                        .flatten()
                        .collect();
                    for f in fields_vec {
                        dto_fields.insert(f.id, f);
                    }
                    dtos.insert(dto.id, dto);
                }

                use_cases.insert(use_case.id, use_case);
            }
        }

        // Single use case scope
        if let Some(use_case_id) = file.use_case {
            let use_case = uow
                .get_use_case(&use_case_id)?
                .ok_or_else(|| anyhow!("Use case not found"))?;

            if !use_case.entities.is_empty() {
                let use_case_entities: Vec<Entity> = uow
                    .get_entity_multi(&use_case.entities)?
                    .into_iter()
                    .flatten()
                    .collect();
                for e in use_case_entities {
                    entities.insert(e.id, e);
                }
            }

            if let Some(dto_in) = use_case.dto_in {
                let dto = uow
                    .get_dto(&dto_in)?
                    .ok_or_else(|| anyhow!("DTO missing for use case"))?;
                let fields_vec: Vec<DtoField> = uow
                    .get_dto_field_multi(&dto.fields)?
                    .into_iter()
                    .flatten()
                    .collect();
                for f in fields_vec {
                    dto_fields.insert(f.id, f);
                }
                dtos.insert(dto.id, dto);
            }
            if let Some(dto_out) = use_case.dto_out {
                let dto = uow
                    .get_dto(&dto_out)?
                    .ok_or_else(|| anyhow!("DTO missing for use case"))?;
                let fields_vec: Vec<DtoField> = uow
                    .get_dto_field_multi(&dto.fields)?
                    .into_iter()
                    .flatten()
                    .collect();
                for f in fields_vec {
                    dto_fields.insert(f.id, f);
                }
                dtos.insert(dto.id, dto);
            }
            use_cases.insert(use_case.id, use_case);
        }

        // Entity scope
        if file.all_entities {
            // Load all entities via Root -> Entities
            let entity_ids = uow.get_workspace_relationship(
                &workspace_id,
                &common::direct_access::workspace::WorkspaceRelationshipField::Entities,
            )?;
            let ents = uow.get_entity_multi(&entity_ids)?;
            for ent_opt in ents.into_iter().flatten() {
                // skip heritage entities; include only those allowed for direct access when generating direct access code
                if ent_opt.only_for_heritage {
                    continue;
                }

                // load fields
                let entity_fields: Vec<Field> = if ent_opt.fields.is_empty() {
                    vec![]
                } else {
                    uow.get_field_multi(&ent_opt.fields)?
                        .into_iter()
                        .flatten()
                        .collect()
                };
                for field in &entity_fields {
                    if let Some(eid) = field.entity
                        && field.field_type == FieldType::Entity
                        && let Some(ent_dep) = uow.get_entity(&eid)?
                    {
                        entities.insert(ent_dep.id, ent_dep);
                    }
                    fields.insert(field.id, field.clone());
                }
                entities.insert(ent_opt.id, ent_opt);
            }
        } else if let Some(entity_id) = file.entity {
            let entity = uow
                .get_entity(&entity_id)?
                .ok_or_else(|| anyhow!("Entity not found"))?;
            let entity_fields: Vec<Field> = if entity.fields.is_empty() {
                vec![]
            } else {
                uow.get_field_multi(&entity.fields)?
                    .into_iter()
                    .flatten()
                    .collect()
            };
            // load fields so as to list entity dependencies
            for field in &entity_fields {
                if let Some(eid) = field.entity
                    && field.field_type == FieldType::Entity
                {
                    let ent = uow
                        .get_entity(&eid)?
                        .ok_or_else(|| anyhow!("Entity not found"))?;
                    entities.insert(ent.id, ent);
                }
                fields.insert(field.id, field.clone());
            }
            entities.insert(entity.id, entity);
        }

        // Load relationships for all collected entities
        let mut relationships_map: IndexMap<EntityId, Relationship> = IndexMap::new();
        {
            let mut rel_ids: Vec<EntityId> = entities
                .values()
                .flat_map(|e| e.relationships.clone())
                .collect();
            rel_ids.sort();
            rel_ids.dedup();
            if !rel_ids.is_empty() {
                let rels = uow.get_relationship_multi(&rel_ids)?;
                for rel_opt in rels.into_iter().flatten() {
                    // Ensure both sides entities are available for templates if referenced
                    let left_entity_id = match rel_opt.left_entity {
                        Some(id) => id,
                        None => bail!("Relationship {} is missing left_entity", rel_opt.id),
                    };
                    let right_entity_id = match rel_opt.right_entity {
                        Some(id) => id,
                        None => bail!("Relationship {} is missing right_entity", rel_opt.id),
                    };

                    if !entities.contains_key(&left_entity_id)
                        && let Some(le) = uow.get_entity(&left_entity_id)?
                    {
                        entities.insert(le.id, le);
                    }
                    if !entities.contains_key(&right_entity_id)
                        && let Some(re) = uow.get_entity(&right_entity_id)?
                    {
                        entities.insert(re.id, re);
                    }
                    relationships_map.insert(rel_opt.id, rel_opt);
                }
            }
        }

        // Now wrap into snapshot similarly to adapter
        let mut entities_vm: IndexMap<EntityId, EntityVM> = IndexMap::new();
        let mut entity_keys: Vec<EntityId> = entities.keys().copied().collect();
        entity_keys.sort();
        for eid in &entity_keys {
            // Use the unified entity view model builder
            let evm = SnapshotBuilder::get_entity_vm(uow, eid)?;
            entities_vm.insert(*eid, evm);
        }

        let mut dtos_vm: IndexMap<EntityId, DtoVM> = IndexMap::new();
        let mut dto_keys: Vec<EntityId> = dtos.keys().copied().collect();
        dto_keys.sort();
        for did in &dto_keys {
            let d = &dtos[did];
            let mut df_vec: Vec<DtoFieldVM> = Vec::new();
            for dfid in &d.fields {
                if let Some(df) = dto_fields.get(dfid) {
                    df_vec.push(SnapshotBuilder::build_dto_field_vm(df));
                }
            }
            dtos_vm.insert(
                *did,
                DtoVM {
                    inner: d.clone(),
                    fields: df_vec,
                    pascal_name: heck::AsPascalCase(&d.name).to_string(),
                },
            );
        }

        let mut sorted_features: Vec<(EntityId, Feature)> = features.into_iter().collect();
        sorted_features.sort_by_key(|(k, _)| *k);
        let features_vm: IndexMap<EntityId, FeatureVM> = sorted_features
            .into_iter()
            .map(|(k, v)| {
                (
                    k,
                    FeatureVM {
                        inner: v.clone(),
                        use_cases: {
                            let use_cases_ids = v.use_cases;
                            let use_cases: IndexMap<EntityId, UseCase> = uow
                                .get_use_case_multi(&use_cases_ids)
                                .unwrap_or_default()
                                .into_iter()
                                .flatten()
                                .map(|uc| (uc.id, uc))
                                .collect();

                            let use_case_vms: IndexMap<EntityId, UseCaseVM> = use_cases
                                .into_iter()
                                .map(|(k, uc): (EntityId, UseCase)| {
                                    (
                                        k,
                                        UseCaseVM {
                                            inner: uc.clone(),
                                            entities: {
                                                let entities_id = uc.entities;
                                                if entities_id.is_empty() {
                                                    IndexMap::new()
                                                }
                                                else {
                                                let entity_vms: IndexMap<EntityId, EntityVM> = uow
                                                    .get_entity_multi(&entities_id)
                                                    .unwrap_or_default()
                                                    .into_iter()
                                                    .flatten()
                                                    .map(|e| {
                                                        (
                                                            e.id,
                                                            SnapshotBuilder::get_entity_vm(
                                                                uow, &e.id,
                                                            )
                                                            .unwrap_or(EntityVM {
                                                                inner: e,
                                                                relationships: Default::default(),
                                                                forward_relationships:
                                                                    Default::default(),
                                                                backward_relationships:
                                                                    Default::default(),
                                                                snake_name: "".to_string(),
                                                                pascal_name: "".to_string(),
                                                                fields: vec![],
                                                                normal_fields: vec![],
                                                                owner: None,
                                                                owner_pascal_name: None,
                                                                owner_snake_name: None,
                                                                owner_relationship_field_pascal_name: None,
                                                                owner_relationship_field_snake_name: None,
                                                                owner_relationship_type: None,
                                                                ownership_chain: vec![],
                                                            }),
                                                        )
                                                    })
                                                    .collect();
                                                entity_vms
                                                    }
                                            },
                                            dto_in: uc.dto_in.and_then(|dto_id| {
                                                dtos.get(&dto_id).map(|d| DtoVM {
                                                    inner: d.clone(),
                                                    fields: {
                                                        let mut df_vec: Vec<DtoFieldVM> = Vec::new();
                                                        for dfid in &d.fields {
                                                            if let Some(df) = dto_fields.get(dfid) {
                                                                df_vec.push(SnapshotBuilder::build_dto_field_vm(df));
                                                            }
                                                        }
                                                        df_vec
                                                    },
                                                    pascal_name: heck::AsPascalCase(&d.name)
                                                        .to_string(),
                                                })
                                            }),
                                            dto_out: uc.dto_out.and_then(|dto_id| {
                                                dtos.get(&dto_id).map(|d| DtoVM {
                                                    inner: d.clone(),
                                                    fields: {
                                                        let mut df_vec: Vec<DtoFieldVM> = Vec::new();
                                                        for dfid in &d.fields {
                                                            if let Some(df) = dto_fields.get(dfid) {
                                                                df_vec.push(SnapshotBuilder::build_dto_field_vm(df));
                                                            }
                                                        }
                                                        df_vec
                                                    },
                                                    pascal_name: heck::AsPascalCase(&d.name)
                                                        .to_string(),
                                                })
                                            }),
                                            pascal_name: heck::AsPascalCase(&uc.name).to_string(),
                                            snake_name: heck::AsSnakeCase(&uc.name).to_string(),
                                        },
                                    )
                                })
                                .collect();
                            use_case_vms
                        },
                        snake_name: heck::AsSnakeCase(&v.name).to_string(),
                        pascal_name: heck::AsPascalCase(&v.name).to_string(),
                    },
                )
            })
            .collect();
        let mut sorted_use_cases: Vec<(EntityId, UseCase)> = use_cases.into_iter().collect();
        sorted_use_cases.sort_by_key(|(k, _)| *k);
        let use_cases_vm: IndexMap<EntityId, UseCaseVM> = sorted_use_cases
            .into_iter()
            .map(|(k, uc)| {
                (
                    k,
                    UseCaseVM {
                        inner: uc.clone(),
                        entities: {
                            let entities_id = uc.entities;
                            if entities_id.is_empty() {
                                IndexMap::new()
                            } else {
                                uow.get_entity_multi(&entities_id)
                                    .unwrap_or_default()
                                    .into_iter()
                                    .flatten()
                                    .map(|e| {
                                        (
                                            e.id,
                                            SnapshotBuilder::get_entity_vm(uow, &e.id).unwrap_or(
                                                EntityVM {
                                                    inner: e,
                                                    relationships: Default::default(),
                                                    forward_relationships: Default::default(),
                                                    backward_relationships: Default::default(),
                                                    snake_name: "".to_string(),
                                                    pascal_name: "".to_string(),
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
                                            ),
                                        )
                                    })
                                    .collect()
                            }
                        },
                        dto_in: uc.dto_in.and_then(|dto_id| {
                            dtos.get(&dto_id).map(|d| DtoVM {
                                inner: d.clone(),
                                fields: {
                                    let mut df_vec: Vec<DtoFieldVM> = Vec::new();
                                    for dfid in &d.fields {
                                        if let Some(df) = dto_fields.get(dfid) {
                                            df_vec.push(SnapshotBuilder::build_dto_field_vm(df));
                                        }
                                    }
                                    df_vec
                                },
                                pascal_name: heck::AsPascalCase(&d.name).to_string(),
                            })
                        }),
                        dto_out: uc.dto_out.and_then(|dto_id| {
                            dtos.get(&dto_id).map(|d| DtoVM {
                                inner: d.clone(),
                                fields: {
                                    let mut df_vec: Vec<DtoFieldVM> = Vec::new();
                                    for dfid in &d.fields {
                                        if let Some(df) = dto_fields.get(dfid) {
                                            df_vec.push(SnapshotBuilder::build_dto_field_vm(df));
                                        }
                                    }
                                    df_vec
                                },
                                pascal_name: heck::AsPascalCase(&d.name).to_string(),
                            })
                        }),
                        pascal_name: heck::AsPascalCase(&uc.name).to_string(),
                        snake_name: heck::AsSnakeCase(&uc.name).to_string(),
                    },
                )
            })
            .collect();

        // compute entity_snake if entity scope
        Ok((
            GenerationSnapshot {
                file: FileVM {
                    inner: file.clone(),
                },
                global: global_vm,
                ui: ui_vm,
                system: system_vm,
                entities: entities_vm,
                features: features_vm,
                use_cases: use_cases_vm,
                dtos: dtos_vm,
            },
            false,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::entities::FileStatus;

    #[test]
    fn test_root_cargo_tera_template() {
        let tera = get_rust_tera();
        let mut context = Context::new();
        let snapshot = GenerationSnapshot {
            file: FileVM {
                inner: File {
                    id: 1,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                    name: "Cargo.toml".into(),
                    relative_path: "".into(),
                    group: "root".into(),
                    template_name: "root_cargo".into(),
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
                },
            },
            global: GlobalVM {
                inner: Global {
                    id: 1,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                    language: "rust".into(),
                    application_name: "test".into(),
                    organisation_name: "test".into(),
                    organisation_domain: "test".into(),
                    prefix_path: "".into(),
                },
                application_kebab_name: "test".into(),
                application_snake_name: "test".into(),
                prefix: "".into(),
            },
            ui: UserInterfaceVM {
                inner: UserInterface {
                    id: 1,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                    rust_cli: false,
                    rust_slint: false,
                    cpp_qt_qtwidgets: false,
                    cpp_qt_qtquick: false,
                    rust_ios: false,
                    rust_android: false,
                },
            },
            system: SystemVM {
                inner: System {
                    id: 1,
                    ..Default::default()
                },
            },
            entities: IndexMap::new(),
            features: IndexMap::new(),
            use_cases: IndexMap::new(),
            dtos: IndexMap::new(),
        };
        context.insert("s", &snapshot);
        let code = tera.render("root_cargo", &context).unwrap();
        println!("{}", code);
    }

    #[test]
    fn test_entity_dtos_template_generation() {
        // Build a minimal snapshot with an entity bound to the file and a couple of fields
        use common::entities::FieldType;
        let entity_id: EntityId = 1;
        let file = File {
            id: 10,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            name: "User DTOs".to_string(),
            relative_path: "src/user_dtos.rs".to_string(),
            group: "entities".to_string(),
            template_name: "entity_dtos".to_string(),
            generated_code: None,
            status: FileStatus::New,
            nature: Default::default(),
            feature: None,
            all_features: false,
            entity: Some(entity_id),
            all_entities: false,
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
        let entity = Entity {
            id: entity_id,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            name: "User".to_string(),
            only_for_heritage: false,
            inherits_from: None,
            single_model: true,

            fields: vec![100, 101],
            relationships: vec![],
            undoable: true,
        };
        let field_relationship = Field {
            id: 100,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            name: "name".to_string(),
            field_type: FieldType::Entity,
            entity: Some(entity_id),
            relationship: FieldRelationshipType::OneToOne,
            optional: true, // nullable
            is_list: false,
            strong: true,
            list_model: false,
            list_model_displayed_field: None,
            enum_name: None,
            enum_values: vec![],
        };
        let field_tags = Field {
            id: 101,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            name: "tags".to_string(),
            field_type: FieldType::String,
            entity: None,
            relationship: FieldRelationshipType::OneToMany,
            optional: true,
            is_list: false,
            strong: true,
            list_model: false,
            list_model_displayed_field: None,
            enum_name: None,
            enum_values: vec![],
        };

        let snapshot = GenerationSnapshot {
            file: FileVM { inner: file },
            global: GlobalVM {
                inner: global,
                application_kebab_name: "".to_string(),
                application_snake_name: "".to_string(),
                prefix: "".to_string(),
            },
            ui: UserInterfaceVM {
                inner: UserInterface {
                    id: 1,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                    rust_cli: false,
                    rust_slint: false,
                    cpp_qt_qtwidgets: false,
                    cpp_qt_qtquick: false,
                    rust_ios: false,
                    rust_android: false,
                },
            },
            system: SystemVM {
                inner: System {
                    id: 1,
                    ..Default::default()
                },
            },
            entities: {
                let mut m = IndexMap::new();
                // Build fields VM
                let fields_vm = vec![
                    FieldVM {
                        inner: field_relationship.clone(),
                        pascal_name: heck::AsPascalCase(&field_relationship.name).to_string(),
                        snake_name: heck::AsSnakeCase(&field_relationship.name).to_string(),
                        relationship: "OneToOne".to_string(),
                        optional: true,
                        is_list: false,
                        rust_base_type: "String".to_string(),
                        rust_type: "String".to_string(),
                        rust_enum_variants: vec![],
                        parsed_variants: vec![],
                        enum_needs_uuid: false,
                        enum_needs_chrono: false,
                        enum_needs_entity_id: false,
                        enum_needs_float: false,
                    },
                    FieldVM {
                        inner: field_tags.clone(),
                        pascal_name: heck::AsPascalCase(&field_tags.name).to_string(),
                        snake_name: heck::AsSnakeCase(&field_tags.name).to_string(),
                        relationship: "OneToMany".to_string(),
                        optional: true,
                        is_list: true,
                        rust_base_type: "String".to_string(),
                        rust_type: "Vec<String>".to_string(),
                        rust_enum_variants: vec![],
                        parsed_variants: vec![],
                        enum_needs_uuid: false,
                        enum_needs_chrono: false,
                        enum_needs_entity_id: false,
                        enum_needs_float: false,
                    },
                ];
                m.insert(
                    entity_id,
                    EntityVM {
                        inner: entity,
                        relationships: IndexMap::new(),
                        forward_relationships: IndexMap::new(),
                        backward_relationships: IndexMap::new(),
                        snake_name: "user".to_string(),
                        pascal_name: "User".to_string(),
                        normal_fields: fields_vm
                            .iter()
                            .filter(|f| f.inner.field_type != FieldType::Entity)
                            .cloned()
                            .collect(),
                        fields: fields_vm,
                        owner: None,
                        owner_pascal_name: None,
                        owner_snake_name: None,
                        owner_relationship_field_pascal_name: None,
                        owner_relationship_field_snake_name: None,
                        owner_relationship_type: None,
                        ownership_chain: vec![],
                    },
                );
                m
            },
            features: IndexMap::new(),
            use_cases: IndexMap::new(),
            dtos: IndexMap::new(),
        };

        // Render directly with Tera to allow adding a workaround for `null` literal in template
        let tera = get_rust_tera();
        let mut context = Context::new();
        context.insert("s", &snapshot);
        // Workaround: the template compares against `null`, which Tera treats as an identifier; provide it explicitly
        context.insert("null", &serde_json::Value::Null);
        let code = tera
            .render("entity_dtos", &context)
            .expect("rendering entity_dtos");
        println!("{}", code);

        // Basic assertion: when file has no bound entity, template emits a clear comment
        assert!(!code.contains("No entity bound to this file"));
    }
}
