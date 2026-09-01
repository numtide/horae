//! Admin "Import from Harvest" screen: connect the account, run an API or CSV
//! import, and read the summary + per-record error report (contracts/importer-api.md).

use dioxus::prelude::*;
use horae_core::harvest_import::types::{
    EntityCounts, EntityType, ImportMode, ImportReport, SyncScope,
};

use crate::server_fns;

#[component]
pub fn HarvestImport() -> Element {
    let status = use_resource(|| async move { server_fns::harvest_connection_status().await });
    let mut report = use_signal(|| None::<Result<ImportReport, String>>);
    let mut running = use_signal(|| false);

    // Kick off the OAuth connect: fetch the authorize URL and redirect the browser
    // to Harvest's authorization endpoint.
    let connect = move |_| async move {
        match server_fns::harvest_connect_start().await {
            Ok(url) => {
                let js = format!(
                    "window.location.href = {};",
                    serde_json::to_string(&url).unwrap_or_default()
                );
                let _ = document::eval(&js).await;
            }
            Err(e) => report.set(Some(Err(format!("Could not start Harvest connect: {e}")))),
        }
    };

    let run_api = move |mode: ImportMode, sync: SyncScope| {
        move |_| async move {
            running.set(true);
            let result = server_fns::import_harvest_api(mode, sync).await;
            report.set(Some(result.map_err(|e| e.to_string())));
            running.set(false);
        }
    };

    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Import from Harvest" }
            }

            // ── Connection ──────────────────────────────────────────────
            div { class: "card",
                div { class: "p-5",
                    h3 { class: "text-sm mb-4 uppercase tracking-wide text-faint", "Connection" }
                    match &*status.read_unchecked() {
                        Some(Ok(s)) if s.connected => rsx! {
                            p { "Connected to Harvest account ",
                                strong { "{s.account_id.clone().unwrap_or_default()}" }
                                if s.token_expired {
                                    span { class: "text-faint", " (token expired — a sync will refresh it)" }
                                }
                            }
                            button { class: "btn", onclick: connect, "Reconnect" }
                        },
                        Some(Ok(_)) => rsx! {
                            p { "No Harvest account is connected yet." }
                            button { class: "btn btn-primary", onclick: connect, "Connect Harvest" }
                        },
                        Some(Err(e)) => rsx! { div { class: "alert alert-danger", "{e}" } },
                        None => rsx! { p { class: "text-faint", "Checking connection…" } },
                    }
                }
            }

            // ── Run an import ───────────────────────────────────────────
            div { class: "card mt-4",
                div { class: "p-5",
                    h3 { class: "text-sm mb-4 uppercase tracking-wide text-faint", "Run import" }
                    p { class: "text-faint mb-4",
                        "A dry-run previews what would change without writing anything."
                    }
                    div { class: "page-actions",
                        button {
                            class: "btn",
                            disabled: running(),
                            onclick: run_api(ImportMode::DryRun, SyncScope::Full),
                            "Dry-run (full)"
                        }
                        button {
                            class: "btn btn-primary",
                            disabled: running(),
                            onclick: run_api(ImportMode::Commit, SyncScope::Full),
                            "Import (full)"
                        }
                        button {
                            class: "btn",
                            disabled: running(),
                            onclick: run_api(ImportMode::Commit, SyncScope::Incremental),
                            "Re-sync (incremental)"
                        }
                    }
                    if running() {
                        p { class: "text-faint mt-4", "Running…" }
                    }
                }
            }

            // ── Report ──────────────────────────────────────────────────
            if let Some(result) = &*report.read() {
                match result {
                    Ok(r) => rsx! { ReportView { report: r.clone() } },
                    Err(e) => rsx! {
                        div { class: "card mt-4",
                            div { class: "p-5",
                                div { class: "alert alert-danger", "{e}" }
                            }
                        }
                    },
                }
            }
        }
    }
}

#[component]
fn ReportView(report: ImportReport) -> Element {
    let rows = EntityType::ALL.iter().map(|&e| {
        let c: EntityCounts = *report.summary.counts(e);
        (label(e), c)
    });
    let mode_label = match report.mode {
        ImportMode::DryRun => "Dry-run preview (nothing written)",
        ImportMode::Commit => "Import complete",
    };

    rsx! {
        div { class: "card mt-4",
            div { class: "p-5",
                h3 { class: "text-sm mb-4 uppercase tracking-wide text-faint", "{mode_label}" }
                table { class: "table",
                    thead {
                        tr {
                            th { "Entity" }
                            th { "Created" }
                            th { "Updated" }
                            th { "Skipped" }
                            th { "Errored" }
                        }
                    }
                    tbody {
                        for (name, c) in rows {
                            tr {
                                td { "{name}" }
                                td { "{c.created}" }
                                td { "{c.updated}" }
                                td { "{c.skipped}" }
                                td { "{c.errored}" }
                            }
                        }
                    }
                }

                if !report.row_errors.is_empty() {
                    h3 { class: "text-sm mt-4 mb-2 uppercase tracking-wide text-faint",
                        "Record errors ({report.row_errors.len()})"
                    }
                    ul {
                        for err in report.row_errors.iter() {
                            li { "{err.source_location}: {err.reason}" }
                        }
                    }
                }
            }
        }
    }
}

fn label(e: EntityType) -> &'static str {
    match e {
        EntityType::Client => "Clients",
        EntityType::Project => "Projects",
        EntityType::Task => "Tasks",
        EntityType::TimeEntry => "Time entries",
    }
}
