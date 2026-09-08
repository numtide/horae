use std::collections::HashMap;
use std::io::Read;
use std::time::Duration;

use extism::{CurrentPlugin, Error, UserData, Val, ValType};
use serde_json::Value;
use sqlx::Acquire;

use super::database::PluginDatabase;

/// Per-plugin capabilities, separate from the application's global writer pool.
#[derive(Clone)]
pub struct HostState {
    /// This plugin's own configuration (from the `[config]` table in its manifest).
    config: HashMap<String, String>,
    database: Option<PluginDatabase>,
}

/// The wall-clock bound on an outbound `horae_http_post`.
const HTTP_TIMEOUT: Duration = Duration::from_secs(10);
const DB_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_HOST_BYTES: usize = 1024 * 1024;
const MAX_DB_ROWS: usize = 1000;

/// Register Horae's logging, SQL lookup, HTTP POST, and configuration functions.
pub fn host_functions(
    config: HashMap<String, String>,
    database: Option<PluginDatabase>,
) -> Vec<extism::Function> {
    let state = UserData::new(HostState { config, database });
    vec![
        extism::Function::new("horae_log", [ValType::I64], [], state.clone(), horae_log),
        extism::Function::new(
            "horae_db_query",
            [ValType::I64],
            [ValType::I64],
            state.clone(),
            horae_db_query,
        ),
        extism::Function::new(
            "horae_http_post",
            [ValType::I64],
            [ValType::I64],
            state.clone(),
            horae_http_post,
        ),
        extism::Function::new(
            "horae_config_get",
            [ValType::I64],
            [ValType::I64],
            state,
            horae_config_get,
        ),
    ]
}

/// Read a plugin-memory argument as a UTF-8 string.
fn read_input(plugin: &mut CurrentPlugin, val: &Val) -> Result<String, Error> {
    let bytes: &[u8] = plugin.memory_get_val(val)?;
    if bytes.len() > MAX_HOST_BYTES {
        return Err(Error::msg("host request exceeds the size limit"));
    }
    Ok(std::str::from_utf8(bytes)?.to_owned())
}

/// Hand a string back to the plugin as its return value (a memory offset).
fn write_output(plugin: &mut CurrentPlugin, out: &mut Val, s: &str) -> Result<(), Error> {
    if s.len() > MAX_HOST_BYTES {
        return Err(Error::msg("host response exceeds the size limit"));
    }
    let handle = plugin.memory_new(s)?;
    *out = plugin.memory_to_val(handle);
    Ok(())
}

/// `horae_log(level, message)` — structured logging annotated with the plugin name.
fn horae_log(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    _outputs: &mut [Val],
    _user_data: UserData<HostState>,
) -> Result<(), Error> {
    let msg = read_input(plugin, &inputs[0])?;

    #[derive(serde::Deserialize)]
    struct LogMsg {
        level: Option<String>,
        message: String,
    }

    if let Ok(log) = serde_json::from_str::<LogMsg>(&msg) {
        let level = log.level.as_deref().unwrap_or("info");
        match level {
            "error" => tracing::error!(target: "plugin", "{}", log.message),
            "warn" => tracing::warn!(target: "plugin", "{}", log.message),
            "debug" => tracing::debug!(target: "plugin", "{}", log.message),
            _ => tracing::info!(target: "plugin", "{}", log.message),
        }
    } else {
        tracing::info!(target: "plugin", "{msg}");
    }

    Ok(())
}

/// `horae_db_query(request_json) -> rows_json` — read-only SQL lookup.
///
/// Input is `{"sql": "...", "params": [...]}`; output is a JSON array of row
/// objects, or `{"error": "..."}` on failure. The prefix guard and subquery
/// wrapper constrain statement syntax, not the privileges of called functions.
/// Failures are returned as JSON, never panicked, so a plugin call is isolated.
fn horae_db_query(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostState>,
) -> Result<(), Error> {
    let input = read_input(plugin, &inputs[0])?;
    let database = user_data
        .get()?
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .database
        .clone();
    let out = run_db_query(&input, database.as_ref()).unwrap_or_else(error_json);
    write_output(plugin, &mut outputs[0], &out)
}

fn run_db_query(input: &str, database: Option<&PluginDatabase>) -> Result<String, String> {
    #[derive(serde::Deserialize)]
    struct Request {
        sql: String,
        #[serde(default)]
        params: Vec<Value>,
    }

    let req: Request = serde_json::from_str(input).map_err(|e| format!("invalid request: {e}"))?;

    if !statement_is_read_only(&req.sql) {
        return Err("only a single SELECT statement is permitted".to_string());
    }

    let database = database.ok_or(
        "plugin SQL is disabled: configure HORAE_PLUGIN_DATABASE_URL with a restricted login",
    )?;
    // The registry executes all WASM calls on blocking workers, so bridging
    // back to the async pool does not block a runtime worker, even on a
    // current-thread runtime.
    let handle =
        tokio::runtime::Handle::try_current().map_err(|_| "no async runtime".to_string())?;
    handle.block_on(execute_db_query(database.pool(), &req.sql, &req.params))
}

async fn execute_db_query(
    pool: &sqlx::PgPool,
    sql: &str,
    params: &[Value],
) -> Result<String, String> {
    let wrapped = wrap_select(sql);
    tokio::time::timeout(DB_TIMEOUT, async {
        let mut connection = pool.acquire().await?;
        // Even cancellation must discard session-level settings and advisory
        // locks; a plugin connection is never returned for another query.
        connection.close_on_drop();
        let mut tx = connection.begin().await?;
        sqlx::query!("SET TRANSACTION READ ONLY")
            .execute(&mut *tx)
            .await?;
        sqlx::query!("SET LOCAL statement_timeout = '5s'")
            .execute(&mut *tx)
            .await?;
        super::database::validate_connection(&mut tx)
            .await
            .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
        let mut q = sqlx::query_scalar::<sqlx::Postgres, Option<String>>(&wrapped);
        for p in params {
            q = match p {
                Value::Null => q.bind(Option::<String>::None),
                Value::Bool(b) => q.bind(*b),
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        q.bind(i)
                    } else if let Some(f) = n.as_f64() {
                        q.bind(f)
                    } else {
                        q.bind(n.to_string())
                    }
                }
                Value::String(s) => q.bind(s.clone()),
                other => q.bind(other.to_string()),
            };
        }
        let mut output = String::from("[");
        {
            let mut rows = q.fetch(&mut *tx);
            let mut count = 0;
            while let Some(row) = std::future::poll_fn(|cx| rows.as_mut().poll_next(cx)).await {
                if count == MAX_DB_ROWS {
                    return Err(sqlx::Error::Protocol(
                        "plugin query exceeds the row limit".into(),
                    ));
                }
                let row = row?.ok_or_else(|| {
                    sqlx::Error::Protocol("plugin query row exceeds the size limit".into())
                })?;
                if output.len() + row.len() + usize::from(count > 0) + 1 > MAX_HOST_BYTES {
                    return Err(sqlx::Error::Protocol(
                        "plugin query response exceeds the size limit".into(),
                    ));
                }
                if count > 0 {
                    output.push(',');
                }
                output.push_str(&row);
                count += 1;
            }
        }
        output.push(']');
        // Do not persist session settings changed by a plugin's SELECT.
        tx.rollback().await?;
        Ok::<_, sqlx::Error>(output)
    })
    .await
    .map_err(|_| "database query timed out".to_string())?
    .map_err(|e| format!("query failed: {e}"))
}

/// Whether `sql` passes the single-SELECT syntax guard. Leading whitespace and
/// `--` line comments are skipped; anything that is not `SELECT`/`WITH`, or that
/// packs a second statement after a `;`, is rejected.
fn statement_is_read_only(sql: &str) -> bool {
    let mut cleaned = String::with_capacity(sql.len());
    for line in sql.lines() {
        let line = match line.split_once("--") {
            Some((code, _comment)) => code,
            None => line,
        };
        cleaned.push_str(line);
        cleaned.push('\n');
    }
    let cleaned = cleaned.trim();

    // Reject a trailing second statement: a `;` is allowed only at the very end.
    if let Some(idx) = cleaned.find(';')
        && idx != cleaned.len() - 1
    {
        return false;
    }

    let head = cleaned.trim_start().to_ascii_uppercase();
    head.starts_with("SELECT") || head.starts_with("WITH")
}

/// Serialize bounded rows in Postgres without aggregating an unbounded JSON
/// array. An oversized row becomes NULL, which the receiver reports as an error.
fn wrap_select(sql: &str) -> String {
    let trimmed = sql.trim().trim_end_matches(';');
    format!(
        "SELECT CASE WHEN octet_length(row_to_json(_t)::text) <= {MAX_HOST_BYTES} THEN row_to_json(_t)::text END FROM ({trimmed}) AS _t LIMIT {}",
        MAX_DB_ROWS + 1
    )
}

/// `horae_http_post(request_json) -> response_json` — outbound HTTP POST.
///
/// Input is `{"url": "...", "body": <json>}`; output is `{"status": u16,
/// "body": "..."}`, or `{"error": "..."}` when the request could not be made.
/// Bounded by [`HTTP_TIMEOUT`]. Any failure is returned as JSON, never panicked.
fn horae_http_post(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    _user_data: UserData<HostState>,
) -> Result<(), Error> {
    let input = read_input(plugin, &inputs[0])?;
    let out = run_http_post(&input).unwrap_or_else(error_json);
    write_output(plugin, &mut outputs[0], &out)
}

fn run_http_post(input: &str) -> Result<String, String> {
    #[derive(serde::Deserialize)]
    struct Request {
        url: String,
        #[serde(default)]
        body: Value,
    }

    let req: Request = serde_json::from_str(input).map_err(|e| format!("invalid request: {e}"))?;
    let body = req.body.to_string();

    let agent = ureq::AgentBuilder::new().timeout(HTTP_TIMEOUT).build();

    // ureq returns `Err(Status(..))` for non-2xx; both carry a usable response.
    let response = match agent
        .post(&req.url)
        .set("Content-Type", "application/json")
        .send_string(&body)
    {
        Ok(r) => r,
        Err(ureq::Error::Status(_code, r)) => r,
        Err(ureq::Error::Transport(t)) => return Err(format!("request failed: {t}")),
    };

    Ok(http_response_json(
        response.status(),
        read_http_body(response.into_reader())?,
    ))
}

fn read_http_body(body: impl Read) -> Result<String, String> {
    let mut bytes = Vec::new();
    body.take((MAX_HOST_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("could not read HTTP response: {e}"))?;
    if bytes.len() > MAX_HOST_BYTES {
        return Err("HTTP response exceeds the size limit".into());
    }
    String::from_utf8(bytes).map_err(|e| format!("HTTP response is not UTF-8: {e}"))
}

/// Shape an HTTP status + body into the `{"status", "body"}` response contract.
fn http_response_json(status: u16, body: String) -> String {
    serde_json::json!({ "status": status, "body": body }).to_string()
}

/// `horae_config_get(request_json) -> value_json` — read this plugin's own config.
///
/// Input is `{"key": "..."}`; output is the JSON string value, or JSON `null`
/// when the key is unset. A plugin can only read its own `[config]` table.
fn horae_config_get(
    plugin: &mut CurrentPlugin,
    inputs: &[Val],
    outputs: &mut [Val],
    user_data: UserData<HostState>,
) -> Result<(), Error> {
    let input = read_input(plugin, &inputs[0])?;

    #[derive(serde::Deserialize)]
    struct Request {
        key: String,
    }

    let value = match serde_json::from_str::<Request>(&input) {
        Ok(req) => {
            let store = user_data.get()?;
            let state = store.lock().unwrap_or_else(|p| p.into_inner());
            state.config.get(&req.key).cloned()
        }
        Err(_) => None,
    };

    let out = serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string());
    write_output(plugin, &mut outputs[0], &out)
}

/// A `{"error": "..."}` JSON string for a failed host-function call.
fn error_json(message: String) -> String {
    serde_json::json!({ "error": message }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    mod database_tests;

    #[sqlx::test(migrations = "./migrations")]
    async fn database_queries_preserve_parameters_and_rollback_session_settings(
        pool: sqlx::PgPool,
    ) {
        let reader = super::super::database::tests::Reader::new(&pool).await;
        let db = &reader.pool;
        let rows = execute_db_query(
            db,
            "SELECT $1::bigint AS value, set_config('application_name', 'plugin-query', false)",
            &[serde_json::json!(42)],
        )
        .await
        .unwrap();
        let rows: Value = serde_json::from_str(&rows).unwrap();
        assert_eq!(rows[0]["value"], 42);
        let application =
            sqlx::query_scalar!(r#"SELECT current_setting('application_name') AS "name!""#)
                .fetch_one(db)
                .await
                .unwrap();
        assert_ne!(application, "plugin-query");
        reader.finish().await;
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn a_query_cannot_disable_its_own_deadline(pool: sqlx::PgPool) {
        let reader = super::super::database::tests::Reader::new(&pool).await;
        let sql = "SELECT set_config('statement_timeout', '0', true), pg_sleep(10)";
        let result = execute_db_query(&reader.pool, sql, &[]).await.unwrap_err();
        assert!(
            result.contains("timed out") || result.contains("timeout"),
            "{result}"
        );
        let wrapped = wrap_select(sql);
        tokio::time::timeout(DB_TIMEOUT, async {
            loop {
                let running = sqlx::query_scalar!(
                    r#"SELECT COUNT(*) AS "count!" FROM pg_stat_activity
                       WHERE datname = current_database() AND state = 'active'
                         AND query = $1 AND pid <> pg_backend_pid()"#,
                    wrapped,
                )
                .fetch_one(&pool)
                .await
                .unwrap();
                if running == 0 {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("the timed-out backend query must also finish");
        reader.finish().await;
    }

    mod statement_is_read_only {
        use super::*;

        #[test]
        fn plain_select_is_allowed() {
            assert!(statement_is_read_only("SELECT 1"));
            assert!(statement_is_read_only("  select id from projects  "));
        }

        #[test]
        fn a_cte_select_is_allowed() {
            assert!(statement_is_read_only(
                "WITH t AS (SELECT 1 AS n) SELECT n FROM t"
            ));
        }

        #[test]
        fn writes_are_rejected() {
            assert!(!statement_is_read_only("INSERT INTO users VALUES (1)"));
            assert!(!statement_is_read_only("UPDATE users SET name = 'x'"));
            assert!(!statement_is_read_only("DELETE FROM users"));
            assert!(!statement_is_read_only("DROP TABLE users"));
        }

        #[test]
        fn a_second_statement_is_rejected() {
            assert!(!statement_is_read_only("SELECT 1; DROP TABLE users"));
        }

        #[test]
        fn a_single_trailing_semicolon_is_allowed() {
            assert!(statement_is_read_only("SELECT 1;"));
        }

        #[test]
        fn a_write_hidden_behind_a_comment_is_rejected() {
            // The comment is stripped, exposing the real leading keyword.
            assert!(!statement_is_read_only("-- harmless\nDELETE FROM users"));
        }
    }

    mod wrap_select {
        use super::*;

        #[test]
        fn wraps_as_a_bounded_row_query() {
            assert_eq!(
                wrap_select("SELECT id FROM projects"),
                "SELECT CASE WHEN octet_length(row_to_json(_t)::text) <= 1048576 THEN row_to_json(_t)::text END FROM (SELECT id FROM projects) AS _t LIMIT 1001"
            );
        }

        #[test]
        fn strips_a_trailing_semicolon_before_wrapping() {
            assert_eq!(
                wrap_select("SELECT 1;"),
                "SELECT CASE WHEN octet_length(row_to_json(_t)::text) <= 1048576 THEN row_to_json(_t)::text END FROM (SELECT 1) AS _t LIMIT 1001"
            );
        }
    }

    mod config_lookup {
        use super::*;

        fn state() -> HostState {
            HostState {
                config: HashMap::from([("webhook_url".to_string(), "https://x".to_string())]),
                database: None,
            }
        }

        #[test]
        fn a_set_key_is_returned() {
            assert_eq!(
                state().config.get("webhook_url").cloned(),
                Some("https://x".to_string())
            );
        }

        #[test]
        fn an_unset_key_is_none() {
            assert_eq!(state().config.get("missing").cloned(), None);
        }
    }

    mod response_shaping {
        use super::*;

        #[test]
        fn http_body_accepts_the_size_limit_and_rejects_larger_responses() {
            let body = vec![b'x'; MAX_HOST_BYTES];
            assert_eq!(
                read_http_body(body.as_slice()).unwrap().len(),
                MAX_HOST_BYTES
            );
            let oversized = vec![b'x'; MAX_HOST_BYTES + 1];
            assert!(
                read_http_body(oversized.as_slice())
                    .unwrap_err()
                    .contains("size limit")
            );
        }

        #[test]
        fn invalid_http_utf8_is_an_error_not_an_empty_success() {
            assert!(
                read_http_body([0xff].as_slice())
                    .unwrap_err()
                    .contains("UTF-8")
            );
        }

        #[test]
        fn http_response_carries_status_and_body() {
            let json = http_response_json(200, "ok".to_string());
            assert!(json.contains("\"status\":200"));
            assert!(json.contains("\"body\":\"ok\""));
        }

        #[test]
        fn error_json_wraps_the_message() {
            assert_eq!(error_json("boom".to_string()), "{\"error\":\"boom\"}");
        }
    }
}
