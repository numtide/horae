use super::*;
use crate::plugin::database::tests::Reader;

#[test]
fn unconfigured_sql_never_falls_back_to_application_state() {
    let result = run_db_query(r#"{"sql":"SELECT 1"}"#, None);
    assert!(result.unwrap_err().contains("plugin SQL is disabled"));
}

#[sqlx::test(migrations = "./migrations")]
async fn select_cannot_write_or_gain_application_privileges(pool: sqlx::PgPool) {
    let reader = Reader::new(&pool).await;
    let writer = sqlx::query_scalar!(r#"SELECT current_user AS "name!""#)
        .fetch_one(&pool)
        .await
        .unwrap();
    let create_object = execute_db_query(&reader.pool, "SELECT lo_create(0)", &[]).await;
    let disable_readonly = execute_db_query(
        &reader.pool,
        "SELECT set_config('transaction_read_only','off',true)",
        &[],
    )
    .await;
    let become_writer = execute_db_query(
        &reader.pool,
        "SELECT set_config('role',$1,true)",
        &[Value::String(writer)],
    )
    .await;
    let read_secret = execute_db_query(
        &reader.pool,
        "SELECT access_token_enc FROM harvest_credentials",
        &[],
    )
    .await;
    let allowed = execute_db_query(
        &reader.pool,
        "SELECT COUNT(*) AS entries FROM time_entries",
        &[],
    )
    .await;
    reader.finish().await;
    assert!(create_object.unwrap_err().contains("read-only"));
    assert!(disable_readonly.is_err());
    assert!(become_writer.unwrap_err().contains("permission denied"));
    assert!(read_secret.unwrap_err().contains("permission denied"));
    assert_eq!(
        serde_json::from_str::<Value>(&allowed.unwrap()).unwrap(),
        serde_json::json!([{"entries":0}])
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn query_results_are_bounded_without_silent_truncation(pool: sqlx::PgPool) {
    let reader = Reader::new(&pool).await;
    let boundary = execute_db_query(&reader.pool, "SELECT generate_series(1,1000) AS n", &[]).await;
    let too_many = execute_db_query(&reader.pool, "SELECT generate_series(1,1001) AS n", &[]).await;
    let huge_row =
        execute_db_query(&reader.pool, "SELECT repeat('x',1048577) AS payload", &[]).await;
    let huge_array = execute_db_query(
        &reader.pool,
        "SELECT repeat('x',600000) AS payload FROM generate_series(1,2)",
        &[],
    )
    .await;
    reader.finish().await;
    assert_eq!(
        serde_json::from_str::<Vec<Value>>(&boundary.unwrap())
            .unwrap()
            .len(),
        1000
    );
    assert!(too_many.unwrap_err().contains("row limit"));
    assert!(huge_row.unwrap_err().contains("row exceeds the size limit"));
    assert!(
        huge_array
            .unwrap_err()
            .contains("response exceeds the size limit")
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn wasm_host_uses_the_restricted_connection_on_a_current_thread_runtime(pool: sqlx::PgPool) {
    let reader = Reader::new(&pool).await;
    let database = reader.database().await;
    let output = tokio::task::spawn_blocking(move || {
        let wasm = r#"(module
            (import "extism:host/user" "horae_db_query" (func $query (param i64) (result i64)))
            (import "extism:host/env" "input_offset" (func $input (result i64)))
            (import "extism:host/env" "length" (func $length (param i64) (result i64)))
            (import "extism:host/env" "output_set" (func $output (param i64 i64)))
            (func (export "query") (result i32) (local $result i64)
                call $input call $query local.set $result
                local.get $result local.get $result call $length call $output
                i32.const 0))"#;
        let functions = host_functions(HashMap::new(), Some(database));
        let mut plugin = extism::Plugin::new(wasm, functions, false).unwrap();
        [
            r#"{"sql":"SELECT $1::bigint AS answer","params":[42]}"#,
            r#"{"sql":"SELECT lo_create(0)"}"#,
            r#"{"sql":"SELECT access_token_enc FROM harvest_credentials"}"#,
        ]
        .into_iter()
        .map(|request| plugin.call::<_, String>("query", request))
        .collect::<Result<Vec<_>, _>>()
    })
    .await
    .unwrap();
    reader.finish().await;
    let output: Vec<Value> = output
        .unwrap()
        .iter()
        .map(|value| serde_json::from_str(value).unwrap())
        .collect();
    assert_eq!(output[0], serde_json::json!([{"answer":42}]));
    assert!(output[1]["error"].as_str().unwrap().contains("read-only"));
    assert!(
        output[2]["error"]
            .as_str()
            .unwrap()
            .contains("permission denied")
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn query_discards_session_advisory_locks(pool: sqlx::PgPool) {
    let reader = Reader::new(&pool).await;
    let rows = execute_db_query(
        &reader.pool,
        "SELECT pg_backend_pid() AS pid, pg_advisory_lock(48372)",
        &[],
    )
    .await;
    reader.finish().await;
    let rows: Value = serde_json::from_str(&rows.unwrap()).unwrap();
    let pid = i32::try_from(rows[0]["pid"].as_i64().unwrap()).unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let held = sqlx::query_scalar!(r#"SELECT EXISTS(SELECT 1 FROM pg_locks WHERE pid = $1 AND locktype = 'advisory') AS "held!""#, pid)
                .fetch_one(&pool).await.unwrap();
            if !held { break; }
            tokio::task::yield_now().await;
        }
    }).await.expect("query session must release its advisory locks");
}
