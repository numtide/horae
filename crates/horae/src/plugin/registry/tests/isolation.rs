use super::*;

fn observing_registry(
    block: Option<std::sync::mpsc::Receiver<()>>,
) -> (
    PluginRegistry,
    tokio::sync::oneshot::Receiver<std::thread::ThreadId>,
) {
    let (send, receive) = tokio::sync::oneshot::channel();
    let observer = extism::Function::new(
        "observe_thread",
        [],
        [],
        extism::UserData::new((Some(send), block)),
        |_, _, _, data| {
            let data = data.get()?;
            let mut data = data.lock().unwrap();
            if let Some(send) = data.0.take() {
                let _ = send.send(std::thread::current().id());
            }
            if let Some(receive) = &data.1 {
                receive.recv_timeout(std::time::Duration::from_secs(10))?;
            }
            Ok(())
        },
    );
    let wasm = br#"(module
        (import "extism:host/user" "observe_thread" (func $observe))
        (func (export "user_logged_in") (result i32) call $observe i32.const 0)
        (func (export "dashboard_widget") (result i32) call $observe i32.const 0)
    )"#;
    let plugin = extism::Plugin::new(wasm.as_slice(), [observer], false).unwrap();
    let mut registry = PluginRegistry::empty();
    registry.plugins.push(Arc::new(LoadedPlugin {
        manifest: PluginManifest {
            name: "thread-observer".into(),
            version: "1.0.0".into(),
            hooks: vec!["user_logged_in".into()],
            config: HashMap::new(),
        },
        plugin: Arc::new(Mutex::new(plugin)),
    }));
    registry.hook_index.insert("user_logged_in".into(), vec![0]);
    (registry, receive)
}

fn login_event() -> AppEvent {
    AppEvent::UserLoggedIn {
        occurred_at: chrono::Utc::now(),
        org_id: uuid::Uuid::now_v7(),
        user: UserPayload {
            id: uuid::Uuid::now_v7(),
            email: "test@example.com".into(),
            name: "Test".into(),
            org_role: "member".into(),
            method: Some("dev".into()),
        },
    }
}

#[tokio::test]
async fn events_execute_outside_the_async_worker() {
    let (registry, observed) = observing_registry(None);
    let caller = std::thread::current().id();
    registry.dispatch(login_event());
    let worker = tokio::time::timeout(std::time::Duration::from_secs(10), observed)
        .await
        .unwrap()
        .unwrap();
    assert_ne!(worker, caller);
}

#[tokio::test]
async fn widgets_execute_outside_the_async_worker() {
    let (registry, observed) = observing_registry(None);
    let caller = std::thread::current().id();
    registry.collect_widgets().await;
    assert_ne!(observed.await.unwrap(), caller);
}

#[tokio::test]
async fn pending_capacity_rejects_new_work_without_spawning_tasks() {
    let (registry, observed) = observing_registry(None);
    let capacity = Arc::clone(&registry.pending)
        .try_acquire_many_owned(MAX_PENDING_CALLS as u32)
        .unwrap();
    registry.dispatch(login_event());
    assert_eq!(Arc::strong_count(&registry.plugins[0]), 1);
    assert!(registry.collect_widgets().await.is_empty());
    drop(capacity);
    registry.dispatch(login_event());
    tokio::time::timeout(Duration::from_secs(10), observed)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn cancelling_a_waiter_keeps_capacity_reserved_until_the_host_call_exits() {
    let (release, blocked) = std::sync::mpsc::channel();
    let (registry, entered) = observing_registry(Some(blocked));
    let pending = Arc::clone(&registry.pending).try_acquire_owned().unwrap();
    let mut tasks = tokio::task::JoinSet::new();
    tasks.spawn(invoke(
        Arc::clone(&registry.plugins[0]),
        "user_logged_in",
        Vec::new(),
        pending,
        Arc::clone(&registry.workers),
    ));
    tokio::time::timeout(Duration::from_secs(10), entered)
        .await
        .unwrap()
        .unwrap();
    tasks.abort_all();
    assert!(tasks.join_next().await.unwrap().unwrap_err().is_cancelled());
    assert_eq!(registry.pending.available_permits(), MAX_PENDING_CALLS - 1);
    assert_eq!(registry.workers.available_permits(), MAX_RUNNING_CALLS - 1);
    assert!(registry.plugins[0].plugin.try_lock().is_err());
    release.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(10), async {
        while registry.pending.available_permits() != MAX_PENDING_CALLS {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(registry.workers.available_permits(), MAX_RUNNING_CALLS);
}

#[tokio::test]
async fn different_plugins_share_the_running_call_limit() {
    let (release, blocked) = std::sync::mpsc::channel();
    let (first, entered_first) = observing_registry(Some(blocked));
    let (second, mut entered_second) = observing_registry(None);
    let workers = Arc::new(Semaphore::new(1));
    let mut tasks = tokio::task::JoinSet::new();
    let invocation = |registry: &PluginRegistry| {
        let pending = Arc::clone(&registry.pending).try_acquire_owned().unwrap();
        invoke(
            Arc::clone(&registry.plugins[0]),
            "user_logged_in",
            Vec::new(),
            pending,
            Arc::clone(&workers),
        )
    };
    tasks.spawn(invocation(&first));
    tokio::time::timeout(Duration::from_secs(10), entered_first)
        .await
        .unwrap()
        .unwrap();
    tasks.spawn(invocation(&second));
    tokio::time::timeout(Duration::from_secs(10), async {
        while second.plugins[0].plugin.try_lock().is_ok() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(matches!(
        entered_second.try_recv(),
        Err(tokio::sync::oneshot::error::TryRecvError::Empty)
    ));
    release.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(10), entered_second)
        .await
        .unwrap()
        .unwrap();
    while let Some(result) = tasks.join_next().await {
        result.unwrap().unwrap();
    }
}

#[tokio::test]
async fn oversized_event_payloads_are_not_queued() {
    let (registry, entered) = observing_registry(None);
    let mut oversized = login_event();
    if let AppEvent::UserLoggedIn { user, .. } = &mut oversized {
        user.name = "x".repeat(MAX_CALL_BYTES + 1);
    }
    registry.dispatch(oversized);
    assert_eq!(Arc::strong_count(&registry.plugins[0]), 1);
    registry.dispatch(login_event());
    tokio::time::timeout(Duration::from_secs(10), entered)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn oversized_wasm_output_is_rejected_before_copying_it() {
    let (registry, _) = observing_registry(None);
    let wasm = format!(
        r#"(module
        (import "extism:host/env" "alloc" (func $alloc (param i64) (result i64)))
        (import "extism:host/env" "output_set" (func $out (param i64 i64)))
        (func (export "user_logged_in") (result i32)
            (call $out (call $alloc (i64.const {size})) (i64.const {size}))
            i32.const 0)
    )"#,
        size = MAX_CALL_BYTES + 1
    );
    *registry.plugins[0].plugin.lock().await =
        extism::Plugin::new(wasm.as_str(), [], false).unwrap();
    let pending = Arc::clone(&registry.pending).try_acquire_owned().unwrap();
    let result = invoke(
        Arc::clone(&registry.plugins[0]),
        "user_logged_in",
        Vec::new(),
        pending,
        Arc::clone(&registry.workers),
    )
    .await;
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("output exceeds the size limit")
    );
}

#[tokio::test]
async fn a_wasm_timeout_releases_the_worker_and_instance() {
    let (registry, _) = observing_registry(None);
    let wasm = r#"(module (func (export "user_logged_in") (result i32)
        (loop $forever (br $forever)) i32.const 0))"#;
    *registry.plugins[0].plugin.lock().await = extism::Plugin::new(wasm, [], false).unwrap();
    let pending = Arc::clone(&registry.pending).try_acquire_owned().unwrap();
    let result = invoke(
        Arc::clone(&registry.plugins[0]),
        "user_logged_in",
        Vec::new(),
        pending,
        Arc::clone(&registry.workers),
    )
    .await;
    assert!(result.unwrap_err().to_string().contains("timed out"));
    tokio::time::timeout(Duration::from_secs(10), async {
        while registry.pending.available_permits() != MAX_PENDING_CALLS
            || registry.plugins[0].plugin.try_lock().is_err()
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(registry.workers.available_permits(), MAX_RUNNING_CALLS);
}

#[tokio::test]
async fn loaded_plugins_cannot_grow_memory_past_the_limit() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("plugin.toml"),
        "[plugin]\nname = 'memory-limit'\nversion = '1.0.0'\nhooks = ['user_logged_in']\n",
    )
    .unwrap();
    let wasm = format!(
        r#"(module
        (memory 1)
        (func (export "user_logged_in") (result i32)
            (if (i32.ne (memory.grow (i32.const {MAX_MEMORY_PAGES})) (i32.const -1))
                (then unreachable))
            i32.const 0))"#
    );
    std::fs::write(dir.path().join("memory.wasm"), wasm).unwrap();
    let mut registry = PluginRegistry::empty();
    registry.load_plugin(dir.path()).unwrap();
    for _ in 0..3 {
        let pending = Arc::clone(&registry.pending).try_acquire_owned().unwrap();
        let result = invoke(
            Arc::clone(&registry.plugins[0]),
            "user_logged_in",
            Vec::new(),
            pending,
            Arc::clone(&registry.workers),
        )
        .await;
        // Wasmtime rejects growth with -1; Extism may instead exhaust its
        // cumulative growth budget and trap. Neither may execute unreachable.
        if let Err(error) = result {
            assert_eq!(error.to_string(), "oom");
        }
    }
}

#[test]
fn loaded_plugins_cannot_start_with_excessive_memory() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("plugin.toml"),
        "[plugin]\nname = 'memory-limit'\nversion = '1.0.0'\nhooks = ['user_logged_in']\n",
    )
    .unwrap();
    let initial_pages = MAX_MEMORY_PAGES + 1;
    let wasm = format!(
        r#"(module
        (memory {initial_pages})
        (func (export "user_logged_in") (result i32) i32.const 0))"#
    );
    std::fs::write(dir.path().join("memory.wasm"), wasm).unwrap();
    let mut registry = PluginRegistry::empty();
    assert!(registry.load_plugin(dir.path()).is_err());
    assert_eq!(registry.plugin_count(), 0);
}
