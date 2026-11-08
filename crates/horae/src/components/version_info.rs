use dioxus::prelude::*;

use crate::route::Route;
use crate::server_fns;

const FORBIDDEN: u16 = 403;

/// The server refuses the version to anyone who is not an administrator. The
/// sidebar can still mount the block for them under a permission catalog this
/// bundle does not know, so the refusal is what hides it.
fn hidden(server: Option<&Result<String, ServerFnError>>) -> bool {
    matches!(
        server,
        Some(Err(ServerFnError::ServerError {
            code: FORBIDDEN,
            ..
        }))
    )
}

/// The server's and this front-end's version labels, for administrators. Both
/// come from one build, so a difference means the browser still runs a bundle
/// from before a deploy, and a reload fetches the current one.
#[component]
pub fn VersionInfo() -> Element {
    // The rail stays mounted across navigation, so ask again on every route
    // change: a tab left open across a deploy then sees the new server. A
    // restart keeps the last answer on screen while it runs.
    let route = use_route::<Route>();
    let server = use_resource(use_reactive!(|route| async move {
        let _ = route;
        server_fns::get_server_version().await
    }));
    // Read after mount: server-side rendering would put the server's label in
    // the front-end row, and hydration would then keep it.
    let mut front_end = use_signal(|| None::<String>);
    use_effect(move || front_end.set(Some(crate::version::current())));

    let server = server.read();
    if hidden(server.as_ref()) {
        return rsx! {};
    }
    let server_label = match &*server {
        Some(Ok(label)) => label.as_str(),
        Some(Err(_)) => "unavailable",
        None => "…",
    };
    let front_end = front_end.read();
    let front_end_label = front_end.as_deref().unwrap_or("…");
    let stale = matches!((&*server, &*front_end), (Some(Ok(s)), Some(f)) if s != f);

    rsx! {
        div {
            class: if stale { "sidebar-version stale" } else { "sidebar-version" },
            title: if stale { "This page runs a different build from the server. Reload to update it." },
            dl {
                div {
                    dt { "Server" }
                    dd { "{server_label}" }
                }
                div {
                    dt { "Front-end" }
                    dd { "{front_end_label}" }
                }
            }
            if stale {
                button {
                    class: "btn btn-secondary btn-sm",
                    r#type: "button",
                    onclick: |_| {
                        document::eval("window.location.reload()");
                    },
                    "Reload"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refused_version_hides_the_block() {
        let refused = ServerFnError::ServerError {
            message: "Administrator authority is required".into(),
            code: FORBIDDEN,
            details: None,
        };
        assert!(hidden(Some(&Err(refused))));
    }

    #[test]
    fn other_answers_keep_the_block() {
        let failed = ServerFnError::ServerError {
            message: "Version is unavailable".into(),
            code: 500,
            details: None,
        };
        assert!(!hidden(Some(&Err(failed))));
        assert!(!hidden(Some(&Ok("dev".into()))));
        assert!(!hidden(None));
    }
}
