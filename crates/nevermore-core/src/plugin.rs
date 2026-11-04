use serde::{Deserialize, Serialize};

/// Standardized manifest describing a Nevermore sidebar plugin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub title: String,
    pub icon: String,
    pub order: u32,
    pub description: String,
}

impl PluginManifest {
    pub fn validate(&self) -> Result<(), String> {
        Self::validate_id(&self.id)?;
        if self.title.trim().is_empty() {
            return Err("Plugin title cannot be empty".to_string());
        }
        Ok(())
    }

    pub fn validate_id(id: &str) -> Result<(), String> {
        if id.is_empty() || id.len() > 32 {
            return Err(format!(
                "Plugin ID '{}' length must be between 1 and 32 characters",
                id
            ));
        }
        if id.contains('/') || id.contains('\\') || id.contains("..") {
            return Err(format!(
                "Plugin ID '{}' contains illegal path traversal characters",
                id
            ));
        }
        for ch in id.chars() {
            if !(ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-') {
                return Err(format!(
                    "Plugin ID '{}' contains invalid character '{}' (only ^[a-z0-9_-]{{1,32}}$ allowed)",
                    id, ch
                ));
            }
        }
        Ok(())
    }

    pub fn from_toml(content: &str) -> Result<Self, String> {
        let manifest: Self =
            toml::from_str(content).map_err(|e| format!("Failed to parse plugin.toml: {}", e))?;
        manifest.validate()?;
        Ok(manifest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_manifest() {
        let sample = r#"
            id = "test-plugin"
            title = "Test Plugin"
            icon = "test.svg"
            order = 1
            description = "A test plugin for Nevermore"
        "#;
        let manifest = PluginManifest::from_toml(sample).expect("valid manifest");
        assert_eq!(manifest.id, "test-plugin");
        assert_eq!(manifest.title, "Test Plugin");
        assert_eq!(manifest.order, 1);
    }

    #[test]
    fn test_invalid_manifest_ids() {
        let bad_ids = [
            "../bad",
            "bad/path",
            "UPPERCASE",
            "has space",
            "special!char",
            "",
            "this_id_is_way_too_long_to_be_allowed_under_the_thirty_two_char_limit",
        ];
        for bad in bad_ids {
            let sample = format!(
                r#"
                id = "{}"
                title = "Bad"
                icon = "icon.svg"
                order = 1
                description = "Desc"
                "#,
                bad
            );
            assert!(
                PluginManifest::from_toml(&sample).is_err(),
                "Expected error for ID '{}'",
                bad
            );
        }
    }
}
