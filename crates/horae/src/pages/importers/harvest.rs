//! The Harvest section: connection, CSV upload, run history and one page per
//! run (spec 019 US2, contracts/harvest-section.md).

use dioxus::prelude::*;
use horae_core::importers::harvest::types::{ConnectionStatus, EntityType};

use crate::server_fns;

#[path = "harvest/back_links.rs"]
mod back_links;
#[path = "harvest/change_account.rs"]
mod change_account;
#[path = "harvest/client_settings.rs"]
mod client_settings;
#[path = "harvest/connection_card.rs"]
mod connection_card;
#[path = "harvest/csv.rs"]
mod csv;
#[path = "harvest/entity_table.rs"]
mod entity_table;
#[path = "harvest/error_groups.rs"]
mod error_groups;
#[path = "harvest/history_table.rs"]
mod history_table;
#[path = "harvest/missing_people.rs"]
mod missing_people;
#[path = "harvest/missing_person_row.rs"]
mod missing_person_row;
#[path = "harvest/oauth.rs"]
mod oauth;
#[path = "harvest/overview.rs"]
mod overview;
#[path = "harvest/presentation.rs"]
mod presentation;
#[path = "harvest/run.rs"]
mod run;
#[path = "harvest/run_list.rs"]
mod run_list;
#[path = "harvest/run_panel.rs"]
mod run_panel;
#[path = "harvest/run_view.rs"]
mod run_view;
#[path = "harvest/runs.rs"]
mod runs;
#[path = "harvest/runs_cursor.rs"]
mod runs_cursor;

pub use csv::HarvestCsv;
pub use oauth::HarvestOAuth;
pub use overview::HarvestOverview;
pub use run::HarvestRun;
pub use runs::HarvestRuns;
pub use runs_cursor::RunsCursor;

/// Loaded/loading/error view of the connection resource.
#[derive(Clone, PartialEq)]
pub(super) enum Conn {
    Loading,
    Err(String),
    Ready(ConnectionStatus),
}

impl Conn {
    pub(super) fn from_resource(
        status: &Resource<Result<ConnectionStatus, ServerFnError>>,
    ) -> Self {
        match (&*status.state().read(), &*status.read_unchecked()) {
            (UseResourceState::Pending, _) | (_, None) => Self::Loading,
            (_, Some(Ok(s))) => Self::Ready(s.clone()),
            (_, Some(Err(e))) => Self::Err(e.to_string()),
        }
    }

    pub(super) fn label(&self) -> &'static str {
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

    /// The badge class that goes with [`Self::label`].
    pub(super) fn tone(&self) -> &'static str {
        match self {
            Self::Ready(s) if s.connected && s.token_expired => "badge-warning",
            Self::Ready(s) if s.connected => "badge-success",
            Self::Err(_) => "badge-danger",
            _ => "badge-neutral",
        }
    }

    fn ready(&self) -> Option<&ConnectionStatus> {
        match self {
            Self::Ready(s) => Some(s),
            _ => None,
        }
    }
}

/// The current connection, reloaded by `restart()` after a change.
pub(super) fn use_connection() -> Resource<Result<ConnectionStatus, ServerFnError>> {
    use_resource(|| async move { server_fns::harvest_connection_status().await })
}

fn entity_label(e: EntityType) -> &'static str {
    match e {
        EntityType::Client => "Clients",
        EntityType::Project => "Projects",
        EntityType::Task => "Tasks",
        EntityType::Person => "People",
        EntityType::TimeEntry => "Time entries",
    }
}

/// The wait between two status reads of a running import.
async fn pause() {
    #[cfg(feature = "web")]
    gloo_timers::future::TimeoutFuture::new(1_000).await;
    #[cfg(feature = "server")]
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
}

/// Harvest's mark beside its name, where Horae imports from it. Decorative:
/// the name next to it is the text alternative.
#[component]
pub(super) fn HarvestLogo(class: &'static str) -> Element {
    rsx! {
        img { class, src: asset!("/assets/harvest-logo.svg"), alt: "", width: "40", height: "40" }
    }
}

/// Hand the browser to Harvest to authorize (or re-authorize) Horae.
async fn go_to_harvest() -> Result<(), ServerFnError> {
    let url = server_fns::harvest_connect_start().await?;
    let js = format!(
        "window.location.href = {};",
        serde_json::to_string(&url).unwrap_or_default()
    );
    let _ = document::eval(&js).await;
    Ok(())
}

/// Shared subtitle of the source pages.
const IMPORT_SUB: &str = "Bring your clients, projects, tasks, people and time entries across. Preview first; import only after reviewing the results.";
