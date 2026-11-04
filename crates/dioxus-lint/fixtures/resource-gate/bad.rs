#[component]
fn List() -> Element {
    let rows = use_resource(load_rows);
    // next line flagged
    if rows.state()() != UseResourceState::Ready {
        return rsx! { p { "Loading…" } };
    }
    let ready = rows.state()() == UseResourceState::Ready;
    rsx! {
        // next line flagged
        if !ready { p { "Loading…" } } else { Rows { rows } }
    }
}
