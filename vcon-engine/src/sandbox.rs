use crate::Manifest;

const ALLOWED_IMPORT_ROOTS: &[&str] = &["vcon"];
const NETWORK_IMPORT_ROOTS: &[&str] = &["socket", "urllib", "http", "requests", "asyncio"];
const BLOCKED_PERMISSIONS: &[&str] = &["network"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyViolation {
    BlockedPermission(String),
    NetworkImport(String),
    ImportNotAllowed(String),
}

impl std::fmt::Display for PolicyViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PolicyViolation::BlockedPermission(permission) => {
                write!(f, "permission `{permission}` is blocked in V1")
            }
            PolicyViolation::NetworkImport(module) => {
                write!(f, "network module `{module}` import is blocked in V1")
            }
            PolicyViolation::ImportNotAllowed(module) => {
                write!(f, "import `{module}` is outside SDK-facing APIs")
            }
        }
    }
}

pub fn validate_manifest_permissions(manifest: &Manifest) -> Vec<PolicyViolation> {
    manifest
        .permissions
        .iter()
        .filter(|permission| BLOCKED_PERMISSIONS.contains(&permission.as_str()))
        .map(|permission| PolicyViolation::BlockedPermission(permission.clone()))
        .collect()
}

pub fn scan_entrypoint_source(source: &str) -> Vec<PolicyViolation> {
    let mut violations = Vec::new();

    for module in extract_import_roots(source) {
        if NETWORK_IMPORT_ROOTS.contains(&module.as_str()) {
            violations.push(PolicyViolation::NetworkImport(module));
            continue;
        }

        if !ALLOWED_IMPORT_ROOTS.contains(&module.as_str()) {
            violations.push(PolicyViolation::ImportNotAllowed(module));
        }
    }

    violations
}

fn extract_import_roots(source: &str) -> Vec<String> {
    source
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("import ") {
                return Some(first_module_root(rest));
            }
            if let Some(rest) = trimmed.strip_prefix("from ") {
                return Some(first_module_root(rest));
            }
            None
        })
        .filter(|module| !module.is_empty())
        .collect()
}

fn first_module_root(input: &str) -> String {
    input
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end_matches(',')
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::{scan_entrypoint_source, validate_manifest_permissions, PolicyViolation};
    use crate::Manifest;

    #[test]
    fn blocks_network_permission() {
        let manifest = Manifest {
            id: "com.example.demo".to_owned(),
            name: "Demo".to_owned(),
            version: "0.1.0".to_owned(),
            entrypoint: "src/main.py".to_owned(),
            sdk_version: "1".to_owned(),
            assets_path: "assets".to_owned(),
            save_quota_mb: 8,
            permissions: vec!["storage".to_owned(), "network".to_owned()],
        };

        let violations = validate_manifest_permissions(&manifest);
        assert_eq!(
            violations,
            vec![PolicyViolation::BlockedPermission("network".to_owned())]
        );
    }

    #[test]
    fn blocks_non_sdk_and_network_imports() {
        let source = r#"
import vcon
import socket
from random import randint
"#;

        let violations = scan_entrypoint_source(source);
        assert_eq!(
            violations,
            vec![
                PolicyViolation::NetworkImport("socket".to_owned()),
                PolicyViolation::ImportNotAllowed("random".to_owned())
            ]
        );
    }

    #[test]
    fn allows_sdk_imports() {
        let source = r#"
import vcon
from vcon import input
"#;

        let violations = scan_entrypoint_source(source);
        assert!(violations.is_empty());
    }
}
