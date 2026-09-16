//! Every persisted setting, declared in one place.
//!
//! `SettingsStore` is a dynamic dotted-key store: a key springs into existence the
//! first time somebody asks for it, and nothing anywhere knows the set of legal
//! keys. That is fine for the app and hostile to anyone writing a settings file by
//! hand, an automation probe most of all, where a typo is not an error but silence.
//! Declaring them here lets `--dump-config` print the effective configuration and
//! lets `--config` reject an unknown key by name.

/// Which theme the user asked for: `light` or `dark`.
///
/// Deliberately separate from the resolved answer below. If a `system` mode is ever
/// added, the question ("follow the OS") and the answer ("it is dark right now")
/// stop being the same value, and one key cannot hold both.
pub const THEME_MODE_KEY: &str = "ui.theme_mode";
pub const THEME_MODE_LIGHT: &str = "light";
pub const THEME_MODE_DARK: &str = "dark";
pub const THEME_MODE_DEFAULT: &str = THEME_MODE_LIGHT;

/// The resolved answer, mirrored from the live theme so the title-bar glyph and the
/// View menu radio can both bind one signal.
pub const DARK_KEY: &str = "ui.dark";

/// Milliseconds of quiet before a manifest edit triggers a validation run.
pub const CHECK_DEBOUNCE_MS_KEY: &str = "check.debounce_ms";
pub const CHECK_DEBOUNCE_MS_DEFAULT: i64 = 500;

/// One row of the settings schema: what a key is, what it defaults to, and what
/// counts as a legal value.
pub struct SettingSpec {
    pub key: &'static str,
    pub ty: &'static str,
    pub default: fn() -> toml::Value,
    pub check: fn(&toml::Value) -> Result<(), String>,
    pub doc: &'static str,
}

fn theme_mode_default() -> toml::Value {
    toml::Value::String(THEME_MODE_DEFAULT.to_string())
}

fn theme_mode_check(v: &toml::Value) -> Result<(), String> {
    match v.as_str() {
        Some(THEME_MODE_LIGHT) | Some(THEME_MODE_DARK) => Ok(()),
        _ => Err(format!(
            "expected one of: {THEME_MODE_LIGHT} | {THEME_MODE_DARK}"
        )),
    }
}

fn dark_default() -> toml::Value {
    toml::Value::Boolean(false)
}

fn bool_check(v: &toml::Value) -> Result<(), String> {
    if v.as_bool().is_some() {
        Ok(())
    } else {
        Err("expected a boolean".to_string())
    }
}

fn debounce_default() -> toml::Value {
    toml::Value::Integer(CHECK_DEBOUNCE_MS_DEFAULT)
}

fn debounce_check(v: &toml::Value) -> Result<(), String> {
    match v.as_integer() {
        Some(n) if (0..=10_000).contains(&n) => Ok(()),
        Some(_) => Err("expected 0..=10000".to_string()),
        None => Err("expected an integer".to_string()),
    }
}

/// The complete set. A key absent from this table is a key `--config` rejects.
pub static SETTINGS: &[SettingSpec] = &[
    SettingSpec {
        key: THEME_MODE_KEY,
        ty: "one of: light | dark",
        default: theme_mode_default,
        check: theme_mode_check,
        doc: "Which theme the user asked for.",
    },
    SettingSpec {
        key: DARK_KEY,
        ty: "boolean",
        default: dark_default,
        check: bool_check,
        doc: "The resolved answer, mirrored from the live theme.",
    },
    SettingSpec {
        key: CHECK_DEBOUNCE_MS_KEY,
        ty: "integer, 0..=10000",
        default: debounce_default,
        check: debounce_check,
        doc: "Milliseconds of quiet before a manifest edit triggers validation.",
    },
];

/// Look a key up in the schema.
pub fn spec_for(key: &str) -> Option<&'static SettingSpec> {
    SETTINGS.iter().find(|s| s.key == key)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every `*_KEY` const declared in this file has a row in `SETTINGS`.
    ///
    /// Reads this file's own source rather than a hand-kept list, so adding a key
    /// and forgetting the row fails here instead of silently shipping a setting
    /// `--config` cannot validate and `--dump-config` cannot print.
    #[test]
    fn every_declared_key_is_registered() {
        let src = include_str!("settings_keys.rs");
        let mut missing = Vec::new();
        for line in src.lines() {
            let line = line.trim();
            let Some(rest) = line.strip_prefix("pub const ") else {
                continue;
            };
            let Some((name, value)) = rest.split_once(": &str = ") else {
                continue;
            };
            if !name.ends_with("_KEY") {
                continue;
            }
            let key = value.trim().trim_end_matches(';').trim_matches('"');
            if spec_for(key).is_none() {
                missing.push(format!("{name} ({key})"));
            }
        }
        assert!(missing.is_empty(), "keys with no SETTINGS row: {missing:?}");
    }

    #[test]
    fn defaults_pass_their_own_validator() {
        for spec in SETTINGS {
            let v = (spec.default)();
            assert!(
                (spec.check)(&v).is_ok(),
                "default for {} fails its own check",
                spec.key
            );
        }
    }

    #[test]
    fn theme_mode_rejects_an_unknown_value() {
        assert!(theme_mode_check(&toml::Value::String("system".into())).is_err());
    }
}
