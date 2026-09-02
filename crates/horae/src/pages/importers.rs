//! Admin "Import from Harvest" screen: pick a source (Harvest API or a CSV file),
//! run a dry-run, review the per-entity summary and record errors, then commit
//! (contracts/importer-api.md).

use dioxus::prelude::*;
use horae_core::importers::harvest::types::{
    ConnectionStatus, EntityCounts, EntityType, ImportMode, ImportReport, SyncScope,
};

use crate::components::toast::{Toast, ToastContainer};
use crate::server_fns;

/// Which import source the admin is working with. `Picker` is the landing list;
/// choosing a source drills into its flow.
#[derive(Clone, Copy, PartialEq)]
enum Source {
    Picker,
    Api,
    Csv,
}

/// A CSV the admin has selected but not yet imported.
#[derive(Clone, PartialEq)]
struct CsvFile {
    name: String,
    size: u64,
    bytes: Vec<u8>,
}

/// One unit of import work, so the four trigger buttons share a single runner.
#[derive(Clone)]
enum Run {
    Api(ImportMode, SyncScope),
    Csv(ImportMode, Vec<u8>),
}

#[component]
pub fn HarvestImport() -> Element {
    let mut status = use_resource(|| async move { server_fns::harvest_connection_status().await });
    let mut source = use_signal(|| Source::Picker);
    let mut report = use_signal(|| None::<Result<ImportReport, String>>);
    let mut running = use_signal(|| false);
    let mut manage_open = use_signal(|| false);
    let mut csv_file = use_signal(|| None::<CsvFile>);
    let mut toast_msg = use_signal(|| None::<String>);

    // Single runner for every trigger: clears the old report, awaits the import,
    // then publishes the new report and a completion toast.
    let execute = move |job: Run| async move {
        running.set(true);
        report.set(None);
        let (res, mode) = match job {
            Run::Api(mode, sync) => (
                server_fns::import_harvest_api(mode, sync)
                    .await
                    .map_err(|e| e.to_string()),
                mode,
            ),
            Run::Csv(mode, bytes) => (
                server_fns::import_harvest_csv(bytes, mode)
                    .await
                    .map_err(|e| e.to_string()),
                mode,
            ),
        };
        if res.is_ok() {
            toast_msg.set(Some(toast_for(&res, mode)));
        }
        report.set(Some(res));
        running.set(false);
    };

    // Fetch the OAuth authorize URL and hand the browser to Harvest.
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

    let disconnect = move |_: MouseEvent| {
        spawn(async move {
            match server_fns::harvest_disconnect().await {
                Ok(()) => {
                    report.set(None);
                    manage_open.set(false);
                    status.restart();
                }
                Err(e) => report.set(Some(Err(e.to_string()))),
            }
        });
    };

    let on_file = move |e: Event<FormData>| async move {
        if let Some(f) = e.files().into_iter().next() {
            let (name, size) = (f.name(), f.size());
            match f.read_bytes().await {
                Ok(bytes) => {
                    report.set(None);
                    csv_file.set(Some(CsvFile {
                        name,
                        size,
                        bytes: bytes.to_vec(),
                    }));
                }
                Err(err) => report.set(Some(Err(format!("Could not read file: {err}")))),
            }
        }
    };

    let conn = match &*status.read_unchecked() {
        None => Conn::Loading,
        Some(Ok(s)) => Conn::Ready(s.clone()),
        Some(Err(e)) => Conn::Err(e.to_string()),
    };
    let configured = match &conn {
        Conn::Ready(s) => s.configured,
        _ => true,
    };
    let connected = matches!(&conn, Conn::Ready(s) if s.connected);
    let src = source();
    let has_report = report.read().is_some();
    let (title, subtitle) = match src {
        Source::Picker => (
            "Importers",
            "Bring data in from another tracker. Every import is reversible until you commit — start with a dry-run.",
        ),
        Source::Api => ("Import from Harvest", IMPORT_SUB),
        Source::Csv => ("Import from CSV", IMPORT_SUB),
    };

    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "{title}" }
            }
            p { class: "text-secondary text-sm mb-6", "{subtitle}" }

            // Back to the landing list (and a shortcut to switch source), from any
            // source flow.
            if src != Source::Picker {
                div { class: "flex items-center gap-3 flex-wrap mb-6",
                    button {
                        r#type: "button",
                        class: "imp-back text-primary text-sm",
                        onclick: move |_| {
                            source.set(Source::Picker);
                            report.set(None);
                        },
                        "← All importers"
                    }
                    if src == Source::Api {
                        span { class: "text-faint text-sm", "·" }
                        button {
                            r#type: "button",
                            class: "imp-back text-secondary text-sm",
                            onclick: move |_| {
                                source.set(Source::Csv);
                                report.set(None);
                            },
                            "Use a CSV instead"
                        }
                    } else if configured {
                        span { class: "text-faint text-sm", "·" }
                        button {
                            r#type: "button",
                            class: "imp-back text-secondary text-sm",
                            onclick: move |_| {
                                source.set(Source::Api);
                                report.set(None);
                            },
                            "Use the Harvest API instead"
                        }
                    }
                }
            }

            // ── Importer list (landing) ─────────────────────────────────
            if src == Source::Picker {
                div { class: "flex flex-col gap-3",
                    if configured {
                        button {
                            r#type: "button",
                            class: "imp-source",
                            onclick: move |_| {
                                source.set(Source::Api);
                                report.set(None);
                            },
                            span { class: "imp-source-icon imp-icon-harvest", "h" }
                            div { class: "flex-1 min-w-0",
                                div { class: "flex items-center gap-2 flex-wrap",
                                    span { class: "text-sm font-semibold", "Harvest" }
                                    if connected {
                                        span { class: "badge badge-success badge-sm", "Connected" }
                                    }
                                }
                                div { class: "text-faint text-sm", "Read-only sync via the Harvest API." }
                            }
                            span { class: "text-faint", "›" }
                        }
                    } else {
                        div { class: "imp-source imp-source-off",
                            span { class: "imp-source-icon imp-icon-harvest", "h" }
                            div { class: "flex-1 min-w-0",
                                div { class: "flex items-center gap-2 flex-wrap",
                                    span { class: "text-sm font-semibold text-faint", "Harvest" }
                                    span { class: "badge badge-neutral badge-sm", "Unavailable" }
                                }
                                div { class: "text-faint text-sm", "API not configured — use CSV." }
                            }
                        }
                    }

                    button {
                        r#type: "button",
                        class: "imp-source",
                        onclick: move |_| {
                            source.set(Source::Csv);
                            report.set(None);
                        },
                        span { class: "imp-source-icon imp-icon-csv text-mono text-xs", "CSV" }
                        div { class: "flex-1 min-w-0",
                            div { class: "text-sm font-semibold", "CSV file" }
                            div { class: "text-faint text-sm",
                                "Upload a Detailed time report exported from Harvest or any tracker."
                            }
                        }
                        span { class: "text-faint", "›" }
                    }

                    PlannedSource { icon: "t", name: "Toggl Track" }
                    PlannedSource { icon: "c", name: "Clockify" }
                }
            }

            // ── API branch ──────────────────────────────────────────────
            if src == Source::Api {
                match &conn {
                    Conn::Loading => rsx! {
                        p { class: "text-faint text-sm", "Checking connection…" }
                    },
                    Conn::Err(e) => rsx! {
                        div { class: "alert alert-danger", "{e}" }
                    },
                    Conn::Ready(s) if s.connected => rsx! {
                        ConnectionChip {
                            account: s.account_id.clone().unwrap_or_default(),
                            token_expired: s.token_expired,
                            manage_open,
                            ondisconnect: disconnect,
                        }
                        if s.token_expired {
                            div { class: "banner banner-warning mt-4",
                                span { class: "banner-icon", "◔" }
                                div { class: "banner-body",
                                    div { class: "banner-title", "Imports paused while the token refreshes" }
                                    div { class: "banner-detail",
                                        "This usually resolves itself within a minute."
                                    }
                                }
                            }
                        }
                        div { class: "flex items-center gap-4 flex-wrap mt-4",
                            button {
                                r#type: "button",
                                class: "btn btn-primary",
                                disabled: running(),
                                onclick: move |_| execute(Run::Api(ImportMode::DryRun, SyncScope::Full)),
                                "Preview import (dry-run)"
                            }
                            span { class: "text-faint text-sm",
                                "A dry-run previews everything without writing a single row."
                            }
                        }
                        if !has_report && !running() {
                            div { class: "flex flex-col items-center text-center gap-3 p-8 mt-4 bg-secondary border rounded-lg",
                                div { class: "text-sm", "Nothing imported yet" }
                                div { class: "text-faint text-sm max-w-md",
                                    "Your account is linked. A dry-run is free and reversible — start there."
                                }
                            }
                        }
                    },
                    Conn::Ready(_) => rsx! {
                        div { class: "flex flex-col items-center text-center gap-4 p-8 card",
                            span { class: "himp-logo", "h" }
                            div { class: "text-lg font-semibold", "Connect your Harvest account" }
                            div { class: "text-faint text-sm max-w-md",
                                "Read-only access. Horae never writes anything back to Harvest."
                            }
                            button {
                                r#type: "button",
                                class: "btn btn-primary mt-2",
                                onclick: connect,
                                "Connect Harvest"
                            }
                        }
                    },
                }
            }

            // ── CSV branch ──────────────────────────────────────────────
            if src == Source::Csv {
                match csv_file.read().as_ref() {
                    None => rsx! {
                        label { class: "dropzone",
                            input {
                                r#type: "file",
                                accept: ".csv,text/csv",
                                class: "hidden",
                                onchange: on_file,
                            }
                            div { class: "text-2xl", "↥" }
                            div { class: "text-base max-w-md",
                                "Export your Detailed time report from Harvest and choose the .csv here, or "
                                span { class: "text-primary font-semibold", "browse" }
                                "."
                            }
                            div { class: "text-mono text-xs text-faint", "UTF-8 · comma-separated" }
                        }
                    },
                    Some(file) => rsx! {
                        div { class: "file-chip flex-wrap",
                            span { class: "file-chip-icon text-mono text-xs", "CSV" }
                            div { class: "min-w-0",
                                div { class: "truncate", "{file.name}" }
                                div { class: "file-chip-size", "{format_size(file.size)}" }
                            }
                            span { class: "badge badge-success badge-sm", "Ready" }
                            div { class: "flex-1" }
                            label { class: "btn btn-ghost btn-sm",
                                input {
                                    r#type: "file",
                                    accept: ".csv,text/csv",
                                    class: "hidden",
                                    onchange: on_file,
                                }
                                "Replace file"
                            }
                        }
                        div { class: "flex items-center gap-4 flex-wrap mt-4",
                            button {
                                r#type: "button",
                                class: "btn btn-primary",
                                disabled: running(),
                                onclick: {
                                    let bytes = file.bytes.clone();
                                    move |_| execute(Run::Csv(ImportMode::DryRun, bytes.clone()))
                                },
                                "Preview file (dry-run)"
                            }
                            span { class: "text-faint text-sm",
                                "A dry-run previews everything without writing a single row."
                            }
                        }
                    },
                }
            }

            // ── Running ─────────────────────────────────────────────────
            if running() {
                div { class: "card mt-4 flex items-center gap-3",
                    span { class: "himp-spinner" }
                    span { class: "text-sm font-semibold", "Import in progress · nothing is written until you commit" }
                }
            }

            // ── Shared report ───────────────────────────────────────────
            if let Some(result) = report.read().as_ref() {
                match result {
                    Ok(r) => rsx! {
                        ReportView {
                            report: r.clone(),
                            busy: running(),
                            show_resync: src == Source::Api,
                            oncommit: move |_| {
                                let job = match src {
                                    Source::Api => Run::Api(ImportMode::Commit, SyncScope::Full),
                                    Source::Csv => match csv_file.read().as_ref() {
                                        Some(f) => Run::Csv(ImportMode::Commit, f.bytes.clone()),
                                        None => return,
                                    },
                                    // No report exists on the landing list.
                                    Source::Picker => return,
                                };
                                spawn(execute(job));
                            },
                            onresync: move |_| {
                                spawn(execute(Run::Api(ImportMode::Commit, SyncScope::Incremental)));
                            },
                        }
                    },
                    Err(e) => rsx! {
                        div { class: "alert alert-danger mt-4", "{e}" }
                    },
                }
            }
        }

        ToastContainer {
            if let Some(msg) = toast_msg.read().as_ref() {
                Toast {
                    message: "{msg}",
                    variant: "success",
                    icon: "✓",
                    dismissible: true,
                    ondismiss: move |_| toast_msg.set(None),
                }
            }
        }
    }
}

/// Loaded/loading/error view of the connection resource, kept out of the render
/// branches for readability.
enum Conn {
    Loading,
    Err(String),
    Ready(ConnectionStatus),
}

/// Compact one-line "Connected" chip with an expandable management panel.
#[component]
fn ConnectionChip(
    account: String,
    token_expired: bool,
    manage_open: Signal<bool>,
    ondisconnect: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        div { class: "card",
            div { class: "flex items-center gap-3 flex-wrap",
                span { class: "integration-logo harvest", "h" }
                div { class: "integration-body",
                    div { class: "integration-name", "Harvest" }
                    div { class: "integration-meta truncate", "{account} · Read-only" }
                }
                if token_expired {
                    span { class: "badge badge-warning badge-sm", "Token expired" }
                } else {
                    span { class: "badge badge-success badge-sm", "Connected" }
                }
                button {
                    r#type: "button",
                    class: "btn btn-ghost btn-sm",
                    onclick: move |_| manage_open.set(!manage_open()),
                    "Manage connection"
                }
            }
            if manage_open() {
                div { class: "border-t mt-4",
                    div { class: "grid grid-cols-3 gap-4 mt-4",
                        div {
                            div { class: "text-xs uppercase tracking-wide text-faint mb-1", "Account" }
                            div { class: "text-sm truncate", "{account}" }
                        }
                        div {
                            div { class: "text-xs uppercase tracking-wide text-faint mb-1", "Token" }
                            if token_expired {
                                div { class: "text-sm text-warning", "Expired · refreshes automatically" }
                            } else {
                                div { class: "text-sm", "Valid" }
                            }
                        }
                        div {
                            div { class: "text-xs uppercase tracking-wide text-faint mb-1", "Scope" }
                            div { class: "text-sm", "Read-only" }
                        }
                    }
                    div { class: "mt-4",
                        button {
                            r#type: "button",
                            class: "btn btn-danger btn-sm",
                            onclick: move |e| ondisconnect.call(e),
                            "Disconnect"
                        }
                    }
                }
            }
        }
    }
}

/// The dry-run/commit result: a status banner, four entity stat tiles, a
/// collapsible record-error list, and (API only) a re-sync action. Shared by
/// both import sources.
#[component]
fn ReportView(
    report: ImportReport,
    busy: bool,
    show_resync: bool,
    oncommit: EventHandler<MouseEvent>,
    onresync: EventHandler<MouseEvent>,
) -> Element {
    let mut errors_open = use_signal(|| true);
    let error_count = report.row_errors.len();
    let is_dry = report.mode == ImportMode::DryRun;
    let committed = !is_dry;

    rsx! {
        div { class: "flex flex-col gap-4 mt-4",

            // Status banner
            if is_dry {
                div { class: "banner banner-warning",
                    span { class: "banner-icon", "◔" }
                    div { class: "banner-body",
                        div { class: "banner-title", "Preview only — nothing was written" }
                        div { class: "banner-detail", "Review the numbers, then commit." }
                    }
                    div { class: "banner-action",
                        button {
                            r#type: "button",
                            class: "btn btn-primary btn-sm",
                            disabled: busy,
                            onclick: move |e| oncommit.call(e),
                            "Commit this import"
                        }
                    }
                }
            } else if error_count > 0 {
                div { class: "banner banner-danger",
                    span { class: "banner-icon", "▲" }
                    div { class: "banner-body",
                        div { class: "banner-title", "Import complete with {error_count} errors" }
                        div { class: "banner-detail", "The records below could not be applied." }
                    }
                }
            } else {
                div { class: "banner banner-success",
                    span { class: "banner-icon", "✓" }
                    div { class: "banner-body",
                        div { class: "banner-title", "Import complete" }
                        div { class: "banner-detail", "Every record was written." }
                    }
                }
            }

            // Entity stat tiles
            div { class: "himp-tiles",
                for e in EntityType::ALL {
                    StatTile { entity: e, counts: *report.summary.counts(e) }
                }
            }

            // Record errors
            if error_count > 0 {
                div { class: "card p-0 overflow-hidden",
                    button {
                        r#type: "button",
                        class: "flex items-center gap-3 w-full p-4 bg-secondary border-0 cursor-pointer text-left",
                        onclick: move |_| errors_open.set(!errors_open()),
                        span { class: "text-faint text-xs", if errors_open() { "▾" } else { "▸" } }
                        span { class: "text-sm font-semibold text-default", "Record errors" }
                        span { class: "badge badge-danger badge-sm", "{error_count}" }
                    }
                    if errors_open() {
                        div { class: "overflow-x-auto border-t",
                            table { class: "table",
                                thead {
                                    tr {
                                        th { "Source location" }
                                        th { "Entity" }
                                        th { "Reason" }
                                    }
                                }
                                tbody {
                                    for err in report.row_errors.iter().take(ERROR_ROW_LIMIT) {
                                        tr {
                                            td { class: "text-mono text-xs", "{err.source_location}" }
                                            td { "{entity_label(err.entity)}" }
                                            td { "{err.reason}" }
                                        }
                                    }
                                }
                            }
                            if error_count > ERROR_ROW_LIMIT {
                                div { class: "p-4 border-t text-faint text-sm",
                                    "Showing {ERROR_ROW_LIMIT} of {error_count}."
                                }
                            }
                        }
                    }
                }
            }

            // Re-sync (API only, once an import has run)
            if show_resync && committed {
                div { class: "flex items-center gap-3 flex-wrap",
                    button {
                        r#type: "button",
                        class: "btn btn-secondary",
                        disabled: busy,
                        onclick: move |e| onresync.call(e),
                        "Re-sync changes"
                    }
                    span { class: "text-faint text-sm",
                        "Incremental — only what changed since the last sync. Local edits are never overwritten."
                    }
                }
            }
        }
    }
}

/// One entity's summary: a total and the created/updated/skipped/errored split.
#[component]
fn StatTile(entity: EntityType, counts: EntityCounts) -> Element {
    let has_errors = counts.errored > 0;
    let tile_class = if has_errors {
        "counter-tile has-errors"
    } else {
        "counter-tile"
    };
    rsx! {
        div { class: "{tile_class}",
            div { class: "counter-head",
                span { class: "counter-head-name", "{entity_label(entity)}" }
                span { class: "counter-head-total", "{counts.processed()}" }
            }
            div { class: "counter-breakdown",
                StatCell { value: counts.created, label: "Created", tone: "created" }
                StatCell { value: counts.updated, label: "Updated", tone: "" }
                StatCell { value: counts.skipped, label: "Skipped", tone: "" }
                StatCell {
                    value: counts.errored,
                    label: "Errored",
                    tone: if has_errors { "errored" } else { "" },
                }
            }
        }
    }
}

#[component]
fn StatCell(value: u64, label: String, tone: String) -> Element {
    rsx! {
        div { class: "counter-stat {tone}",
            span { class: "counter-stat-value", "{value}" }
            span { class: "counter-stat-label", "{label}" }
        }
    }
}

/// A not-yet-available import source: a dimmed, non-interactive card with a
/// "Planned" pill (design: Importers.dc.html).
#[component]
fn PlannedSource(icon: String, name: String) -> Element {
    rsx! {
        div { class: "imp-source imp-source-off",
            span { class: "imp-source-icon imp-icon-muted", "{icon}" }
            div { class: "flex-1 min-w-0",
                div { class: "flex items-center gap-2 flex-wrap",
                    span { class: "text-sm font-semibold text-faint", "{name}" }
                    span { class: "badge badge-neutral badge-sm", "Planned" }
                }
                div { class: "text-faint text-sm", "Not available yet." }
            }
        }
    }
}

/// The subtitle shared by both source flows once a source is chosen.
const IMPORT_SUB: &str = "Bring your clients, projects, tasks and time entries across. Every import is reversible until you commit — start with a dry-run.";

/// Cap the inline error table; the full set is available in the run record.
const ERROR_ROW_LIMIT: usize = 50;

fn entity_label(e: EntityType) -> &'static str {
    match e {
        EntityType::Client => "Clients",
        EntityType::Project => "Projects",
        EntityType::Task => "Tasks",
        EntityType::TimeEntry => "Time entries",
    }
}

/// The completion toast text for a finished run.
fn toast_for(res: &Result<ImportReport, String>, mode: ImportMode) -> String {
    match res {
        Err(_) => "Import failed".to_string(),
        Ok(r) if !r.row_errors.is_empty() => match mode {
            ImportMode::DryRun => format!("Dry-run finished · {} errors", r.row_errors.len()),
            ImportMode::Commit => format!("Import complete · {} errors", r.row_errors.len()),
        },
        Ok(_) => match mode {
            ImportMode::DryRun => "Dry-run finished · nothing written".to_string(),
            ImportMode::Commit => "Import complete".to_string(),
        },
    }
}

/// Human-readable file size for the CSV chip.
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    match bytes {
        b if b >= MB => format!("{:.1} MB", b as f64 / MB as f64),
        b if b >= KB => format!("{:.1} KB", b as f64 / KB as f64),
        b => format!("{b} B"),
    }
}
