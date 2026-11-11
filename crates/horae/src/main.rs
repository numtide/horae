// Horae — Self-hostable time tracking
// Server entry: `horae serve` / `horae migrate`
// Web entry (WASM): `dioxus::launch(App)`

#[cfg(feature = "server")]
mod auth;
#[cfg(feature = "server")]
mod cli;
#[cfg(feature = "server")]
mod config;
#[cfg(feature = "server")]
mod db;
#[cfg(feature = "server")]
mod harvest;
#[cfg(feature = "server")]
mod importers;
#[cfg(feature = "server")]
mod init;
#[cfg(feature = "server")]
mod jobs;
#[cfg(feature = "server")]
mod notifications;
#[cfg(feature = "server")]
mod plugin;
#[cfg(feature = "server")]
mod render;
#[cfg(feature = "server")]
mod reports;
#[cfg(feature = "server")]
mod scheduler;
#[cfg(feature = "server")]
mod seed;
#[cfg(feature = "server")]
mod state;
#[cfg(feature = "server")]
mod timezone;

mod app;
mod components;
mod models;
mod pages;
mod route;
mod server_fns;
mod version;

#[cfg(feature = "server")]
fn main() -> anyhow::Result<()> {
    use clap::Parser;

    use cli::{Cli, Commands, MigrateAction};
    use config::AppConfig;

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            if error.use_stderr() && std::env::args_os().any(|arg| arg == "--json") {
                std::process::exit(i32::from(cli::imports::argument_error()));
            }
            error.exit();
        }
    };
    let remote = cli.remote.clone();
    let command = cli.command();
    if matches!(&command, Commands::Import { .. } | Commands::Jobs { .. }) {
        let code = cli::imports::run(&command, &remote);
        std::process::exit(i32::from(code));
    }
    let cfg = AppConfig::from_env()?;

    match command {
        Commands::Import { .. } | Commands::Jobs { .. } => {
            anyhow::bail!("Remote command was not dispatched");
        }
        Commands::Init {
            org_name,
            admin_email,
            admin_name,
        } => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                init_tracing(&cfg.log_level);
                let pool = db::create_pool(&cfg.database_url).await?;
                // A fresh installation has an empty database, so migrate before
                // inserting: `init` is the first command an operator runs.
                db::run_migrations(&pool).await?;
                init::run(&pool, &org_name, &admin_email, &admin_name).await?;
                anyhow::Ok(())
            })?;
        }

        Commands::Migrate { action } => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                init_tracing(&cfg.log_level);
                let pool = db::create_pool(&cfg.database_url).await?;
                match action {
                    MigrateAction::Run => {
                        tracing::info!("Running migrations...");
                        db::run_migrations(&pool).await?;
                        tracing::info!("Migrations complete.");
                    }
                    MigrateAction::Reset { confirm } => {
                        if !confirm {
                            eprintln!("Pass --confirm to reset the database.");
                            std::process::exit(1);
                        }
                        tracing::warn!("Dropping every table and re-applying migrations...");
                        db::reset(&pool).await?;
                        tracing::info!("Reset complete.");
                    }
                }
                anyhow::Ok(())
            })?;
        }

        Commands::User { action } => {
            // Refuse bad link input before connecting, so a typo never reaches
            // the database. Only the issuer variable is needed for the check.
            if let cli::UserAction::LinkOidc {
                user_id,
                issuer,
                subject,
            } = &action
            {
                use auth::oidc::link::{Rejection, validate};
                // An unreadable value fails closed rather than skipping the check.
                let checked = match std::env::var("HORAE_OIDC_ISSUER") {
                    Ok(configured) if !configured.is_empty() => {
                        validate(issuer, subject, Some(&configured))
                    }
                    Ok(_) | Err(std::env::VarError::NotPresent) => validate(issuer, subject, None),
                    Err(std::env::VarError::NotUnicode(_)) => {
                        Err(Rejection::UnreadableConfiguredIssuer)
                    }
                };
                if let Err(rejection) = checked {
                    eprintln!("{}", rejection.message(*user_id));
                    std::process::exit(1);
                }
            }
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                init_tracing(&cfg.log_level);
                let pool = db::create_pool(&cfg.database_url).await?;
                match action {
                    cli::UserAction::Create { email, name, role } => {
                        let org_role: horae_core::types::OrgRole =
                            role.parse().map_err(|e: String| anyhow::anyhow!(e))?;

                        let mut tx = pool.begin().await?;
                        sqlx::query!("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")
                            .execute(&mut *tx)
                            .await?;
                        // Organization row, then email: the lock order of every
                        // path that creates a user.
                        let org_id = sqlx::query_scalar!(
                            "SELECT id FROM organizations LIMIT 1 FOR KEY SHARE"
                        )
                        .fetch_optional(&mut *tx)
                        .await?
                        .ok_or_else(|| {
                            anyhow::anyhow!("No organization found. Run `init` first.")
                        })?;
                        if db::lock_new_user_email(&mut tx, &email).await? {
                            anyhow::bail!("A user with the email {email} already exists.");
                        }

                        let id = uuid::Uuid::now_v7();
                        sqlx::query!(
                            "INSERT INTO users (id, org_id, email, name, org_role) \
                             VALUES ($1, $2, $3, $4, $5)",
                            id,
                            org_id,
                            email,
                            name,
                            org_role as horae_core::types::OrgRole,
                        )
                        .execute(&mut *tx)
                        .await?;
                        // The operator's chosen role applies under either
                        // policy; the server's create_user is narrower.
                        server_fns::provisioning::provision(
                            &mut tx,
                            org_id,
                            id,
                            server_fns::provisioning::profile_for_role(org_role),
                        )
                        .await?;
                        tx.commit().await?;

                        println!("Created user: {} ({}) [{}]", name, email, role);
                    }
                    cli::UserAction::LinkOidc {
                        user_id,
                        issuer,
                        subject,
                    } => {
                        use auth::oidc::link::{LinkError, link};
                        match link(&pool, user_id, &issuer, &subject).await {
                            Ok(outcome) => println!("{}", outcome.message(user_id)),
                            Err(LinkError::Rejected(rejection)) => {
                                eprintln!("{}", rejection.message(user_id));
                                std::process::exit(1);
                            }
                            Err(LinkError::Database(error)) => return Err(error.into()),
                        }
                    }
                    cli::UserAction::List => {
                        let users = sqlx::query!(
                            "SELECT id, name, email, org_role::text as role, active \
                             FROM users ORDER BY name ASC"
                        )
                        .fetch_all(&pool)
                        .await?;

                        if users.is_empty() {
                            println!("No users found.");
                        } else {
                            println!(
                                "{:<36} {:<30} {:<30} {:<10} STATUS",
                                "ID", "NAME", "EMAIL", "ROLE"
                            );
                            for u in &users {
                                let status = if u.active { "active" } else { "inactive" };
                                println!(
                                    "{:<36} {:<30} {:<30} {:<10} {}",
                                    u.id,
                                    u.name,
                                    u.email,
                                    u.role.as_deref().unwrap_or("?"),
                                    status,
                                );
                            }
                        }
                    }
                }
                anyhow::Ok(())
            })?;
        }

        Commands::Seed => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                init_tracing(&cfg.log_level);
                let pool = db::create_pool(&cfg.database_url).await?;
                seed::run(&pool).await?;
                seed::verify(&pool).await?;
                anyhow::Ok(())
            })?;
        }

        Commands::Serve(args) => {
            init_tracing(&cfg.log_level);

            // Resolve bind address: CLI args, overridden by IP/PORT env vars when
            // running under `dx serve` (which sets those for hot-reload proxying).
            let host = std::env::var("IP").unwrap_or_else(|_| args.host.clone());
            let port = std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(args.port);
            if cfg.dev_login {
                tracing::warn!(
                    "DEV_LOGIN=1: /auth/dev-login signs anyone in as an admin without a \
                     password. Use it for local development only."
                );
            }
            config::check_dev_login_bind(cfg.dev_login, &host)?;

            let addr = format!("{}:{}", host, port);
            tracing::info!("Starting Horae on {addr}");

            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async move {
                use axum::routing::get;
                use dioxus::prelude::{DioxusRouterExt, ServeConfig};

                // Initialise DB + migrations eagerly so auth and server fns share the pool.
                let pool = db::create_pool(&cfg.database_url).await?;
                db::run_migrations(&pool).await?;

                // Load plugins from the configured directory (FR-018).
                let plugins_dir = std::path::Path::new(&cfg.plugins_dir);
                let plugin_database = match cfg.plugin_database_url.as_deref() {
                    Some(url) => Some(plugin::database::PluginDatabase::connect(url).await?),
                    None => None,
                };
                let registry = std::sync::Arc::new(
                    plugin::PluginRegistry::load(plugins_dir, plugin_database).await?,
                );
                state::init_state(
                    pool.clone(),
                    registry,
                    cfg.oidc.clone(),
                    cfg.harvest.clone(),
                    cfg.job_policy,
                    cfg.mail.clone(),
                )
                .await;

                // Start the background poller for forgotten timers (US3).
                scheduler::spawn(state::global_state().await);

                // Session middleware (Postgres-backed, idempotent migrate).
                let session_layer =
                    auth::make_session_layer(pool.clone(), cfg.secure_cookies).await?;

                // Build the Dioxus fullstack router (returns Router<()>), then
                // layer our own routes on top of it.
                let router = axum::Router::<dioxus::server::FullstackState>::new()
                    .serve_dioxus_application(ServeConfig::new(), app::App)
                    .route(
                        "/health",
                        get(|| async { axum::Json(serde_json::json!({"status": "ok"})) }),
                    )
                    // Browsers and crawlers request this path directly, whatever the page links.
                    .route(
                        "/favicon.ico",
                        get(|| async {
                            (
                                [(axum::http::header::CONTENT_TYPE, "image/x-icon")],
                                include_bytes!("../assets/favicon.ico").as_slice(),
                            )
                        }),
                    )
                    .route("/api/reports/export/csv", get(reports::export_csv))
                    .route("/api/reports/export/xlsx", get(reports::export_xlsx))
                    .route(
                        "/api/reports/time/grouped/csv",
                        get(reports::export_time_groups_csv),
                    )
                    .route(
                        "/api/reports/time/grouped/xlsx",
                        get(reports::export_time_groups_xlsx),
                    )
                    .route(
                        "/api/projects/export/csv",
                        get(reports::export_projects_csv),
                    )
                    .route(
                        "/api/projects/export/xlsx",
                        get(reports::export_projects_xlsx),
                    )
                    .route(
                        "/api/invoices/{id}/export/csv",
                        get(reports::export_invoice_csv),
                    )
                    .route(
                        "/api/invoices/{id}/export/xlsx",
                        get(reports::export_invoice_xlsx),
                    )
                    .route(
                        "/api/invoices/{id}/export/pdf",
                        get(reports::export_invoice_pdf),
                    )
                    .merge(auth::router(cfg.dev_login))
                    .merge(importers::harvest::callback_router())
                    .route(
                        "/api/import/harvest/jobs/{job_id}/errors",
                        get(jobs::report::download),
                    )
                    .merge(harvest::router(pool.clone()))
                    // Redirect signed-out page loads to /auth/login. Layered inside
                    // the session layer so the session is populated.
                    .layer(axum::middleware::from_fn(auth::login_redirect_guard))
                    .layer(session_layer);
                // Compression wraps everything, sessions included.
                let router = with_compression(router);

                let listener = tokio::net::TcpListener::bind(&addr).await?;
                let worker = jobs::spawn(state::global_state().await);
                let outbox = jobs::outbox::spawn(state::global_state().await);
                let mail = notifications::spawn(state::global_state().await);
                let stop_worker = worker.stop_sender();
                let stop_outbox = outbox.stop_sender();
                let stop_mail = mail.stop_sender();
                tracing::info!("Listening on {addr}");
                let server_result = axum::serve(listener, router)
                    .with_graceful_shutdown(async move {
                        shutdown_signal().await;
                        stop_worker.send_replace(true);
                        stop_outbox.send_replace(true);
                        stop_mail.send_replace(true);
                    })
                    .await;
                let grace = std::time::Duration::from_secs(30);
                let (worker_result, outbox_result, mail_result) = tokio::join!(
                    worker.shutdown(grace),
                    outbox.shutdown(grace),
                    mail.shutdown(grace)
                );
                worker_result?;
                outbox_result?;
                mail_result?;
                server_result?;
                anyhow::Ok(())
            })?;
        }
    }

    Ok(())
}

#[cfg(feature = "web")]
fn main() {
    dioxus::launch(app::App);
}

#[cfg(feature = "server")]
async fn shutdown_signal() {
    let interrupt = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::error!(%error, "could not listen for Ctrl-C; shutting down");
        }
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => tracing::error!(%error, "could not listen for SIGTERM; shutting down"),
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = interrupt => {},
        _ = terminate => {},
    }
}

/// Response compression for the whole server; see [`compression_layer`] and
/// [`vary_on_encoding`].
#[cfg(feature = "server")]
fn with_compression(router: axum::Router) -> axum::Router {
    router
        .layer(compression_layer())
        .layer(axum::middleware::map_response(vary_on_encoding))
}

/// Gzip responses the client accepts it for. Hashed static assets also have
/// build-time `.br` siblings (`pre_compress` in Dioxus.toml), which the Dioxus
/// file service serves with a `Content-Encoding` this layer leaves alone;
/// brotli is not done on the fly because its default quality is far too slow
/// for a multi-megabyte WASM bundle.
///
/// Deflate buffers its output, so if streaming SSR is enabled later its early
/// chunks would reach the browser late.
#[cfg(feature = "server")]
fn compression_layer()
-> tower_http::compression::CompressionLayer<impl tower_http::compression::Predicate> {
    use axum::http::header::CONTENT_TYPE;
    use tower_http::compression::{CompressionLayer, DefaultPredicate, Predicate};

    const SENT_AS_IS: [&str; 5] = [
        // Browsers get the precompressed `.br`; gzipping megabytes per request
        // for the rest is not worth the CPU.
        "application/wasm",
        // Already compressed formats.
        "application/pdf",
        "application/vnd.openxmlformats-",
        "application/zip",
        "font/woff2",
    ];
    CompressionLayer::new().compress_when(DefaultPredicate::new().and(
        |_, _, headers: &axum::http::HeaderMap, _: &axum::http::Extensions| {
            let content_type = headers
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default()
                .to_ascii_lowercase();
            !SENT_AS_IS
                .iter()
                .any(|prefix| content_type.starts_with(prefix))
        },
    ))
}

/// Every response varies by encoding: the Dioxus file service picks a
/// precompressed `.br` or the plain file by `Accept-Encoding` and caches either
/// as immutable, yet sends no `Vary`, and the compression layer only adds it to
/// responses its predicate allows. Without it a shared cache could hand
/// brotli, or a year of uncompressed WASM, to every client.
#[cfg(feature = "server")]
async fn vary_on_encoding(mut response: axum::response::Response) -> axum::response::Response {
    use axum::http::header::{ACCEPT_ENCODING, VARY};
    let headers = response.headers_mut();
    let varies = headers.get_all(VARY).iter().any(|value| {
        value.to_str().is_ok_and(|value| {
            value.split(',').any(|name| {
                name.trim().eq_ignore_ascii_case("accept-encoding") || name.trim() == "*"
            })
        })
    });
    if !varies {
        headers.append(VARY, ACCEPT_ENCODING.into());
    }
    response
}

#[cfg(all(test, feature = "server"))]
mod compression_tests {
    use std::time::Duration;

    use axum::{
        Router,
        body::Body,
        http::{Request, Response, header},
        response::Html,
        routing::get,
    };
    use futures_util::StreamExt;
    use tokio::sync::mpsc;
    use tower::{Layer, ServiceExt};

    fn gzip_request(uri: &str) -> Request<Body> {
        request(uri, "gzip")
    }

    fn request(uri: &str, accept_encoding: &str) -> Request<Body> {
        Request::get(uri)
            .header(header::ACCEPT_ENCODING, accept_encoding)
            .body(Body::empty())
            .unwrap()
    }

    fn assert_varies_once<B>(response: &Response<B>) {
        let vary: Vec<_> = response.headers().get_all(header::VARY).iter().collect();
        assert_eq!(vary, ["accept-encoding"]);
    }

    #[tokio::test]
    async fn html_is_gzipped_when_accepted() {
        let router = super::with_compression(
            Router::new().route("/", get(|| async { Html("<p>Loading…</p>".repeat(100)) })),
        );

        let response = router.oneshot(gzip_request("/")).await.unwrap();

        assert_eq!(response.headers()[header::CONTENT_ENCODING], "gzip");
        assert_varies_once(&response);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&body[..2], &[0x1f, 0x8b]);
    }

    #[tokio::test]
    async fn precompressed_wasm_varies_and_is_not_gzipped_per_request() {
        // The same file service Dioxus uses for the public directory.
        let dir = tempfile::tempdir().unwrap();
        let wasm = dir.path().join("app.wasm");
        std::fs::write(&wasm, vec![0u8; 4096]).unwrap();
        std::fs::write(dir.path().join("app.wasm.br"), b"brotli bytes").unwrap();
        let router = super::with_compression(Router::new().route_service(
            "/app.wasm",
            tower_http::services::ServeFile::new(&wasm).precompressed_br(),
        ));

        let brotli = router
            .clone()
            .oneshot(request("/app.wasm", "br, gzip"))
            .await
            .unwrap();
        assert_eq!(brotli.headers()[header::CONTENT_ENCODING], "br");
        assert_varies_once(&brotli);

        // The identity variant is cached as immutable too, so it must vary.
        let plain = router
            .clone()
            .oneshot(gzip_request("/app.wasm"))
            .await
            .unwrap();
        assert!(!plain.headers().contains_key(header::CONTENT_ENCODING));
        assert_varies_once(&plain);

        let mut revalidation = request("/app.wasm", "br");
        revalidation.headers_mut().insert(
            header::IF_MODIFIED_SINCE,
            plain.headers()[header::LAST_MODIFIED].clone(),
        );
        let not_modified = router.clone().oneshot(revalidation).await.unwrap();
        assert_eq!(not_modified.status(), axum::http::StatusCode::NOT_MODIFIED);
        assert_varies_once(&not_modified);
        let body = axum::body::to_bytes(plain.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(body.len(), 4096);
    }

    #[tokio::test]
    async fn already_compressed_formats_are_sent_as_is() {
        for content_type in [
            "application/pdf",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            "application/zip",
            "font/woff2",
            "Application/PDF",
        ] {
            let router =
                super::with_compression(Router::new().route(
                    "/file",
                    get(move || async move {
                        ([(header::CONTENT_TYPE, content_type)], vec![b'x'; 4096])
                    }),
                ));
            let response = router.oneshot(gzip_request("/file")).await.unwrap();
            assert!(
                !response.headers().contains_key(header::CONTENT_ENCODING),
                "{content_type} was compressed"
            );
            assert_varies_once(&response);
        }
    }

    #[tokio::test]
    async fn streamed_csv_keeps_streaming_and_failures_still_abort() {
        let (send, receive) = mpsc::unbounded_channel::<Result<String, std::io::Error>>();
        let mut body = Some(Body::from_stream(tokio_stream_from(receive)));
        let export = tower::service_fn(move |_: Request<Body>| {
            let response = Response::builder()
                .header(header::CONTENT_TYPE, "text/csv; charset=utf-8")
                .body(body.take().unwrap());
            async move { response }
        });
        let response = super::compression_layer()
            .layer(export)
            .oneshot(gzip_request("/export.csv"))
            .await
            .unwrap();
        assert_eq!(response.headers()[header::CONTENT_ENCODING], "gzip");
        let mut frames = Body::new(response.into_body()).into_data_stream();

        // Compressed bytes must reach the client while the export is still
        // running, not only after the whole file has been produced.
        let mut seed = 1u64;
        let rows: String = (0..20_000)
            .map(|row| {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                format!("{row},{seed},Project {}\n", seed % 97)
            })
            .collect();
        for chunk in rows.as_bytes().chunks(64 * 1024) {
            send.send(Ok(String::from_utf8(chunk.to_vec()).unwrap()))
                .unwrap();
        }
        let first = tokio::time::timeout(Duration::from_secs(5), frames.next())
            .await
            .expect("compressed bytes before the export finished")
            .unwrap()
            .unwrap();
        assert!(!first.is_empty());

        // A failing export must still interrupt the response rather than end
        // as a well-formed (silently truncated) gzip file.
        send.send(Err(std::io::Error::other("export failed")))
            .unwrap();
        drop(send);
        let mut failed = false;
        while let Some(frame) = frames.next().await {
            if frame.is_err() {
                failed = true;
                break;
            }
        }
        assert!(failed, "the body error must reach the client");
    }

    fn tokio_stream_from<T>(
        receive: mpsc::UnboundedReceiver<T>,
    ) -> impl futures_util::Stream<Item = T> {
        futures_util::stream::unfold(receive, |mut receive| async move {
            receive.recv().await.map(|item| (item, receive))
        })
        .fuse()
    }
}

#[cfg(not(any(feature = "server", feature = "web")))]
fn main() {
    eprintln!("Build with --features server or --features web (or use `dx serve`).");
    std::process::exit(1);
}

#[cfg(feature = "server")]
fn init_tracing(log_level: &str) {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level)),
        )
        .try_init();
}
