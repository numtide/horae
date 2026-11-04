#[component]
fn Page() -> Element {
    let drag = use_signal(|| None::<u32>);
    rsx! { {render_calendar(drag)} }
}

// next line flagged
fn render_calendar(drag: Signal<Option<u32>>) -> Element {
    rsx! { "{drag:?}" }
}
