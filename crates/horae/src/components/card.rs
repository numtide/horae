use dioxus::prelude::*;

/// One section of a screen: a `.card` panel with its title as an
/// `h2.card-title`, which names the section. Without a title it is a plain
/// panel. `id` goes on the title and labels the section; `focusable` lets a
/// retry move focus to that title; `flush` drops the padding for a table or a
/// list that draws its own rows; `class` adds layout utilities such as
/// `max-w-form` or a margin.
#[component]
pub fn Card(
    #[props(into, default)] title: Option<String>,
    #[props(into, default)] id: Option<String>,
    #[props(default)] focusable: bool,
    #[props(default)] flush: bool,
    #[props(into, default)] class: String,
    children: Element,
) -> Element {
    let class = if flush {
        format!("card p-0 overflow-hidden {class}")
    } else {
        format!("card {class}")
    };
    let Some(title) = title else {
        return rsx! { section { class, {children} } };
    };
    let title_class = if flush {
        "card-title px-6 pt-5"
    } else {
        "card-title"
    };
    let tabindex = focusable.then_some("-1");
    rsx! {
        section { class, aria_label: if id.is_none() { Some(title.clone()) } else { None },
            aria_labelledby: id.clone(),
            h2 { id, tabindex, class: title_class, "{title}" }
            {children}
        }
    }
}

/// A single metric: an uppercase label, a mono value, and an optional signed
/// delta. `direction` is `up`, `down`, or `""` for a neutral delta.
#[component]
pub fn MetricCard(
    label: String,
    value: String,
    #[props(default)] delta: String,
    #[props(default)] direction: String,
) -> Element {
    rsx! {
        div { class: "metric-card",
            span { class: "metric-label", "{label}" }
            span { class: "metric-value", "{value}" }
            if !delta.is_empty() {
                span { class: "metric-delta {direction}", "{delta}" }
            }
        }
    }
}
