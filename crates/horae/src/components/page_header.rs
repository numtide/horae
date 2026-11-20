use dioxus::prelude::*;

/// The top of every screen: the title and the screen's actions (`DESIGN.md`,
/// *Page header*). A screen under a section puts a `BackLink` to its parent
/// right above it; screens reached from the sidebar have none.
///
/// `title_id` makes the heading a focus target, for pages that return focus to it.
///
/// The Timesheet's `.ts-header` rules in `horae.css` dissolve `.page-heading`,
/// its inner `div` and `.page-title-row` on narrow widths; keep them in step
/// with this markup.
#[component]
pub fn PageHeader(
    #[props(into)] title: String,
    #[props(into)] title_id: Option<String>,
    /// An integration's own logo before the title (Harvest); nothing else.
    leading: Option<Element>,
    /// One line above the title, for facts about the record (a project's client).
    context: Option<Element>,
    /// Inline after the title: status badges and short qualifiers.
    meta: Option<Element>,
    actions: Option<Element>,
) -> Element {
    let focusable = title_id.is_some().then_some("-1");
    rsx! {
        header { class: "page-header",
            div { class: "page-heading flex items-center gap-4 min-w-0",
                if let Some(leading) = leading {
                    div { class: "page-leading", {leading} }
                }
                div { class: "min-w-0",
                    if let Some(context) = context {
                        div { class: "text-sm text-secondary mb-2", {context} }
                    }
                    div { class: "page-title-row flex flex-wrap items-center gap-3 min-w-0",
                        h1 { id: title_id, tabindex: focusable, class: "page-title", "{title}" }
                        {meta}
                    }
                }
            }
            if let Some(actions) = actions {
                div { class: "page-actions", {actions} }
            }
        }
    }
}
