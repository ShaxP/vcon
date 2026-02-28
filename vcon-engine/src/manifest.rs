use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub entrypoint: String,
    pub sdk_version: String,
    pub assets_path: String,
    pub save_quota_mb: u32,
    #[serde(default)]
    pub permissions: Vec<String>,
}

impl Manifest {
    pub fn parse(input: &str) -> Result<Self, ManifestError> {
        let manifest: Manifest = toml::from_str(input)
            .map_err(|source| ManifestError::Parse(source.to_string()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.id.trim().is_empty() {
            return Err(ManifestError::Validation(
                "manifest key `id` must be a non-empty string".to_owned(),
            ));
        }
        if self.name.trim().is_empty() {
            return Err(ManifestError::Validation(
                "manifest key `name` must be a non-empty string".to_owned(),
            ));
        }
        if self.version.trim().is_empty() {
            return Err(ManifestError::Validation(
                "manifest key `version` must be a non-empty string".to_owned(),
            ));
        }
        if self.entrypoint.trim().is_empty() {
            return Err(ManifestError::Validation(
                "manifest key `entrypoint` must be a non-empty string".to_owned(),
            ));
        }
        if !self.entrypoint.ends_with(".py") {
            return Err(ManifestError::Validation(
                "manifest key `entrypoint` must point to a .py file".to_owned(),
            ));
        }
        if self.sdk_version.trim().is_empty() {
            return Err(ManifestError::Validation(
                "manifest key `sdk_version` must be a non-empty string".to_owned(),
            ));
        }
        if self.assets_path.trim().is_empty() {
            return Err(ManifestError::Validation(
                "manifest key `assets_path` must be a non-empty string".to_owned(),
            ));
        }
        if self.save_quota_mb == 0 {
            return Err(ManifestError::Validation(
                "manifest key `save_quota_mb` must be greater than 0".to_owned(),
            ));
        }

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("manifest parse error: {0}")]
    Parse(String),
    #[error("manifest validation error: {0}")]
    Validation(String),
}

#[cfg(test)]
mod tests {
    use super::Manifest;

    #[test]
    fn parses_valid_manifest() {
        let input = r#"
id = "com.example.demo"
name = "Demo"
version = "0.1.0"
entrypoint = "src/main.py"
sdk_version = "1"
assets_path = "assets"
save_quota_mb = 8
permissions = ["storage"]
"#;

        let manifest = Manifest::parse(input).expect("manifest should parse");
        assert_eq!(manifest.id, "com.example.demo");
        assert_eq!(manifest.permissions, vec!["storage"]);
    }

    #[test]
    fn rejects_missing_required_key() {
        let input = r#"
name = "Demo"
version = "0.1.0"
entrypoint = "src/main.py"
sdk_version = "1"
assets_path = "assets"
save_quota_mb = 8
permissions = ["storage"]
"#;

        let err = Manifest::parse(input).expect_err("missing id must fail");
        assert!(err.to_string().contains("missing field `id`"));
    }

    #[test]
    fn rejects_non_python_entrypoint() {
        let input = r#"
id = "com.example.demo"
name = "Demo"
version = "0.1.0"
entrypoint = "src/main.txt"
sdk_version = "1"
assets_path = "assets"
save_quota_mb = 8
permissions = ["storage"]
"#;

        let err = Manifest::parse(input).expect_err("invalid entrypoint should fail");
        assert!(err.to_string().contains("must point to a .py file"));
    }
}
