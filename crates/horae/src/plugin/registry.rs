use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};

use super::event::AppEvent;
use super::manifest::PluginManifest;

const CALL_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_PENDING_CALLS: usize = 64;
const MAX_RUNNING_CALLS: usize = 8;
const MAX_CALL_BYTES: usize = 1024 * 1024;
const MAX_MEMORY_PAGES: u32 = 1024;
const MAX_CALL_FUEL: u64 = 100_000_000;

/// A loaded plugin: its manifest and the extism plugin instance.
struct LoadedPlugin {
    manifest: PluginManifest,
    plugin: Arc<Mutex<extism::Plugin>>,
}

/// Registry of loaded WASM plugins, indexed by hook name for fast dispatch.
/// Held as `Arc<PluginRegistry>` in AppState.
pub struct PluginRegistry {
    /// All loaded plugins.
    plugins: Vec<Arc<LoadedPlugin>>,
    /// Hook name → indices into `plugins` for subscribed plugins.
    hook_index: HashMap<String, Vec<usize>>,
    pending: Arc<Semaphore>,
    workers: Arc<Semaphore>,
    database: Option<super::database::PluginDatabase>,
}

impl PluginRegistry {
    /// Create an empty registry (no plugins loaded).
    pub fn empty() -> Self {
        Self {
            plugins: Vec::new(),
            hook_index: HashMap::new(),
            pending: Arc::new(Semaphore::new(MAX_PENDING_CALLS)),
            workers: Arc::new(Semaphore::new(MAX_RUNNING_CALLS)),
            database: None,
        }
    }

    /// Scan a directory for plugin subdirectories, each containing a
    /// `plugin.toml` and a `*.wasm` file. Malformed plugins are logged
    /// and skipped (FR-018 edge case).
    pub async fn load(
        plugins_dir: &Path,
        database: Option<super::database::PluginDatabase>,
    ) -> anyhow::Result<Self> {
        let plugins_dir = plugins_dir.to_path_buf();
        Ok(
            tokio::task::spawn_blocking(move || Self::load_directory(&plugins_dir, database))
                .await?,
        )
    }

    fn load_directory(
        plugins_dir: &Path,
        database: Option<super::database::PluginDatabase>,
    ) -> Self {
        let mut registry = Self::empty();
        registry.database = database;

        if !plugins_dir.exists() {
            tracing::info!(
                "plugins directory does not exist: {}",
                plugins_dir.display()
            );
            return registry;
        }

        let entries = match std::fs::read_dir(plugins_dir) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("failed to read plugins directory: {e}");
                return registry;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            if let Err(e) = registry.load_plugin(&path) {
                tracing::warn!("skipping plugin at {}: {e:#}", path.display());
            }
        }

        tracing::info!(
            "loaded {} plugin(s), {} hook subscription(s)",
            registry.plugins.len(),
            registry.hook_index.values().map(|v| v.len()).sum::<usize>()
        );

        registry
    }

    fn load_plugin(&mut self, dir: &Path) -> anyhow::Result<()> {
        let manifest_path = dir.join("plugin.toml");
        let manifest = PluginManifest::from_file(&manifest_path)?;

        // Find the .wasm file
        let wasm_path = find_wasm(dir)?;

        // Build extism manifest for this plugin
        let wasm = extism::Wasm::file(&wasm_path);
        let extism_manifest = extism::Manifest::new([wasm])
            .with_timeout(CALL_TIMEOUT)
            .with_memory_max(MAX_MEMORY_PAGES);

        let host_functions =
            super::host::host_functions(manifest.config.clone(), self.database.clone());

        let mut allocation = wasmtime::PoolingAllocationConfig::default();
        // Extism links the kernel and guest before lazily instantiating the
        // callable guest. Leave bounded headroom for those auxiliary instances.
        allocation
            .max_memory_size(MAX_MEMORY_PAGES as usize * 65536)
            .max_memories_per_module(1)
            .total_memories(4)
            .total_tables(4)
            .total_core_instances(4)
            .total_component_instances(0);
        let mut engine = wasmtime::Config::new();
        engine.allocation_strategy(wasmtime::InstanceAllocationStrategy::Pooling(allocation));
        let plugin = extism::PluginBuilder::new(&extism_manifest)
            .with_functions(host_functions)
            .with_wasi(true)
            // Extism initializes the guest before starting its call timer.
            // Fuel covers that phase too and is replenished for every call.
            .with_fuel_limit(MAX_CALL_FUEL)
            .with_wasmtime_config(engine)
            .build()
            .with_context(|| format!("failed to load WASM {}", wasm_path.display()))?;

        // Verify the plugin exports the declared hook functions
        for hook in &manifest.hooks {
            if !plugin.function_exists(hook) {
                anyhow::bail!(
                    "plugin '{}' declares hook '{}' but does not export a matching WASM function",
                    manifest.name,
                    hook
                );
            }
        }

        let idx = self.plugins.len();
        let name = manifest.name.clone();
        let version = manifest.version.clone();
        let hooks = manifest.hooks.clone();

        self.plugins.push(Arc::new(LoadedPlugin {
            manifest,
            plugin: Arc::new(Mutex::new(plugin)),
        }));

        for hook in &hooks {
            self.hook_index.entry(hook.clone()).or_default().push(idx);
        }

        tracing::info!(
            "loaded plugin '{}' v{} for hooks: {}",
            name,
            version,
            hooks.join(", ")
        );
        Ok(())
    }

    /// Dispatch an event to all subscribed plugins concurrently.
    /// Failures are logged and isolated — the caller is never blocked (FR-021).
    pub fn dispatch(&self, event: AppEvent) {
        let hook = event.hook_name();
        let indices = match self.hook_index.get(hook) {
            Some(idx) if !idx.is_empty() => idx.clone(),
            _ => return,
        };

        let json = event.to_json();
        if json.len() > MAX_CALL_BYTES {
            tracing::warn!("plugin event '{hook}' exceeds the payload size limit");
            return;
        }

        for idx in indices {
            let plugin = Arc::clone(&self.plugins[idx]);
            let Ok(pending) = Arc::clone(&self.pending).try_acquire_owned() else {
                tracing::warn!(
                    "plugin '{}' skipped '{hook}': pending call limit reached",
                    plugin.manifest.name
                );
                continue;
            };
            let payload = json.as_bytes().to_vec();
            let workers = Arc::clone(&self.workers);

            tokio::spawn(async move {
                let name = plugin.manifest.name.clone();
                match invoke(plugin, hook, payload, pending, workers).await {
                    Ok(_) => tracing::debug!("plugin '{name}' handled '{hook}' successfully"),
                    Err(e) => tracing::warn!("plugin '{name}' failed on '{hook}': {e}"),
                }
            });
        }
    }

    /// Collect dashboard widgets from all plugins that export a
    /// `dashboard_widget` function. Returns structured widget data.
    pub async fn collect_widgets(&self) -> Vec<PluginWidget> {
        let mut widgets = Vec::new();

        for loaded in &self.plugins {
            let Ok(pending) = Arc::clone(&self.pending).try_acquire_owned() else {
                tracing::warn!(
                    "plugin '{}' widget skipped: pending call limit reached",
                    loaded.manifest.name
                );
                continue;
            };

            match invoke(
                Arc::clone(loaded),
                "dashboard_widget",
                Vec::new(),
                pending,
                Arc::clone(&self.workers),
            )
            .await
            {
                Ok(output) => {
                    let output_str = String::from_utf8_lossy(&output);
                    if let Ok(w) = serde_json::from_str::<WidgetResponse>(&output_str) {
                        widgets.push(PluginWidget {
                            plugin_name: loaded.manifest.name.clone(),
                            title: w.widget.title,
                            body_format: w.widget.body_format,
                            body: w.widget.body,
                        });
                    }
                }
                Err(e) => {
                    tracing::debug!(
                        "plugin '{}' dashboard_widget error: {e}",
                        loaded.manifest.name
                    );
                }
            }
        }

        widgets
    }

    #[cfg(test)]
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }
}

/// Bound queued work separately from running work. The blocking closure owns
/// both permits and the instance lock until it really exits, even if its async
/// caller is cancelled or times out. Host calls have their own I/O timeouts.
async fn invoke(
    loaded: Arc<LoadedPlugin>,
    hook: &'static str,
    payload: Vec<u8>,
    pending: OwnedSemaphorePermit,
    workers: Arc<Semaphore>,
) -> anyhow::Result<Vec<u8>> {
    let (mut plugin, worker) = tokio::time::timeout(CALL_TIMEOUT, async {
        let plugin = Arc::clone(&loaded.plugin).lock_owned().await;
        // A failed initializer can leave Extism's instance partially initialized.
        // Do not let a later call replenish its fuel and bypass initialization.
        if plugin
            .fuel_consumed()
            .is_some_and(|fuel| fuel >= MAX_CALL_FUEL)
        {
            anyhow::bail!("plugin fuel budget exhausted; restart required");
        }
        let worker = workers.acquire_owned().await?;
        Ok::<_, anyhow::Error>((plugin, worker))
    })
    .await
    .map_err(|_| anyhow::anyhow!("timed out waiting for a plugin worker"))??;
    let cancel = plugin.cancel_handle();
    let mut task = tokio::task::spawn_blocking(move || {
        let (_pending, _worker) = (pending, worker);
        let output: &[u8] = plugin.call(hook, payload.as_slice())?;
        if output.len() > MAX_CALL_BYTES {
            anyhow::bail!("plugin output exceeds the size limit");
        }
        Ok(output.to_vec())
    });
    match tokio::time::timeout(CALL_TIMEOUT, &mut task).await {
        Ok(result) => result?,
        Err(_) => {
            // Cancel WASM through the engine. An already-running synchronous
            // host call cannot be preempted; its permits remain held meanwhile.
            if let Err(e) = cancel.cancel() {
                tracing::warn!("could not cancel plugin '{}': {e}", loaded.manifest.name);
            }
            task.abort();
            anyhow::bail!("plugin call timed out");
        }
    }
}

/// Find a .wasm file in a plugin directory.
fn find_wasm(dir: &Path) -> anyhow::Result<std::path::PathBuf> {
    for entry in std::fs::read_dir(dir)?.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "wasm") {
            return Ok(path);
        }
    }
    anyhow::bail!("no .wasm file found in {}", dir.display())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginWidget {
    pub plugin_name: String,
    pub title: String,
    pub body_format: String,
    pub body: String,
}

#[derive(serde::Deserialize)]
struct WidgetResponse {
    widget: WidgetInner,
}

#[derive(serde::Deserialize)]
struct WidgetInner {
    title: String,
    body_format: String,
    body: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::event::{InvoicePayload, TimeEntryPayload, UserPayload};

    mod initialization;
    mod isolation;

    fn fixtures_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/plugins")
    }

    #[tokio::test]
    async fn load_empty_directory() {
        let dir = tempfile::tempdir().unwrap();
        let registry = PluginRegistry::load(dir.path(), None).await.unwrap();
        assert_eq!(registry.plugin_count(), 0);
    }

    #[tokio::test]
    async fn load_nonexistent_directory() {
        let registry = PluginRegistry::load(std::path::Path::new("/does/not/exist"), None)
            .await
            .unwrap();
        assert_eq!(registry.plugin_count(), 0);
    }

    #[tokio::test]
    async fn load_test_plugins() {
        let registry = PluginRegistry::load(&fixtures_dir(), None).await.unwrap();
        assert_eq!(registry.plugin_count(), 2);
    }

    #[tokio::test]
    async fn dispatch_to_echo_plugin_succeeds() {
        let registry = PluginRegistry::load(&fixtures_dir(), None).await.unwrap();

        let event = AppEvent::TimeEntryCreated {
            occurred_at: chrono::Utc::now(),
            org_id: uuid::Uuid::now_v7(),
            time_entry: TimeEntryPayload {
                id: uuid::Uuid::now_v7(),
                user_id: uuid::Uuid::now_v7(),
                project_id: uuid::Uuid::now_v7(),
                task_id: uuid::Uuid::now_v7(),
                spent_date: chrono::NaiveDate::from_ymd_opt(2026, 7, 13).unwrap(),
                minutes: 60,
                billable: true,
                is_running: false,
                notes: Some("test entry".into()),
                started_at: None,
            },
        };

        // Both echo-plugin and fail-plugin subscribe to time_entry_created.
        // Echo succeeds, fail panics — caller must not be affected.
        registry.dispatch(event);
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }

    #[tokio::test]
    async fn failing_plugin_does_not_block_caller() {
        let registry = PluginRegistry::load(&fixtures_dir(), None).await.unwrap();

        let event = AppEvent::TimeEntryCreated {
            occurred_at: chrono::Utc::now(),
            org_id: uuid::Uuid::now_v7(),
            time_entry: TimeEntryPayload {
                id: uuid::Uuid::now_v7(),
                user_id: uuid::Uuid::now_v7(),
                project_id: uuid::Uuid::now_v7(),
                task_id: uuid::Uuid::now_v7(),
                spent_date: chrono::NaiveDate::from_ymd_opt(2026, 7, 13).unwrap(),
                minutes: 30,
                billable: false,
                is_running: false,
                notes: None,
                started_at: None,
            },
        };

        // dispatch() must return immediately — it spawns tasks, never blocks.
        let before = std::time::Instant::now();
        registry.dispatch(event);
        let elapsed = before.elapsed();

        assert!(
            elapsed < std::time::Duration::from_millis(100),
            "dispatch() should return immediately, took {elapsed:?}"
        );

        // Wait for spawned tasks to complete/fail.
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }

    #[tokio::test]
    async fn dispatch_event_with_no_subscribers() {
        let registry = PluginRegistry::load(&fixtures_dir(), None).await.unwrap();

        // Neither test plugin subscribes to user_logged_in.
        let event = AppEvent::UserLoggedIn {
            occurred_at: chrono::Utc::now(),
            org_id: uuid::Uuid::now_v7(),
            user: UserPayload {
                id: uuid::Uuid::now_v7(),
                email: "test@example.com".into(),
                name: "Test".into(),
                org_role: "member".into(),
                method: Some("dev".into()),
            },
        };

        registry.dispatch(event);
    }

    #[tokio::test]
    async fn collect_widgets_from_echo_plugin() {
        let registry = PluginRegistry::load(&fixtures_dir(), None).await.unwrap();
        let widgets = registry.collect_widgets().await;

        // echo-plugin exports dashboard_widget; fail-plugin does not.
        assert_eq!(widgets.len(), 1);
        assert_eq!(widgets[0].plugin_name, "echo-plugin");
        assert_eq!(widgets[0].title, "Echo Plugin");
        assert!(widgets[0].body.contains("running"));
    }

    #[tokio::test]
    async fn dispatch_invoice_sent_to_echo_plugin() {
        let registry = PluginRegistry::load(&fixtures_dir(), None).await.unwrap();

        let event = AppEvent::InvoiceSent {
            occurred_at: chrono::Utc::now(),
            org_id: uuid::Uuid::now_v7(),
            invoice: InvoicePayload {
                id: uuid::Uuid::now_v7(),
                client_id: uuid::Uuid::now_v7(),
                invoice_number: "INV-202607-001".into(),
                status: "sent".into(),
                issue_date: chrono::NaiveDate::from_ymd_opt(2026, 7, 13).unwrap(),
                due_date: chrono::NaiveDate::from_ymd_opt(2026, 8, 12).unwrap(),
                currency: "EUR".into(),
                total_cents: 42000,
            },
        };

        registry.dispatch(event);
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
}
