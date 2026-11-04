#[component]
fn List() -> Element {
    let rows = use_resource(load_rows);
    // Pending flags, effects and first-load gates keep the subtree mounted on reload.
    let pending = rows.state()() != UseResourceState::Ready;
    use_effect(move || {
        if rows.state()() != UseResourceState::Ready {
            return;
        }
    });
    if rows.read().is_none() {
        return rsx! { p { "Loading…" } };
    }
    rsx! {
        div { aria_busy: pending, Rows { rows } }
        if pending { p { "Refreshing…" } }
    }
}

#[component]
fn Rebound() -> Element {
    let rows = use_resource(load_rows);
    let ready = rows.state()() == UseResourceState::Ready;
    // Re-binding the name replaces the tracked comparison.
    let ready = ready || rows.read().is_some();
    if !ready {
        return rsx! { p { "Loading…" } };
    }
    rsx! { Rows { rows } }
}

#[component]
fn Scoped(ready: bool) -> Element {
    let rows = use_resource(load_rows);
    let refresh = use_callback(move |_| {
        // Scoped to the closure; the `ready` below is the prop.
        let ready = rows.state()() == UseResourceState::Ready;
        if ready {
            rows.restart();
        }
    });
    if !ready {
        return rsx! { p { "Loading…" } };
    }
    rsx! { Rows { rows, refresh } }
}
