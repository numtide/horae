use dioxus::prelude::*;
use horae_core::importers::harvest::client::ClientSource;

use crate::server_fns;

/// How to fix credentials Harvest refused: here for a stored client, by the
/// operator for one set in the environment.
#[component]
pub(super) fn CredentialsHint() -> Element {
    let settings = use_resource(server_fns::harvest_client_settings);
    let environment = match &*settings.read() {
        Some(Ok(current)) => current.source == ClientSource::Environment,
        _ => return rsx! {},
    };
    rsx! {
        p { class: "banner-detail",
            if environment {
                "This client is set by "
                code { "HORAE_HARVEST_CLIENT_ID" }
                " and "
                code { "HORAE_HARVEST_CLIENT_SECRET" }
                ". Ask the operator to check them and restart the server."
            } else {
                "Check the client ID and secret. A blank secret keeps the stored one, so if you regenerated it in Harvest, paste the new one."
            }
        }
    }
}
