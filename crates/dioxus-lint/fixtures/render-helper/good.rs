#[component]
fn Page() -> Element {
    let drag = use_signal(|| None::<u32>);
    rsx! { Calendar { drag } }
}

#[component]
fn Calendar(drag: Signal<Option<u32>>) -> Element {
    rsx! { "{drag:?}" }
}

fn render_label(minutes: i64) -> String {
    format!("{minutes} min")
}

#[dioxus::prelude::component]
fn render_badge(label: String) -> Element {
    rsx! { "{label}" }
}
