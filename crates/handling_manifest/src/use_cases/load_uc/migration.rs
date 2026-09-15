use crate::use_cases::common::CURRENT_SCHEMA_VERSION;
use anyhow::{Result, anyhow};
use serde_json::Value;

/// Migrate a manifest JSON value to the current schema version.
/// Operates on raw `serde_json::Value` before schema validation.
pub fn migrate_to_current(value: &mut Value) -> Result<()> {
    let version = value
        .get("schema")
        .and_then(|s| s.get("version"))
        .and_then(|v| v.as_i64())
        .ok_or_else(|| anyhow!("Missing or invalid schema.version"))?;

    if version > CURRENT_SCHEMA_VERSION {
        return Err(anyhow!(
            "Manifest schema version {} is newer than supported version {}",
            version,
            CURRENT_SCHEMA_VERSION
        ));
    }

    if version <= 2 {
        migrate_v2_to_v3(value);
    }
    if version <= 3 {
        migrate_v3_to_v4(value);
    }
    if version <= 4 {
        migrate_v4_to_v5(value);
    }
    if version <= 5 {
        migrate_v5_to_v6(value);
    }

    Ok(())
}

/// Migrate a v2 manifest to v3: remove `allow_direct_access` from entities, bump version.
fn migrate_v2_to_v3(value: &mut Value) {
    // Remove allow_direct_access from each entity
    if let Some(entities) = value.get_mut("entities").and_then(|e| e.as_array_mut()) {
        for entity in entities.iter_mut() {
            if let Some(obj) = entity.as_object_mut() {
                obj.remove("allow_direct_access");
            }
        }
    }

    // Bump version to 3
    if let Some(schema) = value.get_mut("schema").and_then(|s| s.as_object_mut()) {
        schema.insert("version".to_string(), Value::Number(3.into()));
    }
}

/// Migrate a v4 manifest to v5: additive change (is_list on entity fields), just bump version.
fn migrate_v4_to_v5(value: &mut Value) {
    if let Some(schema) = value.get_mut("schema").and_then(|s| s.as_object_mut()) {
        schema.insert("version".to_string(), Value::Number(5.into()));
    }
}

/// Migrate a v5 manifest to v6: additive change (`rust_teksilo` on the `ui`
/// block), just bump version.
fn migrate_v5_to_v6(value: &mut Value) {
    if let Some(schema) = value.get_mut("schema").and_then(|s| s.as_object_mut()) {
        schema.insert("version".to_string(), Value::Number(6.into()));
    }
}

/// Migrate a v3 manifest to v4: remove `validator` from use cases, bump version.
fn migrate_v3_to_v4(value: &mut Value) {
    // Remove validator from each use case
    if let Some(features) = value.get_mut("features").and_then(|u| u.as_array_mut()) {
        for feature in features.iter_mut() {
            if let Some(obj) = feature.as_object_mut()
                && let Some(use_cases) = obj.get_mut("use_cases").and_then(|u| u.as_array_mut())
            {
                for use_case in use_cases.iter_mut() {
                    if let Some(uc_obj) = use_case.as_object_mut() {
                        uc_obj.remove("validator");
                    }
                }
            }
        }
    }

    // Bump version to 4
    if let Some(schema) = value.get_mut("schema").and_then(|s| s.as_object_mut()) {
        schema.insert("version".to_string(), Value::Number(4.into()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_migrate_v2_strips_allow_direct_access() {
        let mut value = json!({
            "schema": { "version": 2 },
            "global": { "language": "rust", "application_name": "Test", "organisation": { "name": "Test", "domain": "test.com" }, "prefix_path": "" },
            "entities": [
                { "name": "Foo", "allow_direct_access": true, "fields": [] },
                { "name": "Bar", "allow_direct_access": false, "fields": [] }
            ],
            "features": [],
            "ui": {}
        });

        migrate_to_current(&mut value).unwrap();

        // Version bumped all the way to current
        assert_eq!(value["schema"]["version"], CURRENT_SCHEMA_VERSION);

        // allow_direct_access removed from all entities
        for entity in value["entities"].as_array().unwrap() {
            assert!(entity.get("allow_direct_access").is_none());
        }
    }

    #[test]
    fn test_v3_passes_through() {
        let mut value = json!({
            "schema": { "version": 3 },
            "global": { "language": "rust", "application_name": "Test", "organisation": { "name": "Test", "domain": "test.com" }, "prefix_path": "" },
            "entities": [
                { "name": "Foo", "fields": [] }
            ],
            "features": [],
            "ui": {}
        });

        migrate_to_current(&mut value).unwrap();
        assert_eq!(value["schema"]["version"], CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn test_migrate_v3_strips_validator() {
        let mut value = json!({
            "schema": { "version": 3 },
            "global": { "language": "rust", "application_name": "Test", "organisation": { "name": "Test", "domain": "test.com" }, "prefix_path": "" },
            "entities": [
            ],
            "features": [
                { "name": "Foo", "use_cases": [
                    { "name": "Bar", "validator": true }
                ]}
            ],
            "ui": {}
        });

        migrate_to_current(&mut value).unwrap();

        // Version bumped all the way to current
        assert_eq!(value["schema"]["version"], CURRENT_SCHEMA_VERSION);

        // validator removed from all use cases
        for feature in value["features"].as_array().unwrap() {
            for uc in feature["use_cases"].as_array().unwrap() {
                assert!(uc.get("validator").is_none());
            }
        }
    }

    #[test]
    fn test_migrate_v4_to_v5() {
        let mut value = json!({
            "schema": { "version": 4 },
            "global": { "language": "rust", "application_name": "Test", "organisation": { "name": "Test", "domain": "test.com" }, "prefix_path": "" },
            "entities": [],
            "features": [
                    { "name": "Foo", "use_cases": [
                        { "name": "Bar" }
                    ]}
            ],
            "ui": {}
        });

        migrate_to_current(&mut value).unwrap();
        assert_eq!(value["schema"]["version"], CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn test_migrate_v5_to_v6() {
        let mut value = json!({
            "schema": { "version": 5 },
            "global": { "language": "rust", "application_name": "Test", "organisation": { "name": "Test", "domain": "test.com" }, "prefix_path": "" },
            "entities": [],
            "features": [
                    { "name": "Foo", "use_cases": [
                        { "name": "Bar" }
                    ]}
            ],
            "ui": {}
        });

        migrate_to_current(&mut value).unwrap();
        assert_eq!(value["schema"]["version"], CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn test_v6_passes_through() {
        let mut value = json!({
            "schema": { "version": 6 },
            "global": { "language": "rust", "application_name": "Test", "organisation": { "name": "Test", "domain": "test.com" }, "prefix_path": "" },
            "entities": [],
            "features": [
                    { "name": "Foo", "use_cases": [
                        { "name": "Bar" }
                    ]}
            ],
            "ui": {}
        });

        migrate_to_current(&mut value).unwrap();
        assert_eq!(value["schema"]["version"], CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn test_future_version_errors() {
        let mut value = json!({
            "schema": { "version": 99 },
            "global": {},
            "entities": [],
            "features": [],
            "ui": {}
        });

        let result = migrate_to_current(&mut value);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("newer than supported")
        );
    }
}
