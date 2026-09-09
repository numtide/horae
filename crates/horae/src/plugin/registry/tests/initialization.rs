use super::*;

#[tokio::test]
async fn guest_initializers_have_an_engine_instruction_budget() {
    const PROBE: &str = "HORAE_PLUGIN_INITIALIZER_PROBE";
    if let Ok(initializer) = std::env::var(PROBE) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("plugin.toml"),
            "[plugin]\nname = 'initializer'\nversion = '1.0.0'\nhooks = ['user_logged_in']\n",
        )
        .unwrap();
        let declaration = match initializer.as_str() {
            "start" => "(func $init (loop $forever (br $forever))) (start $init)",
            "haskell" => {
                "(func (export \"hs_init\") (param i32 i32) (loop $forever (br $forever)))"
            }
            "reactor" => "(func (export \"_initialize\") (loop $forever (br $forever)))",
            "constructors" => "(func (export \"__wasm_call_ctors\") (loop $forever (br $forever)))",
            _ => panic!("unknown initializer probe"),
        };
        std::fs::write(
            dir.path().join("initializer.wasm"),
            format!(
                "(module {declaration} (func (export \"user_logged_in\") (result i32) i32.const 0))"
            ),
        )
        .unwrap();
        let mut registry = PluginRegistry::empty();
        if let Err(error) = registry.load_plugin(dir.path()) {
            assert!(
                format!("{error:#}").contains("fuel"),
                "{initializer}: {error:#}"
            );
            assert_eq!(registry.plugin_count(), 0);
            return;
        }
        for _ in 0..2 {
            let pending = Arc::clone(&registry.pending).try_acquire_owned().unwrap();
            let result = invoke(
                Arc::clone(&registry.plugins[0]),
                "user_logged_in",
                Vec::new(),
                pending,
                Arc::clone(&registry.workers),
            )
            .await;
            let error = format!("{:#}", result.unwrap_err());
            assert!(error.contains("fuel"), "{initializer}: {error}");
            assert_eq!(registry.pending.available_permits(), MAX_PENDING_CALLS);
            assert_eq!(registry.workers.available_permits(), MAX_RUNNING_CALLS);
            assert!(registry.plugins[0].plugin.try_lock().is_ok());
        }
        return;
    }

    // A regression must fail this test, not hang the test runtime's shutdown
    // while it waits for an uninterruptible initializer on a blocking thread.
    for initializer in ["start", "reactor", "constructors", "haskell"] {
        let mut child = tokio::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "plugin::registry::tests::initialization::guest_initializers_have_an_engine_instruction_budget",
                "--nocapture",
            ])
            .env(PROBE, initializer)
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        match tokio::time::timeout(Duration::from_secs(15), child.wait()).await {
            Ok(status) => assert!(status.unwrap().success(), "{initializer} probe failed"),
            Err(_) => {
                child.kill().await.unwrap();
                child.wait().await.unwrap();
                panic!("{initializer} initializer did not release its worker");
            }
        }
    }
}

#[tokio::test]
async fn instruction_budget_is_replenished_for_each_call() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("plugin.toml"),
        "[plugin]\nname = 'fuel-reset'\nversion = '1.0.0'\nhooks = ['user_logged_in']\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("loop.wasm"),
        r#"(module
            (func (export "user_logged_in") (result i32) (local $remaining i32)
                (local.set $remaining (i32.const 1000000))
                (loop $again
                    (local.set $remaining (i32.sub (local.get $remaining) (i32.const 1)))
                    (br_if $again (local.get $remaining)))
                i32.const 0))"#,
    )
    .unwrap();
    let mut registry = PluginRegistry::empty();
    registry.load_plugin(dir.path()).unwrap();
    let mut total_fuel = 0;
    for _ in 0..20 {
        let pending = Arc::clone(&registry.pending).try_acquire_owned().unwrap();
        invoke(
            Arc::clone(&registry.plugins[0]),
            "user_logged_in",
            Vec::new(),
            pending,
            Arc::clone(&registry.workers),
        )
        .await
        .unwrap();
        total_fuel += registry.plugins[0]
            .plugin
            .lock()
            .await
            .fuel_consumed()
            .unwrap();
    }
    assert!(total_fuel > MAX_CALL_FUEL, "total fuel: {total_fuel}");
    assert_eq!(registry.pending.available_permits(), MAX_PENDING_CALLS);
}
