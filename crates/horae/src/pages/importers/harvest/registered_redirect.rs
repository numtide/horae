use dioxus::prelude::*;

use super::redirect_url_field::{RedirectUrlField, redirect_url, use_origin};
use crate::server_fns;

/// The redirect URL this workspace sends Harvest, to register there.
#[component]
pub(super) fn RegisteredRedirect() -> Element {
    let settings = use_resource(server_fns::harvest_client_settings);
    let origin = use_origin();
    let url = match &*settings.read() {
        Some(Ok(current)) => redirect_url(current, &origin()),
        _ => return rsx! {},
    };
    rsx! {
        RedirectUrlField { url }
    }
}
