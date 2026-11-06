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
#[path = "harvest/error_groups.rs"]
mod error_groups;
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
#[path = "harvest/report_summary.rs"]
mod report_summary;
#[path = "harvest/run.rs"]
mod run;
#[path = "harvest/run_header.rs"]
mod run_header;
#[path = "harvest/run_list.rs"]
mod run_list;
#[path = "harvest/run_view.rs"]
mod run_view;
#[path = "harvest/runs.rs"]
mod runs;
#[path = "harvest/runs_cursor.rs"]
mod runs_cursor;
#[path = "harvest/stat_cell.rs"]
mod stat_cell;
#[path = "harvest/stat_tile.rs"]
mod stat_tile;

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

/// Shared subtitle of the source pages.
const IMPORT_SUB: &str = "Bring your clients, projects, tasks, people and time entries across. Preview first; commit only after reviewing the results.";
