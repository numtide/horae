use dioxus::prelude::*;

use crate::components::icons::NavIcon;

/// The kit's back link, on its own row above a `PageHeader`. It names the
/// parent in the hierarchy ("Back to Projects"), never the previous screen.
#[component]
pub fn BackLink(#[props(into)] to: NavigationTarget, #[props(into)] label: String) -> Element {
    rsx! {
        Link { to, id: "page-back", class: "back-link",
            span { class: "inline-flex", aria_hidden: "true", NavIcon { name: "arrow-left", class: "size-4" } }
            "{label}"
        }
    }
}
