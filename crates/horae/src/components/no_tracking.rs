use dioxus::prelude::*;

use crate::route::Route;

/// Why time cannot be tracked yet, shown wherever a start or add control would
/// otherwise sit disabled without a reason. Controls it explains point at `id`
/// with `aria-describedby`. `on_follow` runs when the link is taken, for a
/// popover that should close as the page changes.
#[component]
pub fn NoTrackingHint(
    id: String,
    can_create: bool,
    #[props(default)] on_follow: EventHandler<()>,
) -> Element {
    rsx! {
        p { id, class: "no-tracking-hint",
            "No projects to track yet."
            if can_create {
                " "
                Link { to: Route::NewProject {}, onclick: move |_| on_follow.call(()), "Create a project" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The creator's link needs a router; tests/browser/empty-tracking.cjs
    // covers it in the running app.
    #[test]
    fn explains_without_offering_creation_to_those_who_cannot_create() {
        // The default `on_follow` handler needs a runtime.
        let mut dom = VirtualDom::new(|| rsx! { NoTrackingHint { id: "hint", can_create: false } });
        dom.rebuild_in_place();
        let html = dioxus::ssr::render(&dom);
        assert!(html.contains("No projects to track yet."), "{html}");
        assert!(!html.contains("<a"), "{html}");
    }
}
