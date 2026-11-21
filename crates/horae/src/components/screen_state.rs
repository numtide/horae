use std::cell::Cell;
use std::rc::Rc;

use dioxus::prelude::*;

use crate::components::icons::NavIcon;

/// The body of an `ErrorState` for a load that failed for no reason the page
/// can name.
pub const LOAD_FAILED: &str =
    "The request did not complete. Try again, or reload the page if it keeps failing.";

/// Whether a failed read was refused (403), so the region shows `NoAccess`.
pub fn is_refusal(error: &ServerFnError) -> bool {
    const FORBIDDEN: u16 = 403;
    matches!(
        error,
        ServerFnError::ServerError {
            code: FORBIDDEN,
            ..
        }
    )
}

/// Runs a read pinned to the account that first loaded the screen. When the
/// server refuses the pin (403) or answers for another account, the signed-in
/// account changed: the read runs once more unpinned, so the screen re-pins to
/// the account now signed in, and only a refusal of that read is a real one.
/// The flag says the account changed, so the screen drops what it held for
/// the old one.
pub async fn load_pinned<T, R, Fut>(
    pinned: Option<R>,
    load: impl Fn(Option<R>) -> Fut,
    requester: impl Fn(&T) -> R,
) -> (Result<T, ServerFnError>, bool)
where
    R: Copy + PartialEq,
    Fut: Future<Output = Result<T, ServerFnError>>,
{
    let result = load(pinned).await;
    let changed = pinned.is_some_and(|pinned| match &result {
        Ok(page) => requester(page) != pinned,
        Err(error) => is_refusal(error),
    });
    if changed {
        (load(None).await, true)
    } else {
        (result, false)
    }
}

/// The one state for a screen or region the person may not open (`DESIGN.md`,
/// *Screen States → No permission*). It names who grants access and nothing
/// about what is behind it; the screen around it keeps only its header.
#[component]
pub fn NoAccess(#[props(default)] inset: bool) -> Element {
    let tr = crate::i18n::use_tr();
    rsx! {
        ErrorState { title: tr.no_access_title(), body: tr.no_access_body(), inset }
    }
}

/// What an `ErrorBoundary` shows when a page cannot load what it needs to
/// render at all (the session, read with `use_loader`). Reloading runs the
/// reads again. When the session `ended`, signing out leads to the sign-in
/// page, as the account menu's "Sign out" does.
#[component]
pub fn PageFailed(#[props(default)] ended: bool) -> Element {
    let tr = crate::i18n::use_tr();
    let reload = use_callback(|()| {
        document::eval("window.location.reload()");
    });
    if ended {
        return rsx! {
            ErrorState { title: tr.page_session_ended(), body: tr.common_session_ended(),
                actions: rsx! {
                    form { method: "post", action: "/auth/logout",
                        button { class: "btn btn-primary", r#type: "submit", {tr.page_sign_in_again()} }
                    }
                },
            }
        };
    }
    rsx! {
        ErrorState { title: tr.page_failed(), body: tr.page_failed_hint(), onretry: reload }
    }
}

/// The server's own message, for the request line of an `ErrorState`. A
/// transport failure ("error sending request…") has none worth showing.
pub fn error_line(error: &ServerFnError) -> Option<String> {
    match error {
        ServerFnError::ServerError { message, .. } => Some(message.clone()),
        _ => None,
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
///
/// Keep it mounted while a retry is pending (`retry_disabled`), so focus stays
/// on its button. `focus_on_recovery` names the region's heading (focusable,
/// `tabindex=-1`): once a retry succeeds and the state unmounts, focus moves
/// there instead of falling to the page.
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
    #[props(into)] focus_on_recovery: Option<String>,
    /// Replaces the glyph the state picks (warning with a retry, lock without).
    icon: Option<&'static str>,
    /// As for `EmptyState`.
    #[props(default)]
    inset: bool,
    #[props(into)] id: Option<String>,
    actions: Option<Element>,
) -> Element {
    let retried = use_hook(|| Rc::new(Cell::new(false)));
    use_drop({
        let retried = retried.clone();
        let target = focus_on_recovery.clone();
        move || {
            if let (true, Some(target)) = (retried.get(), target) {
                document::eval(&format!(
                    "requestAnimationFrame(() => document.getElementById({target:?})?.focus())"
                ));
            }
        }
    });
    let icon = icon.unwrap_or(if onretry.is_some() { "warning" } else { "lock" });
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
                        // `aria-disabled`, not `disabled`: a disabled button drops
                        // focus to the page while the retry runs.
                        button {
                            id: retry_id,
                            r#type: "button",
                            class: if retry_disabled { "btn btn-secondary is-disabled" } else { "btn btn-secondary" },
                            aria_disabled: retry_disabled.then_some("true"),
                            onclick: move |_| {
                                if retry_disabled {
                                    return;
                                }
                                retried.set(true);
                                onretry.call(());
                            },
                            "Try again"
                        }
                    }
                    {actions}
                }
            }
        }
    }
}

/// A failed server read, shown by what the failure means: a refusal (401 or
/// 403) names who can grant access, a missing record (404) says so, and only
/// anything else offers "Try again". `what` names the data ("this invoice").
#[component]
pub fn LoadError(
    error: ServerFnError,
    #[props(into)] what: String,
    retry: Callback<()>,
    #[props(default)] pending: bool,
    #[props(default)] inset: bool,
    #[props(into)] focus_on_recovery: Option<String>,
    #[props(into)] retry_id: Option<String>,
) -> Element {
    const UNAUTHORIZED: u16 = 401;
    const FORBIDDEN: u16 = 403;
    const NOT_FOUND: u16 = 404;
    let tr = crate::i18n::use_tr();
    match &error {
        ServerFnError::ServerError {
            code: UNAUTHORIZED, ..
        } => rsx! {
            ErrorState { title: "Could not load {what}", body: tr.common_session_ended(), inset }
        },
        ServerFnError::ServerError {
            code: FORBIDDEN, ..
        } => rsx! {
            NoAccess { inset }
        },
        ServerFnError::ServerError {
            code: NOT_FOUND,
            message,
            ..
        } => rsx! {
            ErrorState {
                title: "{message}",
                body: "It may have been deleted, or the link may be out of date.",
                icon: "info",
                inset,
            }
        },
        _ => rsx! {
            ErrorState {
                title: "Could not load {what}",
                body: LOAD_FAILED,
                request: error_line(&error).unwrap_or_default(),
                onretry: retry,
                retry_disabled: pending,
                focus_on_recovery,
                retry_id,
                inset,
            }
        },
    }
}

/// How `loaded` shows a failed read (see `LoadError`).
pub struct Failure {
    /// Names the data in the error title ("Could not load {what}").
    pub what: &'static str,
    /// Made with `use_callback`, so the error state's props stay equal.
    pub retry: Callback<()>,
    /// A retry is under way: the error stays, with its button disabled.
    pub pending: bool,
    /// Inside a panel with its own border (see `EmptyState`).
    pub inset: bool,
    /// The focusable heading of the region, for when a retry succeeds.
    pub focus: Option<&'static str>,
}

/// Render a resource's value: the shared `Loading` line before the first
/// answer, a `LoadError` when the latest answer failed (also while a retry is
/// pending), and the content otherwise.
pub fn loaded<T>(
    state: &Option<Result<T, ServerFnError>>,
    failure: Failure,
    render: impl FnOnce(&T) -> Element,
) -> Element {
    match state {
        Some(Ok(v)) => render(v),
        Some(Err(e)) => rsx! {
            LoadError {
                error: e.clone(),
                what: failure.what,
                retry: failure.retry,
                pending: failure.pending,
                inset: failure.inset,
                focus_on_recovery: failure.focus.map(String::from),
            }
        },
        None => rsx! {
            Loading {}
        },
    }
}

/// The one inline loading message, a `role=status` line. It is not itself
/// `aria-busy`, which would let a screen reader hold back the announcement; a
/// region that keeps its old content while reloading marks its own container.
#[component]
pub fn Loading(
    #[props(into, default = "Loading…".to_string())] label: String,
    #[props(into, default)] class: String,
) -> Element {
    rsx! {
        p { class: "loading {class}", role: "status", "{label}" }
    }
}

/// Placeholder rows for a listing whose layout is known before its data
/// arrives. Only the data region is replaced; headers and filters stay live.
/// `columns` is the number of cells per row; the first one is the widest. The
/// rows are busy and hidden from assistive technology; `label` is announced as
/// visually hidden status text instead.
#[component]
pub fn Skeleton(
    #[props(default = 6)] rows: usize,
    #[props(default = 4)] columns: usize,
    #[props(into, default = "Loading…".to_string())] label: String,
) -> Element {
    rsx! {
        div { class: "skeleton",
            p { class: "sr-only", role: "status", "{label}" }
            div { aria_busy: "true", aria_hidden: "true",
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(error: ServerFnError) -> String {
        let mut dom = VirtualDom::new_with_props(
            |error: ServerFnError| {
                let retry = use_callback(|_| {});
                rsx! { LoadError { error, what: "this invoice", retry } }
            },
            error,
        );
        dom.rebuild_in_place();
        dioxus::ssr::render(&dom)
    }

    fn server(code: u16, message: &str) -> ServerFnError {
        ServerFnError::ServerError {
            code,
            message: message.into(),
            details: None,
        }
    }

    #[test]
    fn refusals_and_missing_records_offer_no_retry() {
        for (error, expected) in [
            (server(401, "Sign in"), "Sign in again"),
            (server(403, "Forbidden"), "Access unavailable"),
            (server(404, "Invoice not found"), "Invoice not found"),
        ] {
            let html = render(error);
            assert!(html.contains(expected), "{html}");
            assert!(!html.contains("Try again"), "{html}");
        }
    }

    #[test]
    fn other_failures_retry_without_transport_text() {
        let html = render(server(500, "Database unavailable"));
        assert!(
            html.contains("Try again") && html.contains("Database unavailable"),
            "{html}"
        );
        let html = render(ServerFnError::StreamError("Failed to fetch".into()));
        assert!(html.contains("Try again"), "{html}");
        assert!(!html.contains("Failed to fetch"), "{html}");
    }
}
