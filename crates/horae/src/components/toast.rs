use dioxus::prelude::*;

/// A single toast notification. `variant` is `success`, `danger`, `warning`,
/// `info`, or `""` for neutral.
#[component]
pub fn Toast(
    message: String,
    #[props(default)] variant: String,
    #[props(default)] icon: String,
    /// Show a dismiss (×) button that fires `ondismiss`.
    #[props(default)]
    dismissible: bool,
    #[props(default)] ondismiss: EventHandler<MouseEvent>,
    /// Label of an optional action beside the message, e.g. Undo; empty shows none.
    #[props(default)]
    action_label: String,
    /// Marks the action unavailable without disabling it, so it keeps focus
    /// while pending and after a retryable failure.
    #[props(default)]
    action_busy: bool,
    #[props(default)] onaction: EventHandler<MouseEvent>,
) -> Element {
    // An action can make the toast too wide for narrow screens and large text.
    let wrap = if action_label.is_empty() {
        ""
    } else {
        " flex-wrap"
    };
    rsx! {
        div { class: "toast {variant}{wrap}", role: "status",
            if !icon.is_empty() {
                span { "{icon}" }
            }
            span { class: "toast-msg", "{message}" }
            if !action_label.is_empty() {
                button {
                    r#type: "button",
                    class: if action_busy { "btn btn-ghost btn-sm is-disabled" } else { "btn btn-ghost btn-sm" },
                    "aria-disabled": action_busy,
                    onclick: move |e| if !action_busy { onaction.call(e) },
                    "{action_label}"
                }
            }
            if dismissible {
                button {
                    r#type: "button",
                    class: "toast-close",
                    "aria-label": "Dismiss",
                    onclick: move |e| ondismiss.call(e),
                    "×"
                }
            }
        }
    }
}

/// The fixed bottom-right stack that holds active toasts.
#[component]
pub fn ToastContainer(children: Element) -> Element {
    rsx! {
        div { class: "toast-container", {children} }
    }
}
