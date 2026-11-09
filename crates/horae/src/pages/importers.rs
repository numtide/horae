//! The importers index and the Harvest section. Every page has its own URL,
//! so a run can be linked, reloaded and bookmarked (spec 019 US2).

use dioxus::prelude::*;

use crate::route::Route;

#[path = "importers/harvest.rs"]
mod harvest;

pub use harvest::{
    ConnectOutcome, HarvestCsv, HarvestOAuth, HarvestOverview, HarvestRun, HarvestRuns, RunsCursor,
    SecretFocus,
};

/// The sources Horae imports from: the Harvest API and a Harvest CSV export.
#[component]
pub fn ImporterIndex() -> Element {
    let status = harvest::use_connection();
    let connection = harvest::Conn::from_resource(&status);
    rsx! {
        div { class: "himp-page min-w-0",
            div { class: "page-header",
                h1 { class: "page-title", "Importers" }
            }
            p { class: "text-secondary text-sm mb-6",
                "Bring data in from another tracker. Start with a preview before importing business data."
            }
            div { class: "flex flex-col gap-3",
                Link { to: Route::HarvestOverview { outcome: Default::default() }, class: "imp-source", "data-testid": "choose-api",
                    harvest::HarvestLogo { class: "imp-source-icon" }
                    div { class: "flex-1 min-w-0",
                        div { class: "flex items-center gap-2 flex-wrap",
                            span { class: "text-sm font-semibold", "Harvest" }
                            span { class: "badge {connection.tone()} badge-sm", "data-testid": "index-connection-status", "{connection.label()}" }
                        }
                        div { class: "text-faint text-sm", "Read-only sync via the Harvest API." }
                    }
                    span { class: "text-faint", "›" }
                }
                Link { to: Route::HarvestCsv {}, class: "imp-source", "data-testid": "choose-csv",
                    span { class: "imp-source-icon imp-icon-csv text-mono text-xs", "CSV" }
                    div { class: "flex-1 min-w-0",
                        div { class: "text-sm font-semibold", "CSV file" }
                        div { class: "text-faint text-sm",
                            "Upload a supported Detailed time report exported from Harvest."
                        }
                    }
                    span { class: "text-faint", "›" }
                }
            }
        }
    }
}
