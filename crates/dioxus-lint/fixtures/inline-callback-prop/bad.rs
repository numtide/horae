#[component]
fn Page() -> Element {
    let mut open = use_signal(|| false);
    rsx! {
        // next line flagged
        Modal { on_dismiss: move |_| open.set(false), "Body" }
    }
}
