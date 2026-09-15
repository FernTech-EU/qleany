use serde::{Deserialize, Serialize};

fn serde_true() -> bool {
    true
}

fn is_false(b: &bool) -> bool {
    !b
}

#[derive(Serialize, Deserialize)]
pub struct Schema {
    pub version: i32,
}

#[derive(Serialize, Deserialize)]
pub struct Organisation {
    pub name: String,
    pub domain: String,
}

#[derive(Serialize, Deserialize)]
pub struct Global {
    pub language: String,
    pub application_name: String,
    pub organisation: Organisation,
    pub prefix_path: String,
}

#[derive(Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub r#type: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relationship: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_list: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strong: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_model: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_model_displayed_field: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_name: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub enum_values: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Entity {
    pub name: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub only_for_heritage: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub single_model: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inherits_from: Option<String>,
    pub fields: Vec<Field>,
    #[serde(default = "serde_true")]
    pub undoable: bool,
}

#[derive(Serialize, Deserialize)]
pub struct DtoField {
    pub name: String,
    pub r#type: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_list: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_name: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub enum_values: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Dto {
    pub name: String,
    pub fields: Vec<DtoField>,
}

#[derive(Serialize, Deserialize)]
pub struct UseCase {
    pub name: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entities: Option<Vec<String>>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub undoable: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_only: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_operation: Option<bool>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dto_in: Option<Dto>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dto_out: Option<Dto>,
}

#[derive(Serialize, Deserialize)]
pub struct Feature {
    pub name: String,
    pub use_cases: Vec<UseCase>,
}

#[derive(Serialize, Deserialize)]
pub struct Ui {
    #[serde(default)]
    pub rust_cli: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub rust_slint: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub rust_teksilo: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub cpp_qt_qtwidgets: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub cpp_qt_qtquick: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub rust_ios: bool,
    #[serde(default)]
    #[serde(skip_serializing_if = "is_false")]
    pub rust_android: bool,
}

#[derive(Serialize, Deserialize)]
pub struct Manifest {
    pub schema: Schema,
    pub global: Global,
    pub entities: Vec<Entity>,
    pub features: Vec<Feature>,
    pub ui: Ui,
}
