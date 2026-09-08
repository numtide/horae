use dioxus::prelude::*;

/// Keep mounted while toggling `open`: closing the native dialog restores focus
/// to its opener, and the browser keeps the rest of the page inert while open.
#[component]
pub fn Modal(
    id: &'static str,
    labelledby: &'static str,
    open: bool,
    #[props(default = false)] busy: bool,
    #[props(default = false)] large: bool,
    on_dismiss: EventHandler<()>,
    children: Element,
) -> Element {
    let mut backdrop_press = use_signal(|| false);
    use_effect(use_reactive!(|(open, id)| {
        let id = serde_json::json!(id);
        document::eval(&format!(
            "const dialog = document.getElementById({id});
             if (dialog) {{
                 if ({open} && !dialog.open) dialog.showModal();
                 else if (!{open} && dialog.open) dialog.close();
             }}"
        ));
    }));

    rsx! {
        dialog {
            id,
            class: "modal-overlay",
            aria_labelledby: labelledby,
            aria_busy: busy,
            oncancel: move |e| {
                e.prevent_default();
                if !busy { on_dismiss.call(()); }
            },
            onkeydown: move |e| {
                // Escape belongs to this dialog, not the mobile navigation.
                if e.key() == Key::Escape { e.stop_propagation(); }
            },
            onpointerdown: move |_| backdrop_press.set(true),
            onclick: move |_| {
                // Selecting text and releasing outside must not discard a form.
                if backdrop_press() && !busy { on_dismiss.call(()); }
            },
            div {
                class: if large { "modal modal-lg" } else { "modal" },
                onpointerdown: move |e| {
                    backdrop_press.set(false);
                    e.stop_propagation();
                },
                onclick: move |e| e.stop_propagation(),
                {children}
            }
        }
    }
}
