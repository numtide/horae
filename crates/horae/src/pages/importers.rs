//! Admin "Import from Harvest" screen: pick a source (Harvest API or a CSV file),
//! run a dry-run, review the per-entity summary and record errors, then commit
//! (contracts/importer-api.md).

use dioxus::html::FileData;
use dioxus::prelude::*;
use horae_core::importers::harvest::types::{
    ConnectionStatus, EntityCounts, EntityType, ImportMode, ImportReport, SyncScope,
};

use crate::components::toast::{Toast, ToastContainer};
use crate::server_fns;

#[path = "importers/presentation.rs"]
mod presentation;

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
    file: FileData,
}

/// One unit of import work, so the four trigger buttons share a single runner.
#[derive(Clone)]
enum Run {
    Api(ImportMode, SyncScope, Option<i64>),
    Csv(ImportMode, FileData),
}

impl Run {
    fn commit(&self) -> Self {
        match self {
            Self::Api(_, sync, generation) => Self::Api(ImportMode::Commit, *sync, *generation),
            Self::Csv(_, file) => Self::Csv(ImportMode::Commit, file.clone()),
        }
    }
}

#[component]
pub fn HarvestImport() -> Element {
    let mut status = use_resource(|| async move { server_fns::harvest_connection_status().await });
    let mut source = use_signal(|| Source::Picker);
    let mut report = use_signal(|| None::<Result<ImportReport, String>>);
    let mut running = use_signal(|| false);
    let mut active_job = use_signal(|| None::<uuid::Uuid>);
    let mut job_progress = use_signal(|| None::<crate::models::JobStatus>);
    let manage_open = use_signal(|| false);
    let mut csv_file = use_signal(|| None::<CsvFile>);
    let mut toast_msg = use_signal(|| None::<String>);
    let mut history_pages = use_signal(|| vec![None::<uuid::Uuid>]);
    let history_cursor = use_memo(move || history_pages.read().last().copied().flatten());
    let mut history = use_resource(move || {
        let before = history_cursor();
        async move {
            (
                before,
                server_fns::list_harvest_import_jobs(before, None).await,
            )
        }
    });
    let mut restore_history = use_signal(|| true);
    let mut poll_version = use_signal(|| 0_u64);
    let mut poll_error = use_signal(|| None::<(uuid::Uuid, String)>);
    let mut action_error = use_signal(|| None::<String>);
    let mut connection_error = use_signal(|| None::<(&'static str, String)>);
    let mut action_pending = use_signal(|| false);
    let mut run_origin = use_signal(|| None::<(uuid::Uuid, Run)>);

    let mut watch_job = move |id| {
        restore_history.set(false);
        poll_version += 1;
        poll_error.set(None);
        action_error.set(None);
        report.set(None);
        toast_msg.set(None);
        job_progress.set(None);
        running.set(true);
        active_job.set(Some(id));
    };

    use_effect(move || {
        let loaded = history.read();
        if !*restore_history.peek() {
            return;
        }
        if let Some((None, Ok(jobs))) = loaded.as_ref() {
            restore_history.set(false);
            if let Some(job) = jobs.iter().find(|job| job.is_active()) {
                watch_job(job.id);
            }
        }
    });

    let job_status = use_resource(move || {
        let id = active_job();
        let version = poll_version();
        async move {
            let id = id?;
            loop {
                let current = server_fns::get_harvest_import_job(id)
                    .await
                    .map_err(|error| error.to_string())
                    .and_then(|job| {
                        job.ok_or_else(|| "Import job not found or no longer available".to_string())
                    });
                if *active_job.peek() != Some(id) || *poll_version.peek() != version {
                    return None;
                }
                match current {
                    Ok(job) if job.is_active() => {
                        job_progress.set(Some(job));
                        #[cfg(feature = "web")]
                        gloo_timers::future::TimeoutFuture::new(1_000).await;
                        #[cfg(feature = "server")]
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    }
                    terminal => return Some((id, version, terminal)),
                }
            }
        }
    });

    use_effect(move || {
        let snapshot = job_status.read();
        let Some(Some((id, version, result))) = snapshot.as_ref() else {
            return;
        };
        // A resource retains its previous result while a new request is pending.
        if *active_job.peek() != Some(*id) || *poll_version.peek() != *version {
            return;
        }
        running.set(false);
        active_job.set(None);
        match result {
            Err(error) => poll_error.set(Some((*id, error.clone()))),
            Ok(job) => {
                job_progress.set(Some(job.clone()));
                if job.status == "succeeded" || job.report.is_some() {
                    let decoded = job
                        .report
                        .clone()
                        .ok_or_else(|| "Import completed without a report".to_string())
                        .and_then(|value| {
                            serde_json::from_value::<ImportReport>(value).map_err(|e| e.to_string())
                        });
                    if job.status == "succeeded"
                        && let Ok(import) = &decoded
                    {
                        toast_msg.set(Some(toast_for(&decoded, import.mode)));
                    }
                    report.set(Some(decoded));
                } else {
                    report.set(Some(Err(job.last_error.clone().unwrap_or_else(|| {
                        if job.status == "cancelled" {
                            "Import cancelled".into()
                        } else {
                            "Import failed".into()
                        }
                    }))));
                }
                history.restart();
            }
        }
    });

    // Keep the submitted source with its preview; the picker may change later.
    let execute = move |job: Run| async move {
        if *running.peek() || *action_pending.peek() {
            return;
        }
        restore_history.set(false);
        action_pending.set(true);
        running.set(true);
        active_job.set(None);
        job_progress.set(None);
        poll_error.set(None);
        action_error.set(None);
        report.set(None);
        run_origin.set(None);
        let mut submitted = job.clone();
        let result = match job {
            Run::Api(mode, sync, generation) => {
                let generation = generation.or_else(|| {
                    status
                        .read()
                        .as_ref()
                        .and_then(|result| result.as_ref().ok())
                        .map(|status| status.account_generation)
                });
                // A confirmation must retain the preview's account even if another
                // tab changes the connection before the server validates it.
                submitted = Run::Api(mode, sync, generation);
                server_fns::start_harvest_api_import(mode, sync, generation).await
            }
            Run::Csv(mode, file) => server_fns::start_harvest_csv_import(mode, file.into()).await,
        };
        action_pending.set(false);
        match result {
            Ok(job) => {
                run_origin.set(Some((job.id, submitted)));
                watch_job(job.id);
                job_progress.set(Some(job));
                history_pages.set(vec![None]);
                history.restart();
            }
            Err(error) => {
                report.set(Some(Err(error.to_string())));
                running.set(false);
            }
        }
    };

    let retry_job = move |id| async move {
        if *running.peek() || *action_pending.peek() {
            return;
        }
        action_pending.set(true);
        action_error.set(None);
        match server_fns::retry_harvest_import_job(id).await {
            Ok(job) => {
                run_origin.set(None);
                watch_job(job.id);
                job_progress.set(Some(job));
                history.restart();
            }
            Err(error) => {
                action_error.set(Some(format!("Could not retry import: {error}. History has been refreshed; check the reason before trying again.")));
                history.restart();
            }
        }
        action_pending.set(false);
    };

    // Fetch the OAuth authorize URL and hand the browser to Harvest.
    let connect = move |_| async move {
        if *action_pending.peek() {
            return;
        }
        action_pending.set(true);
        connection_error.set(None);
        match server_fns::harvest_connect_start().await {
            Ok(url) => {
                let js = format!(
                    "window.location.href = {};",
                    serde_json::to_string(&url).unwrap_or_default()
                );
                let _ = document::eval(&js).await;
            }
            Err(e) => connection_error.set(Some((
                "Could not start Harvest connection. Try connecting again.",
                e.to_string(),
            ))),
        }
        action_pending.set(false);
    };

    let disconnect = move |_: MouseEvent| {
        spawn(async move {
            if *action_pending.peek() {
                return;
            }
            action_pending.set(true);
            connection_error.set(None);
            match server_fns::harvest_disconnect().await {
                Ok(()) => {
                    report.set(None);
                    run_origin.set(None);
                    status.restart();
                }
                Err(e) => connection_error.set(Some((
                    "Could not disconnect Harvest. Check the connection before trying again.",
                    e.to_string(),
                ))),
            }
            action_pending.set(false);
        });
    };

    let on_file = move |e: Event<FormData>| async move {
        if let Some(f) = e.files().into_iter().next() {
            let (name, size) = (f.name(), f.size());
            report.set(None);
            csv_file.set(Some(CsvFile {
                name,
                size,
                file: f,
            }));
        }
    };

    let conn = match (&*status.state().read(), &*status.read_unchecked()) {
        (UseResourceState::Pending, _) => Conn::Loading,
        (_, None) => Conn::Loading,
        (_, Some(Ok(s))) => Conn::Ready(s.clone()),
        (_, Some(Err(e))) => Conn::Err(e.to_string()),
    };
    let configured = match &conn {
        Conn::Ready(s) => s.configured,
        _ => true,
    };
    let src = source();
    let history_loading = !matches!(*history.state().read(), UseResourceState::Ready);
    let has_report = report.read().is_some();
    let can_commit = run_origin.read().as_ref().is_some_and(|(id, origin)| {
        let same_account = match origin {
            Run::Csv(..) => true,
            Run::Api(_, _, generation) => matches!(&conn, Conn::Ready(s)
                if s.configured && s.connected && Some(s.account_generation) == *generation),
        };
        same_account
            && job_progress
                .read()
                .as_ref()
                .is_some_and(|job| job.id == *id && job.status == "succeeded")
    });
    let show_resync = can_commit && matches!(run_origin.read().as_ref(), Some((_, Run::Api(..))));
    let current_preview = can_commit
        && report.read().as_ref().is_some_and(|result| {
            result
                .as_ref()
                .is_ok_and(|report| report.mode == ImportMode::DryRun)
        });
    let (title, subtitle) = match src {
        Source::Picker => (
            "Importers",
            "Bring data in from another tracker. Start with a preview before importing business data.",
        ),
        Source::Api => ("Import from Harvest", IMPORT_SUB),
        Source::Csv => ("Import from CSV", IMPORT_SUB),
    };

    rsx! {
        div { class: "himp-page min-w-0",
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
                        "data-testid": "all-importers".to_owned(),
                        onclick: move |_| {
                            source.set(Source::Picker);
                            report.set(None);
                            run_origin.set(None);
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
                                run_origin.set(None);
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
                                run_origin.set(None);
                            },
                            "Use the Harvest API instead"
                        }
                    }
                }
            }

            // ── Importer list (landing) ─────────────────────────────────
            if src == Source::Picker {
                div { class: "flex flex-col gap-3",
                        button {
                            r#type: "button",
                            class: "imp-source",
                            "data-testid": "choose-api".to_owned(),
                            onclick: move |_| {
                                source.set(Source::Api);
                                report.set(None);
                                run_origin.set(None);
                            },
                            span { class: "imp-source-icon imp-icon-harvest", "h" }
                            div { class: "flex-1 min-w-0",
                                div { class: "flex items-center gap-2 flex-wrap",
                                    span { class: "text-sm font-semibold", "Harvest" }
                                    span { class: "badge badge-neutral badge-sm", "{conn.label()}" }
                                }
                                div { class: "text-faint text-sm", "Read-only sync via the Harvest API." }
                            }
                            span { class: "text-faint", "›" }
                        }

                    button {
                        r#type: "button",
                        class: "imp-source",
                        "data-testid": "choose-csv".to_owned(),
                        onclick: move |_| {
                            source.set(Source::Csv);
                            report.set(None);
                            run_origin.set(None);
                        },
                        span { class: "imp-source-icon imp-icon-csv text-mono text-xs", "CSV" }
                        div { class: "flex-1 min-w-0",
                            div { class: "text-sm font-semibold", "CSV file" }
                            div { class: "text-faint text-sm",
                                "Upload a supported Detailed time report exported from Harvest."
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
                ConnectionChip {
                    connection: conn.clone(), manage_open, busy: action_pending,
                    onconnect: connect, ondisconnect: disconnect,
                    onrefresh: move |_| { connection_error.set(None); status.restart(); },
                    on_changed: move |_| { status.restart(); run_origin.set(None); },
                }
                if let Some((message, detail)) = connection_error.read().as_ref() {
                    div { class: "alert alert-danger mt-4", role: "alert",
                        p { "{message}" }
                        details { summary { "Details" } p { "{detail}" } }
                    }
                }
                match &conn {
                    Conn::Ready(s) if s.configured && s.connected => rsx! {
                        div { class: "flex items-center gap-4 flex-wrap mt-4",
                            button {
                                r#type: "button",
                                class: if current_preview { "btn btn-secondary" } else { "btn btn-primary" },
                                "data-testid": "preview-api".to_owned(),
                                disabled: running() || action_pending(),
                                onclick: move |_| execute(Run::Api(ImportMode::DryRun, SyncScope::Full, None)),
                                "Preview import (dry-run)"
                            }
                            span { class: "text-faint text-sm",
                                "Preview leaves business data unchanged. Its job and report are retained."
                            }
                        }
                        if !has_report && !running() {
                            div { class: "flex flex-col items-center text-center gap-3 p-8 mt-4 bg-secondary border rounded-lg",
                                div { class: "text-sm", "{presentation::empty_selection()}" }
                                div { class: "text-faint text-sm max-w-md",
                                    "Start a preview, or select an existing import from history below."
                                }
                            }
                        }
                    },
                    _ => rsx! {},
                }
            }

            // ── CSV branch ──────────────────────────────────────────────
            if src == Source::Csv {
                match csv_file.read().as_ref() {
                    None => rsx! {
                        label { class: "dropzone himp-file-picker relative",
                            input {
                                r#type: "file",
                                accept: ".csv,text/csv",
                                class: "himp-file-input absolute w-full h-full cursor-pointer",
                                aria_label: "Choose Harvest CSV file",
                                "data-testid": "csv-file".to_owned(),
                                onchange: on_file,
                            }
                            div { class: "text-2xl", "↥" }
                            div { class: "text-base max-w-md",
                                "Export your Detailed time report from Harvest and choose the .csv here, or "
                                span { class: "text-primary font-semibold", "browse" }
                                "."
                            }
                            div { class: "text-xs text-faint", "Harvest Detailed time report · CSV" }
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
                            label { class: "btn btn-ghost btn-sm himp-file-picker relative",
                                input {
                                    r#type: "file",
                                    accept: ".csv,text/csv",
                                    class: "himp-file-input absolute w-full h-full cursor-pointer",
                                    aria_label: "Replace Harvest CSV file",
                                    "data-testid": "csv-file".to_owned(),
                                    onchange: on_file,
                                }
                                "Replace file"
                            }
                        }
                        div { class: "flex items-center gap-4 flex-wrap mt-4",
                            button {
                                r#type: "button",
                                class: if current_preview { "btn btn-secondary" } else { "btn btn-primary" },
                                disabled: running() || action_pending(),
                                onclick: {
                                    let selected = file.file.clone();
                                    move |_| execute(Run::Csv(ImportMode::DryRun, selected.clone()))
                                },
                                "data-testid": "preview-csv".to_owned(),
                                "Preview file (dry-run)"
                            }
                            span { class: "text-faint text-sm",
                                "Preview leaves business data unchanged. Its job and report are retained."
                            }
                        }
                    },
                }
            }

            // ── Running ─────────────────────────────────────────────────
            if running() {
                div { class: "card mt-4 flex items-center gap-3 flex-wrap",
                    span { class: "himp-spinner" }
                    div { class: "text-sm font-semibold",
                        div { "Following import" }
                        if let Some(progress) = job_progress.read().as_ref() {
                            div { role: "status", aria_live: "polite", aria_atomic: "true",
                                "{presentation::state_label(&progress.status)} · {presentation::phase_label(progress.phase.as_deref())}"
                                if progress.status == "queued" { p { "Waiting to start." } }
                                if progress.phase.as_deref() == Some("cancelling") { p { "Cancellation requested; waiting for the current batch to finish." } }
                            }
                            div { class: "text-xs text-faint",
                                "{progress.processed_count} processed"
                                if let Some(total) = progress.total_count { " of {total}" }
                            }
                            if let Some(error) = &progress.last_error {
                                div { class: "text-xs text-warning", "Last attempt: {error}" }
                            }
                        }
                        p { class: "text-xs text-faint", "Leaving this page does not stop the import." }
                    }
                    if let Some(job_id) = active_job() {
                        button {
                            r#type: "button",
                            class: "btn btn-ghost btn-sm ml-auto",
                            "data-testid": "cancel-import".to_owned(),
                            disabled: action_pending() || job_progress.read().as_ref().is_some_and(|job| job.phase.as_deref() == Some("cancelling")),
                            onclick: move |_| async move {
                                if *action_pending.peek() { return; }
                                action_pending.set(true);
                                action_error.set(None);
                                match server_fns::cancel_harvest_import_job(job_id).await {
                                    Ok(job) => {
                                        if *active_job.peek() == Some(job_id) {
                                            job_progress.set(Some(job));
                                        }
                                        history.restart();
                                    }
                                    Err(error) => action_error.set(Some(format!("Could not cancel import: {error}"))),
                                }
                                action_pending.set(false);
                            },
                            "Cancel"
                        }
                    }
                }
            }

            if let Some((id, error)) = poll_error.read().as_ref() {
                div { class: "alert alert-danger mt-4", role: "alert",
                    p { "Status unavailable. The import may still be running. Resume monitoring to check the same import." }
                    details { summary { "Details" } p { "{error}" } }
                    button {
                        r#type: "button", class: "btn btn-secondary btn-sm",
                        "data-testid": "resume-monitoring".to_owned(),
                        disabled: action_pending(),
                        onclick: { let id = *id; move |_| watch_job(id) },
                        "Resume monitoring"
                    }
                }
            }
            if let Some(error) = action_error.read().as_ref() {
                div { class: "alert alert-danger mt-4", role: "alert", "{error}" }
            }

            // ── Shared report ───────────────────────────────────────────
            if has_report && let Some(job) = job_progress.read().as_ref()
                && presentation::partial_report(&job.status) && job.report.is_some()
            {
                div { class: "alert alert-danger mt-4", role: "alert",
                    p { if job.status == "cancelled" { "Import cancelled" } else { "Import failed" } }
                    if let Some(error) = &job.last_error { p { "{error}" } }
                    p { "{job.processed_count} processed in confirmed batches" }
                }
            }
            if let Some(result) = report.read().as_ref() {
                match result {
                    Ok(r) => rsx! {
                        if let Some((_, Run::Csv(_, file))) = run_origin.read().as_ref() {
                            p { class: "text-xs text-faint mt-4", "Report source: {file.name()}" }
                        }
                        ReportView {
                            report: r.clone(),
                            job_id: job_progress.read().as_ref().map(|job| job.id),
                            busy: running() || action_pending(),
                            can_commit,
                            show_resync,
                            partial: job_progress.read().as_ref().is_some_and(|job| presentation::partial_report(&job.status)),
                            oncommit: move |_| {
                                let job = run_origin.read().as_ref().map(|(_, job)| job.commit());
                                if let Some(job) = job { spawn(execute(job)); }
                            },
                            onresync: move |_| {
                                spawn(execute(Run::Api(ImportMode::Commit, SyncScope::Incremental, None)));
                            },
                        }
                    },
                    Err(e) => rsx! {
                        div { class: "alert alert-danger mt-4", role: "alert",
                            p { "Could not show a complete import report. Check history and the details before starting another import." }
                            details { summary { "Details" } p { "{e}" } }
                        }
                    },
                }
            }

            section { class: "card mt-6", aria_label: "Import history",
                div { class: "flex items-center justify-between gap-3",
                    h2 { class: "text-sm font-semibold", "Import history" }
                    button {
                        r#type: "button", class: "btn btn-ghost btn-sm",
                        "data-testid": "refresh-history".to_owned(),
                        onclick: move |_| {
                            if history_cursor().is_none() { history.restart(); }
                            else { history_pages.set(vec![None]); }
                        },
                        "Refresh history"
                    }
                }
                p { class: "text-xs text-faint", "Select an import to view its report or follow its progress." }
                if history_pages.read().len() > 1 {
                    button {
                        r#type: "button", class: "btn btn-ghost btn-sm",
                        "data-testid": "newer-history".to_owned(),
                        disabled: history_loading,
                        onclick: move |_| { history_pages.write().pop(); },
                        "Newer imports"
                    }
                }
                match &*history.read() {
                    None => rsx! { p { "Loading import history…" } },
                    Some((key, _)) if *key != history_cursor() || history_loading => rsx! { p { "Loading import history…" } },
                    Some((_, Err(error))) => rsx! { p { role: "alert", "Could not load import history: {error}" } },
                    Some((_, Ok(jobs))) if jobs.is_empty() => rsx! { p {
                        if history_cursor().is_none() { "No retained imports. Start a preview to create a new report." }
                        else { "No imports on this page." }
                    } },
                    Some((_, Ok(jobs))) => rsx! {
                        ul { class: "flex flex-col gap-3 mt-4",
                            for job in jobs {
                                li { key: "{job.id}", class: "flex items-center gap-3 flex-wrap",
                                    button {
                                        r#type: "button", class: "btn btn-ghost btn-sm flex-wrap text-left himp-history",
                                        "data-testid": "select-{job.id}",
                                        aria_pressed: job_progress.read().as_ref().is_some_and(|selected| selected.id == job.id).to_string(),
                                        disabled: action_pending(),
                                        onclick: { let id = job.id; move |_| {
                                            run_origin.set(None);
                                            watch_job(id);
                                        } },
                                        span { "{presentation::source_label(&job.kind)} · {presentation::mode_label(job.report.as_ref())}" }
                                        time { datetime: job.created_at.to_rfc3339(), class: "text-xs text-faint", {job.created_at.format("%d %b %Y, %H:%M UTC").to_string()} }
                                        span { class: "badge badge-neutral badge-sm", "{presentation::state_label(&job.status)}" }
                                    }
                                    if job.can_retry() {
                                        if job.retry_availability == crate::models::RetryAvailability::Available { button {
                                            r#type: "button", class: "btn btn-secondary btn-sm",
                                            "data-testid": "retry-{job.id}",
                                            disabled: action_pending() || running(),
                                            onclick: { let id = job.id; move |_| retry_job(id) },
                                            "Retry import"
                                        } } else { p { class: "text-xs text-faint", "{presentation::retry_reason(job.retry_availability)}" } }
                                    }
                                }
                            }
                        }
                        if jobs.len() == 20 {
                            if let Some(last) = jobs.last() {
                                button {
                                    r#type: "button", class: "btn btn-ghost btn-sm",
                                    "data-testid": "older-history".to_owned(),
                                    disabled: history_loading,
                                    onclick: { let id = last.id; move |_| history_pages.write().push(Some(id)) },
                                    "Older imports"
                                }
                            }
                        }
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
#[derive(Clone, PartialEq)]
enum Conn {
    Loading,
    Err(String),
    Ready(ConnectionStatus),
}

impl Conn {
    fn label(&self) -> &'static str {
        match self {
            Self::Loading => "Checking connection…",
            Self::Err(_) => "Status unavailable",
            Self::Ready(s) if !s.configured => "Not configured",
            Self::Ready(s) if s.connected && s.token_expired => "Token expired",
            Self::Ready(s) if s.connected => "Connected",
            Self::Ready(s) if s.account_id.is_some() => "Disconnected",
            Self::Ready(_) => "Not connected",
        }
    }
}

/// Compact one-line "Connected" chip with an expandable management panel.
#[component]
fn ConnectionChip(
    connection: Conn,
    manage_open: Signal<bool>,
    busy: Signal<bool>,
    onconnect: EventHandler<MouseEvent>,
    ondisconnect: EventHandler<MouseEvent>,
    onrefresh: EventHandler<MouseEvent>,
    on_changed: EventHandler<()>,
) -> Element {
    let loaded = match &connection {
        Conn::Ready(s) => Some(s),
        _ => None,
    };
    let account = loaded.and_then(|s| s.account_id.as_deref());
    let connected = loaded.is_some_and(|s| s.connected);
    let expired = loaded.is_some_and(|s| s.connected && s.token_expired);
    let tone = if expired {
        "badge-warning"
    } else if connected {
        "badge-success"
    } else {
        "badge-neutral"
    };
    rsx! {
        div { class: "card",
            div { class: "flex items-center gap-3 flex-wrap",
                span { class: "integration-logo harvest", "h" }
                div { class: "integration-body",
                    div { class: "integration-name", "Harvest" }
                    div { class: "integration-meta",
                        if let Some(account) = account { "Account {account} · " }
                        "Read-only access to Harvest"
                    }
                }
                span { class: "badge {tone} badge-sm", "{connection.label()}" }
                if account.is_some() { button {
                    r#type: "button",
                    class: "btn btn-ghost btn-sm",
                    "data-testid": "manage-connection".to_owned(),
                    aria_expanded: manage_open().to_string(),
                    aria_controls: "harvest-connection-management",
                    disabled: busy(),
                    onclick: move |_| manage_open.set(!manage_open()),
                    "Manage connection"
                } }
            }
            match &connection {
                Conn::Loading => rsx! { p { class: "text-sm text-secondary mt-4", role: "status", "Checking connection… CSV import remains available." } },
                Conn::Err(error) => rsx! {
                    div { class: "alert alert-danger mt-4", role: "alert",
                        p { "Could not check Harvest connection. Try checking again." }
                        details { summary { "Details" } p { "{error}" } }
                    }
                },
                Conn::Ready(s) if !s.configured => rsx! {
                    p { class: "text-sm text-secondary mt-4", "Deployment setup required. Ask the deployment administrator to configure Harvest OAuth, or use CSV import." }
                },
                Conn::Ready(s) => rsx! {
                    if !s.connected || s.token_expired {
                        p { class: "text-sm text-secondary mt-4",
                            if s.token_expired { "The stored token has expired. Reconnect the original account, or check the connection again after a refresh." }
                            else if account.is_some() { "Disconnected. Imported data and the original account binding are retained." }
                            else { "Connect your Harvest account. Horae never writes anything back to Harvest." }
                        }
                        button { r#type: "button", class: if s.connected { "btn btn-secondary mt-4" } else { "btn btn-primary mt-4" }, "data-testid": "connect-harvest".to_owned(),
                            disabled: busy(), onclick: move |e| onconnect.call(e),
                            if busy() { "Connecting…" } else if account.is_some() { "Reconnect original account" } else { "Connect Harvest" }
                        }
                    }
                },
            }
            if !matches!(connection, Conn::Loading) {
                button { r#type: "button", class: "btn btn-ghost btn-sm mt-4", "data-testid": "refresh-connection".to_owned(),
                    disabled: busy(), onclick: move |e| onrefresh.call(e), "Check connection again"
                }
            }
            div { id: "harvest-connection-management",
                if manage_open() && account.is_some() {
                    div { class: "border-t mt-4 pt-4",
                        p { class: "text-sm text-faint", "Disconnect removes credentials but retains the original account binding and imported data." }
                        if connected { button {
                            r#type: "button",
                            class: "btn btn-danger btn-sm",
                            "data-testid": "disconnect-harvest".to_owned(),
                            disabled: busy(),
                            onclick: move |e| ondisconnect.call(e),
                            if busy() { "Disconnecting…" } else { "Disconnect" }
                        } }
                    }
                }
                ChangeAccount {
                    connection: loaded.cloned().unwrap_or_default(),
                    visible: manage_open(), busy,
                    on_changed,
                }
            }
        }
    }
}

#[component]
fn ChangeAccount(
    connection: ConnectionStatus,
    visible: bool,
    mut busy: Signal<bool>,
    on_changed: EventHandler<()>,
) -> Element {
    let mut inspected = use_signal(|| None::<ConnectionStatus>);
    let mut error = use_signal(|| None::<String>);
    let blocker = connection.change_account_blocker();
    let bound = connection.account_id.is_some();
    let confirm = move |_| async move {
        if busy() {
            return;
        }
        let Some(snapshot) = inspected.peek().clone() else {
            return;
        };
        let Some(account) = snapshot.account_id else {
            return;
        };
        busy.set(true);
        error.set(None);
        match server_fns::harvest_change_account(
            account,
            snapshot.account_generation,
            snapshot.connection_revision,
        )
        .await
        {
            Err(e) => {
                error.set(Some(e.to_string()));
                busy.set(false);
            }
            Ok(()) => {
                inspected.set(None);
                on_changed.call(());
                match server_fns::harvest_connect_start().await {
                    Ok(url) => {
                        let _ = document::eval(&format!(
                            "window.location.href = {};",
                            serde_json::json!(url)
                        ))
                        .await;
                    }
                    Err(e) => error.set(Some(format!(
                        "Account released. Use Connect Harvest to try authorization again: {e}"
                    ))),
                }
                busy.set(false);
            }
        }
    };
    rsx! {
        if bound && visible {
            div { class: "mt-4",
                button { r#type: "button", class: "btn btn-secondary btn-sm", "data-testid": "change-account".to_owned(),
                    disabled: busy() || blocker.is_some(),
                    onclick: move |_| { error.set(None); inspected.set(Some(connection.clone())); },
                    "Change account"
                }
                if let Some(reason) = blocker { p { class: "text-sm text-faint", "{reason}" } }
            }
        }
        if inspected().is_none() {
            if let Some(message) = error() { div { role: "alert", class: "alert alert-danger", "{message}" } }
        }
        crate::components::modal::Modal {
            id: "harvest-change-account", labelledby: "harvest-change-account-title", open: inspected().is_some(), busy: busy(),
            on_dismiss: move |_| { if !busy() { inspected.set(None); } },
            h2 { id: "harvest-change-account-title", "Change Harvest account?" }
            if let Some(snapshot) = inspected() {
                p { "Disconnect and release account {snapshot.account_id.clone().unwrap_or_default()} before authorizing a different account." }
                p { "Business data and retained reports are kept. Old import attempts cannot be retried against the replacement account. Cancelling authorization afterward leaves Horae disconnected." }
                if let Some(message) = error() { p { role: "alert", "{message}" } }
                div { class: "flex gap-3 flex-wrap mt-4",
                    button { r#type: "button", class: "btn btn-secondary", "data-testid": "cancel-change-account".to_owned(), disabled: busy(),
                        onclick: move |_| { inspected.set(None); error.set(None); }, "Cancel" }
                    button { r#type: "button", class: "btn btn-danger", "data-testid": "confirm-change-account".to_owned(), disabled: busy(), onclick: confirm,
                        if busy() { "Changing account…" } else { "Change account and connect" }
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
    job_id: Option<uuid::Uuid>,
    busy: bool,
    can_commit: bool,
    show_resync: bool,
    partial: bool,
    oncommit: EventHandler<MouseEvent>,
    onresync: EventHandler<MouseEvent>,
) -> Element {
    let mut errors_open = use_signal(|| true);
    let error_count = report.error_count();
    let shown_errors = report.row_errors.len().min(ERROR_ROW_LIMIT);
    let is_dry = report.mode == ImportMode::DryRun;
    let committed = !is_dry;

    rsx! {
        div { class: "flex flex-col gap-4 mt-4",

            // Status banner
            if partial {
                div { class: "banner banner-warning",
                    span { class: "banner-icon", "◔" }
                    div { class: "banner-body",
                        div { class: "banner-title", "Partial report — confirmed batches only" }
                        div { class: "banner-detail",
                            if is_dry { "Preview stopped before completion. No business data was written. Its job and partial report are retained." }
                            else { "Only confirmed work is included. Unconfirmed work was rolled back." }
                        }
                    }
                }
            } else if is_dry {
                div { class: "banner banner-warning",
                    span { class: "banner-icon", "◔" }
                    div { class: "banner-body",
                        div { class: "banner-title", "Preview complete — no business data changed" }
                        div { class: "banner-detail",
                            "No business data was written. The job and report are retained. "
                            if can_commit { "Review the numbers, then commit." }
                            else { "Historical preview. Start a new preview to commit this import." }
                        }
                    }
                    if can_commit { div { class: "banner-action",
                        button {
                            r#type: "button",
                            class: "btn btn-primary btn-sm",
                            "data-testid": "commit-preview".to_owned(),
                            disabled: busy,
                            onclick: move |e| oncommit.call(e),
                            "Commit this import"
                        }
                    } }
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
                        div { class: "banner-detail", "Review the created, updated and skipped counts below." }
                    }
                }
            }

            p { class: "text-sm text-secondary", "{presentation::result_summary(&report)}" }

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
                        "data-testid": "toggle-import-errors".to_owned(),
                        aria_expanded: errors_open().to_string(),
                        aria_controls: "import-record-errors",
                        onclick: move |_| errors_open.set(!errors_open()),
                        span { class: "text-faint text-xs", if errors_open() { "▾" } else { "▸" } }
                        span { class: "text-sm font-semibold text-default", "Record errors" }
                        span { class: "badge badge-danger badge-sm", "{error_count}" }
                    }
                    div { id: "import-record-errors", hidden: !errors_open(),
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
                            if error_count > shown_errors as u64 {
                                div { class: "p-4 border-t text-faint text-sm",
                                    "Showing {shown_errors} inline errors of {error_count}."
                                }
                            }
                            if let Some(job_id) = job_id {
                                div { class: "p-4 border-t",
                                    a { class: "btn btn-secondary btn-sm",
                                        href: format!("/api/import/harvest/jobs/{job_id}/errors"),
                                        "Download all errors (NDJSON)"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Re-sync (API only, once an import has run)
            if show_resync && committed && !partial {
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
const IMPORT_SUB: &str = "Bring your clients, projects, tasks and time entries across. Preview first; confirm only after reviewing the results.";

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
        Ok(r) if r.error_count() > 0 => match mode {
            ImportMode::DryRun => format!("Dry-run finished · {} errors", r.error_count()),
            ImportMode::Commit => format!("Import complete · {} errors", r.error_count()),
        },
        Ok(_) => match mode {
            ImportMode::DryRun => "Preview complete · no business data changed".to_string(),
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
