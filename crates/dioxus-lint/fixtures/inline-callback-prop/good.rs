#[component]
fn Page() -> Element {
    let mut open = use_signal(|| false);
    let on_dismiss = use_callback(move |_| open.set(false));
    rsx! {
        // Native elements take closures directly; only component props compare.
        button { onclick: move |_| open.set(true), "Open" }
        Modal { on_dismiss, "Body" }
    }
}
