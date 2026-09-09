use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

use super::event::KNOWN_HOOKS;

#[derive(Debug, Deserialize)]
struct ManifestFile {
    plugin: PluginManifest,
    /// Optional per-plugin configuration, exposed to the plugin via the
    /// `horae_config_get` host function. Lives in a top-level `[config]` table.
    #[serde(default)]
    config: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub hooks: Vec<String>,
    /// Populated from the file's top-level `[config]` table in `from_file`, not
    /// from the `[plugin]` table itself.
    #[serde(skip)]
    pub config: HashMap<String, String>,
}

impl PluginManifest {
    /// Parse a `plugin.toml` file. Returns an error if the file is malformed
    /// or declares hooks not in the known catalog.
    pub fn from_file(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let file: ManifestFile = toml::from_str(&content)?;
        let mut manifest = file.plugin;
        manifest.config = file.config;

        for hook in &manifest.hooks {
            if !KNOWN_HOOKS.contains(&hook.as_str()) {
                anyhow::bail!(
                    "plugin '{}': unknown hook '{}' (known: {})",
                    manifest.name,
                    hook,
                    KNOWN_HOOKS.join(", ")
                );
            }
        }

        if manifest.hooks.is_empty() {
            anyhow::bail!("plugin '{}': must declare at least one hook", manifest.name);
        }

        Ok(manifest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn accepts_all_published_business_events() {
        let hooks = [
            "time_entry_created",
            "time_entry_stopped",
            "invoice_created",
            "invoice_sent",
            "user_logged_in",
            "time_entry_updated",
            "time_entry_deleted",
            "timesheet_submitted",
            "submission_approved",
            "submission_rejected",
            "invoice_paid",
            "invoice_voided",
            "client_created",
            "client_updated",
            "client_deactivated",
            "client_reactivated",
            "project_created",
            "project_updated",
            "project_deactivated",
            "project_reactivated",
            "task_created",
            "task_updated",
            "task_deactivated",
            "task_reactivated",
            "user_created",
            "user_role_changed",
            "user_deactivated",
            "user_logged_out",
            "user_assigned_to_project",
            "assignment_removed",
            "org_branding_updated",
            "project_budget_threshold_reached",
            "project_over_budget",
            "timer_running_too_long",
        ];
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("plugin.toml");
        std::fs::write(
            &path,
            format!("[plugin]\nname = 'all-events'\nversion = '1.0.0'\nhooks = {hooks:?}\n"),
        )
        .unwrap();
        let manifest = PluginManifest::from_file(&path).unwrap();
        assert_eq!(manifest.hooks, hooks);
    }

    #[test]
    fn parse_valid_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("plugin.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"[plugin]
name = "test-plugin"
version = "1.0.0"
hooks = ["time_entry_created", "invoice_sent"]
"#
        )
        .unwrap();

        let m = PluginManifest::from_file(&path).unwrap();
        assert_eq!(m.name, "test-plugin");
        assert_eq!(m.hooks.len(), 2);
    }

    #[test]
    fn parses_optional_config_table() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("plugin.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"[plugin]
name = "webhook"
version = "1.0.0"
hooks = ["invoice_sent"]

[config]
webhook_url = "https://example.test/hook"
"#
        )
        .unwrap();

        let m = PluginManifest::from_file(&path).unwrap();
        assert_eq!(
            m.config.get("webhook_url").map(String::as_str),
            Some("https://example.test/hook")
        );
    }

    #[test]
    fn config_defaults_to_empty_when_absent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("plugin.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"[plugin]
name = "noconfig"
version = "1.0.0"
hooks = ["invoice_sent"]
"#
        )
        .unwrap();

        let m = PluginManifest::from_file(&path).unwrap();
        assert!(m.config.is_empty());
    }

    #[test]
    fn reject_unknown_hook() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("plugin.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"[plugin]
name = "bad"
version = "1.0.0"
hooks = ["nonexistent_event"]
"#
        )
        .unwrap();

        let err = PluginManifest::from_file(&path).unwrap_err();
        assert!(err.to_string().contains("unknown hook"));
    }

    #[test]
    fn reject_empty_hooks() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("plugin.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"[plugin]
name = "empty"
version = "1.0.0"
hooks = []
"#
        )
        .unwrap();

        let err = PluginManifest::from_file(&path).unwrap_err();
        assert!(err.to_string().contains("at least one hook"));
    }
}
