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
    pub fn from_toml(content: &str) -> Result<Self, String> {
        toml::from_str(content).map_err(|e| format!("Failed to parse plugin.toml: {}", e))
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
}
