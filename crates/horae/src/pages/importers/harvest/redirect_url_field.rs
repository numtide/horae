use dioxus::prelude::*;
use horae_core::importers::harvest::client::{CALLBACK_PATH, ClientSettings};

/// This page's origin, read once in the browser (empty during SSR).
pub(super) fn use_origin() -> Signal<String> {
    let mut origin = use_signal(String::new);
    use_effect(move || {
        spawn(async move {
            if let Ok(current) = document::eval("return window.location.origin;")
                .join::<String>()
                .await
            {
                origin.set(current);
            }
        });
    });
    origin
}

/// The redirect URL Harvest must have registered: the one in effect, or this
/// instance's own callback for a first save.
pub(super) fn redirect_url(current: &ClientSettings, origin: &str) -> String {
    current
        .redirect_url
        .clone()
        .unwrap_or_else(|| format!("{origin}{CALLBACK_PATH}"))
}

/// The redirect URL, read-only, with a copy button.
#[component]
pub(super) fn RedirectUrlField(url: String) -> Element {
    let mut copied = use_signal(|| false);
    let copy = {
        let url = url.clone();
        move |_| {
            let js = format!(
                "navigator.clipboard.writeText({});",
                serde_json::to_string(&url).unwrap_or_default()
            );
            document::eval(&js);
            copied.set(true);
        }
    };
    rsx! {
        div { class: "input-group",
            input {
                id: "harvest-redirect-url",
                // The banner shows it without a <label>.
                aria_label: "Redirect URL",
                class: "input-group-field text-mono text-xs truncate",
                readonly: true,
                value: "{url}",
            }
            button {
                r#type: "button",
                class: "btn btn-secondary btn-sm",
                "data-testid": "copy-redirect-url",
                aria_label: if copied() { "Redirect URL copied" } else { "Copy redirect URL" },
                onclick: copy,
                if copied() { "Copied" } else { "Copy" }
            }
        }
    }
}
