use dioxus::prelude::*;

use crate::components::sidebar::Sidebar;
use crate::components::timer_widget::use_running_timer_provider;

/// Sidebar width bounds, in px. A drag narrower than `SIDEBAR_COLLAPSE_AT` snaps
/// the rail to its collapsed icon strip instead of shrinking further.
const SIDEBAR_MIN: f64 = 200.0;
const SIDEBAR_MAX: f64 = 360.0;
const SIDEBAR_DEFAULT: f64 = 264.0;
const SIDEBAR_COLLAPSE_AT: f64 = 150.0;

#[component]
pub fn AppLayout() -> Element {
    // The running timer is shared from here: the rail below renders it, and the
    // pages in the Outlet can start one and have the rail follow.
    use_running_timer_provider();

    // Owned here (not in the sidebar) so the shell class can narrow the content
    // area together with the rail, and the edge handle can resize it.
    let mut collapsed = use_signal(|| false);
    let mut width = use_signal(|| SIDEBAR_DEFAULT);
    let mut dragging = use_signal(|| false);
    let mut mobile_open = use_signal(|| false);

    let on_navigate = move |_| {
        if mobile_open() {
            mobile_open.set(false);
            document::eval("document.getElementById('app-main').focus();");
        }
    };

    rsx! {
        div {
            class: format!("app-shell{}{}", if collapsed() { " collapsed" } else { "" }, if mobile_open() { " mobile-open" } else { "" }),
            style: "--sidebar-width: {width()}px;",
            onkeydown: move |e: KeyboardEvent| {
                if mobile_open() && e.key() == Key::Escape {
                    mobile_open.set(false);
                    e.prevent_default();
                    document::eval("document.getElementById('navigation-toggle').focus();");
                }
            },
            button {
                id: "navigation-toggle",
                class: "mobile-nav-toggle btn btn-secondary m-4",
                r#type: "button",
                "aria-controls": "app-navigation",
                "aria-expanded": "{mobile_open}",
                onclick: move |_| mobile_open.set(!mobile_open()),
                if mobile_open() { "Close navigation" } else { "Open navigation" }
            }
            Sidebar { collapsed, on_navigate }

            // Drag handle on the rail's right edge. Present when collapsed too, so
            // the rail can be dragged back open (the brand toggle also works).
            div {
                class: "sidebar-resize",
                "aria-hidden": "true",
                onmousedown: move |e: MouseEvent| {
                    e.prevent_default();
                    dragging.set(true);
                },
            }

            main { id: "app-main", class: "app-content", tabindex: "-1",
                Outlet::<crate::route::Route> {}
            }

            // While dragging, a full-viewport catcher tracks the pointer so the
            // resize keeps working over the content (and any embedded iframe).
            // Crossing the threshold either way collapses or re-expands the rail.
            if dragging() {
                div {
                    class: "resize-overlay",
                    onmousemove: move |e: MouseEvent| {
                        let x = e.client_coordinates().x;
                        if x < SIDEBAR_COLLAPSE_AT {
                            collapsed.set(true);
                        } else {
                            collapsed.set(false);
                            width.set(x.clamp(SIDEBAR_MIN, SIDEBAR_MAX));
                        }
                    },
                    onmouseup: move |_| dragging.set(false),
                }
            }
        }
    }
}
