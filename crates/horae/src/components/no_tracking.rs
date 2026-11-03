use dioxus::prelude::*;

use crate::route::Route;

/// Why `assignee` (the viewer when `None`) has nothing to track. The catalog
/// is per person and per assignment, so this names assignments rather than
/// claiming the organization has no projects.
pub fn no_tracking_message(assignee: Option<&str>) -> String {
    match assignee {
        None => "No projects are assigned to you yet.".into(),
        Some(name) => format!("No projects are assigned to {name} yet."),
    }
}

/// Why time cannot be tracked, shown wherever a start or add control would
/// otherwise sit disabled without a reason. Controls it explains point at `id`
/// with `aria-describedby`. On the viewer's own catalog, `can_manage` adds a
/// link to Projects, where projects are created and teams assigned;
/// `on_follow` runs when it is taken, for a popover that should close.
#[component]
pub fn NoTrackingHint(
    id: String,
    #[props(default)] assignee: Option<String>,
    can_manage: bool,
    #[props(default)] on_follow: EventHandler<()>,
) -> Element {
    let own = assignee.is_none();
    rsx! {
        p { id, class: "no-tracking-hint",
            {no_tracking_message(assignee.as_deref())}
            if own && can_manage {
                " "
                Link { to: Route::ProjectList {}, onclick: move |_| on_follow.call(()), "Open Projects" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(assignee: Option<&str>) -> String {
        let assignee = assignee.map(str::to_string);
        // The default `on_follow` handler needs a runtime.
        let mut dom = VirtualDom::new_with_props(
            |assignee: Option<String>| rsx! { NoTrackingHint { id: "hint", assignee, can_manage: false } },
            assignee,
        );
        dom.rebuild_in_place();
        dioxus::ssr::render(&dom)
    }

    // The manager's link needs a router; tests/browser/empty-tracking.cjs
    // covers it in the running app.
    #[test]
    fn names_whose_assignments_are_missing() {
        let own = render(None);
        assert!(own.contains("No projects are assigned to you yet."), "{own}");
        assert!(!own.contains("<a"), "{own}");

        let other = render(Some("Ada Lovelace"));
        assert!(other.contains("No projects are assigned to Ada Lovelace yet."), "{other}");
    }
}
