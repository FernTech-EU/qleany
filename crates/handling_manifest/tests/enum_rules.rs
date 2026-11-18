//! Rule C50: an enum name shared by several fields must mean one enum.
//!
//! The generators emit one definition per name and scope (all entities share
//! `common::entities` and one C++ namespace; a feature's DTOs share one module),
//! so a reuse with the same values is fine and a reuse with other values must be
//! refused before it reaches the compiler.

use handling_manifest::handling_manifest_controller as controller;
use serde_yml::Value;
mod init;
mod rules_fixture;
use rules_fixture::{Fixture, fields};

const SAME_VALUES: &str = "must declare the same values";

fn enum_field(name: &str, enum_name: &str, values: &[&str]) -> Value {
    serde_yml::from_str(&format!(
        "name: {name}\ntype: enum\nenum_name: {enum_name}\nenum_values: [{}]\n",
        values.join(", ")
    ))
    .unwrap()
}

/// The fields of the DTO named `dto` in the use case's `dto_in` or `dto_out`.
fn dto_fields<'a>(manifest: &'a mut Value, dto: &str) -> &'a mut Vec<Value> {
    manifest["features"]
        .as_sequence_mut()
        .unwrap()
        .iter_mut()
        .flat_map(|f| f["use_cases"].as_sequence_mut().unwrap().iter_mut())
        .flat_map(|uc| {
            let uc = uc.as_mapping_mut().unwrap();
            uc.iter_mut()
                .filter(|(k, _)| matches!(k.as_str(), Some("dto_in") | Some("dto_out")))
                .map(|(_, v)| v)
        })
        .find(|d| d["name"].as_str() == Some(dto))
        .unwrap()["fields"]
        .as_sequence_mut()
        .unwrap()
}

fn critical_errors(f: &Fixture) -> Vec<String> {
    controller::check(&f.db, &f.hub).unwrap().critical_errors
}

#[test]
fn rule_is_listed() {
    assert!(
        controller::get_check_rules()
            .iter()
            .any(|rule| rule.id == "C50" && rule.severity == "critical")
    );
}

#[test]
fn entities_may_share_an_enum_with_the_same_values() {
    let f = Fixture::new(|m| {
        let status = &["Active", "OnHold", "Completed", "Archived"];
        // Across entities, and twice in one entity.
        fields(m, "Task").push(enum_field("project_status", "ProjectStatus", status));
        fields(m, "Project").push(enum_field("previous_status", "ProjectStatus", status));
    });
    let errors = critical_errors(&f);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn entities_sharing_an_enum_name_with_other_values_are_refused() {
    let f = Fixture::new(|m| {
        fields(m, "Task").push(enum_field(
            "project_status",
            "ProjectStatus",
            &["Active", "Done"],
        ));
    });
    let errors = critical_errors(&f);
    assert!(
        errors
            .iter()
            .any(|e| e.contains("ProjectStatus") && e.contains(SAME_VALUES)),
        "{errors:?}"
    );
}

#[test]
fn reordered_values_are_other_values() {
    // The order sets the default variant and the C++ integer values.
    let f = Fixture::new(|m| {
        fields(m, "Task").push(enum_field(
            "project_status",
            "ProjectStatus",
            &["OnHold", "Active", "Completed", "Archived"],
        ));
    });
    let errors = critical_errors(&f);
    assert!(errors.iter().any(|e| e.contains(SAME_VALUES)), "{errors:?}");
}

#[test]
fn dtos_of_one_feature_sharing_an_enum_name_with_other_values_are_refused() {
    // CreateProjectDto and ArchiveProjectDto both belong to project_management.
    let f = Fixture::new(|m| {
        dto_fields(m, "ArchiveProjectDto").push(enum_field(
            "status",
            "ProjectStatus",
            &["Open", "Closed"],
        ));
    });
    let errors = critical_errors(&f);
    assert!(
        errors
            .iter()
            .any(|e| e.contains("project_management") && e.contains(SAME_VALUES)),
        "{errors:?}"
    );
}

#[test]
fn dtos_of_different_features_may_reuse_a_name_with_other_values() {
    // ProjectStatsResultDto is project_management's only TaskDifficulty user;
    // task_management's DTOs keep [Easy, Medium, Hard, Expert]. Each feature
    // generates its own module, so the two enums are independent.
    let f = Fixture::new(|m| {
        for field in dto_fields(m, "ProjectStatsResultDto").iter_mut() {
            if field["enum_name"].as_str() == Some("TaskDifficulty") {
                field["enum_values"] = serde_yml::from_str("[Low, High]").unwrap();
            }
        }
    });
    let errors = critical_errors(&f);
    assert!(
        !errors.iter().any(|e| e.contains(SAME_VALUES)),
        "{errors:?}"
    );
}
