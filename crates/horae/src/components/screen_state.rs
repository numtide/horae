use dioxus::prelude::*;

use crate::components::icons::NavIcon;

/// The body of an `ErrorState` for a load that failed for no reason the page
/// can name.
pub const LOAD_FAILED: &str =
    "The request did not complete. Try again, or reload the page if it keeps failing.";

/// A server function error as one line for an `ErrorState`: the server's own
/// message, without the "error running server function" wrapper.
pub fn error_line(error: &ServerFnError) -> String {
    match error {
        ServerFnError::ServerError { message, .. } => message.clone(),
        other => other.to_string(),
    }
}

/// A region with nothing to show yet (`DESIGN.md`, *Empty, loading and error
/// states*): an icon tile, a serif title and copy that says what to do next.
/// `actions` holds the optional button and text link; `success` tints the icon
/// green for an all-clear moment such as nothing left to approve.
///
/// `inset` is for a state inside a panel that already has a border and a
/// heading of its own: it drops the card and titles the state with an `h3`.
#[component]
pub fn EmptyState(
    icon: &'static str,
    #[props(into)] title: String,
    #[props(into, default)] body: String,
    #[props(default)] success: bool,
    #[props(default)] inset: bool,
    #[props(into)] id: Option<String>,
    actions: Option<Element>,
) -> Element {
    let tone = if success {
        "empty-state-icon success"
    } else {
        "empty-state-icon"
    };
    rsx! {
        div { id, class: if inset { "empty-state inset" } else { "empty-state" },
            span { class: tone, aria_hidden: "true", NavIcon { name: icon } }
            if inset { h3 { class: "empty-state-title", "{title}" } } else { h2 { class: "empty-state-title", "{title}" } }
            if !body.is_empty() {
                p { class: "empty-state-text", "{body}" }
            }
            if let Some(actions) = actions {
                div { class: "empty-state-actions", {actions} }
            }
        }
    }
}

/// A region or screen that failed to load. It keeps the empty state's shape,
/// with the icon in the danger tint and "Try again" as the action. A
/// permission error passes no `onretry` and names who can grant access in
/// `body` instead.
#[component]
pub fn ErrorState(
    #[props(into)] title: String,
    #[props(into)] body: String,
    /// A mono line for support, such as a request reference.
    #[props(into, default)]
    request: String,
    onretry: Option<EventHandler<()>>,
    /// The retry button's id, for pages that move focus to it.
    #[props(into)]
    retry_id: Option<String>,
    #[props(default)] retry_disabled: bool,
    /// As for `EmptyState`.
    #[props(default)]
    inset: bool,
    #[props(into)] id: Option<String>,
    actions: Option<Element>,
) -> Element {
    let icon = if onretry.is_some() { "warning" } else { "lock" };
    rsx! {
        div { id, class: if inset { "empty-state inset" } else { "empty-state" }, role: "alert",
            span { class: "empty-state-icon danger", aria_hidden: "true", NavIcon { name: icon } }
            if inset { h3 { class: "empty-state-title", "{title}" } } else { h2 { class: "empty-state-title", "{title}" } }
            p { class: "empty-state-text", "{body}" }
            if !request.is_empty() {
                code { class: "empty-state-code", "{request}" }
            }
            if onretry.is_some() || actions.is_some() {
                div { class: "empty-state-actions",
                    if let Some(onretry) = onretry {
                        button {
                            id: retry_id,
                            r#type: "button",
                            class: "btn btn-secondary",
                            disabled: retry_disabled,
                            onclick: move |_| onretry.call(()),
                            "Try again"
                        }
                    }
                    {actions}
                }
            }
        }
    }
}

/// The one inline loading message: a `role=status` line marked busy, so
/// assistive technology hears the same thing in every region.
#[component]
pub fn Loading(
    #[props(into, default = "Loading…".to_string())] label: String,
    #[props(into, default)] class: String,
) -> Element {
    rsx! {
        p { class: "loading {class}", role: "status", aria_busy: "true", "{label}" }
    }
}

/// Placeholder rows for a listing whose layout is known before its data
/// arrives. Only the data region is replaced; headers and filters stay live.
/// `columns` is the number of cells per row; the first one is the widest.
#[component]
pub fn Skeleton(
    #[props(default = 6)] rows: usize,
    #[props(default = 4)] columns: usize,
    #[props(into, default = "Loading…".to_string())] label: String,
) -> Element {
    rsx! {
        div { class: "skeleton", role: "status", aria_busy: "true", aria_label: "{label}",
            for row in 0..rows {
                div { key: "{row}", class: "skeleton-row",
                    for cell in 0..columns {
                        span { key: "{cell}", class: "skeleton-cell", span { class: "skeleton-bar" } }
                    }
                }
            }
        }
    }
}
