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
    /// Extra utility classes on the trigger; compact styling remains the default.
    #[props(default)]
    trigger_class: String,
    /// Disable the native trigger without changing other menus' defaults.
    #[props(default)]
    disabled: bool,
    /// The trigger's accessible name when `label` alone is ambiguous, such as
    /// one "Actions" menu per table row.
    #[props(into)]
    aria_label: Option<String>,
    /// An action chosen from this menu is still running; the trigger says so,
    /// since the closed menu cannot.
    #[props(default)]
    busy: bool,
    /// The trigger is the 32px «⋮» icon button of a table row, named by
    /// `aria_label` alone; `label` is not shown. Use [`RowActions`].
    #[props(default)]
    icon: bool,
    children: Element,
) -> Element {
    let popover = if align_right {
        "menu menu-popover menu-popover-right"
    } else {
        "menu menu-popover"
    };
    let trigger = if icon {
        "btn btn-secondary menu-kebab"
    } else {
        "btn btn-secondary btn-sm"
    };
    rsx! {
        document::Script { src: asset!("/assets/js/menu.js") }
        div { class: "menu-anchor",
            button {
                id: "{id}-trigger",
                r#type: "button",
                class: "{trigger} {trigger_class}",
                disabled,
                aria_label,
                "aria-busy": busy.then_some("true"),
                "aria-haspopup": "menu",
                "aria-expanded": "false",
                "aria-controls": "{id}",
                popovertarget: "{id}",
                if icon {
                    // Three stacked dots: zero-length strokes with round caps.
                    svg { view_box: "0 0 16 16", fill: "none", stroke: "currentColor",
                        stroke_width: "2.4", stroke_linecap: "round", "aria-hidden": "true",
                        path { d: "M8 3 V3 M8 8 V8 M8 13 V13" }
                    }
                } else {
                    "{label}"
                    span { class: "ml-2 text-faint", "▾" }
                }
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

/// The actions of one table row: a «⋮» [`Menu`] named "Actions for {name}",
/// anchored to the right edge of the row's last cell.
#[component]
pub fn RowActions(
    /// Unique and stable, as for [`Menu`].
    id: String,
    /// The row's record, for the trigger's accessible name.
    name: String,
    /// As for [`Menu`].
    #[props(default)]
    busy: bool,
    children: Element,
) -> Element {
    let tr = crate::i18n::use_tr();
    rsx! {
        Menu { id, label: String::new(), icon: true, align_right: true,
            aria_label: tr.action_menu_for(&name), busy,
            {children}
        }
    }
}

/// One selectable row inside a [`Menu`].
#[component]
pub fn MenuItem(
    #[props(into)] id: Option<String>,
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
            id,
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
