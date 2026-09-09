use dioxus::prelude::*;

/// A dropdown menu: a trigger button that reveals a popover of [`MenuItem`]s.
/// Uses the browser's top layer so scrollable tables cannot clip its items.
/// Native light dismissal and menu.js handle dismissal, positioning and keys.
#[component]
pub fn Menu(
    /// Unique, stable DOM ID, also used to associate the trigger and menu.
    id: String,
    /// Text shown on the trigger button (a `▾` caret is appended).
    label: String,
    /// Anchor the popover to the right edge — for right-aligned cells.
    #[props(default)]
    align_right: bool,
    children: Element,
) -> Element {
    let popover = if align_right {
        "menu menu-popover menu-popover-right"
    } else {
        "menu menu-popover"
    };
    rsx! {
        document::Script { src: asset!("/assets/js/menu.js") }
        div { class: "menu-anchor",
            button {
                id: "{id}-trigger",
                r#type: "button",
                class: "btn btn-secondary btn-sm",
                "aria-haspopup": "menu",
                "aria-expanded": "false",
                "aria-controls": "{id}",
                popovertarget: "{id}",
                "{label}"
                span { class: "ml-2 text-faint", "▾" }
            }
            div {
                id: "{id}",
                class: popover,
                popover: "auto",
                role: "menu",
                "aria-labelledby": "{id}-trigger",
                {children}
            }
        }
    }
}

/// One selectable row inside a [`Menu`].
#[component]
pub fn MenuItem(
    #[props(default)] onclick: EventHandler<MouseEvent>,
    #[props(default)] selected: bool,
    #[props(default)] danger: bool,
    #[props(default)] disabled: bool,
    children: Element,
) -> Element {
    let mut class = String::from("menu-item");
    if selected {
        class.push_str(" selected");
    }
    if danger {
        class.push_str(" danger");
    }
    if disabled {
        class.push_str(" disabled");
    }
    rsx! {
        button {
            r#type: "button",
            role: "menuitem",
            tabindex: "-1",
            "aria-current": selected.then_some("true"),
            class: "{class}",
            disabled,
            onclick: move |e| onclick.call(e),
            {children}
        }
    }
}

/// A hairline separator between groups of [`MenuItem`]s.
#[component]
pub fn MenuDivider() -> Element {
    rsx! {
        div { class: "menu-divider", role: "separator" }
    }
}
