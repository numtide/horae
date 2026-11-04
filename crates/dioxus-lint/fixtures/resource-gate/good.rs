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
