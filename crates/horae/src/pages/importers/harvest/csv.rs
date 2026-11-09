use dioxus::html::FileData;
use dioxus::prelude::*;
use horae_core::importers::harvest::types::ImportMode;

use super::IMPORT_SUB;
use crate::route::Route;
use crate::server_fns;

/// Choose a Harvest CSV export and preview it. The preview's run page commits
/// the retained upload, so the file is chosen only once.
#[component]
pub fn HarvestCsv() -> Element {
    let navigator = use_navigator();
    let mut file = use_signal(|| None::<FileData>);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    let on_file = move |e: Event<FormData>| {
        if let Some(chosen) = e.files().into_iter().next() {
            error.set(None);
            file.set(Some(chosen));
        }
    };
    let preview = move |_| async move {
        let Some(chosen) = file.peek().clone() else {
            return;
        };
        if *busy.peek() {
            return;
        }
        busy.set(true);
        error.set(None);
        match server_fns::start_harvest_csv_import(ImportMode::DryRun, chosen.into()).await {
            Ok(job) => {
                navigator.push(Route::HarvestRun { id: job.id });
            }
            Err(e) => error.set(Some(e.to_string())),
        }
        busy.set(false);
    };

    rsx! {
        div { class: "himp-page min-w-0",
            div { class: "page-header",
                h1 { class: "page-title", "Import from CSV" }
            }
            p { class: "text-secondary text-sm mb-6", "{IMPORT_SUB}" }
            div { class: "flex items-center gap-3 flex-wrap mb-6",
                Link { to: Route::ImporterIndex {}, class: "imp-back text-primary text-sm", "data-testid": "all-importers", "← All importers" }
                span { class: "text-faint text-sm", "·" }
                Link { to: Route::HarvestOverview { outcome: Default::default() }, class: "imp-back text-secondary text-sm", "data-testid": "use-api", "Use the Harvest API instead" }
            }
            match file.read().as_ref() {
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
                Some(chosen) => rsx! {
                    div { class: "file-chip flex-wrap",
                        span { class: "file-chip-icon text-mono text-xs", "CSV" }
                        div { class: "min-w-0",
                            div { class: "truncate", "{chosen.name()}" }
                            div { class: "file-chip-size", "{format_size(chosen.size())}" }
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
                            class: "btn btn-primary",
                            disabled: busy(),
                            onclick: preview,
                            "data-testid": "preview-csv".to_owned(),
                            "Preview file (dry run)"
                        }
                        span { class: "text-faint text-sm",
                            "Preview leaves business data unchanged. The file is kept for a day after the preview finishes, so the preview can be committed without choosing it again."
                        }
                    }
                },
            }
            if let Some(message) = error() {
                div { class: "alert alert-danger mt-4", role: "alert", "Could not start the preview: {message}" }
            }
        }
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
