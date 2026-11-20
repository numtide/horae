#[component]
fn Tasks(count: usize, busy: bool, prefix: String) -> Element {
    let tr = use_tr();
    rsx! {
        h1 { {tr.tasks_title()} }
        p { {tr.tasks_count(count)} }
        nav { aria_label: tr.tasks_pages() }
        PageHeader { title: tr.tasks_empty() }
        if busy {
            span { {tr.tasks_saving()} }
        }
        // A formatted value with no letters, machine props and a NOT_LANGUAGE literal.
        span { "{count} · {count}%" }
        Button { class: "btn btn-secondary", id: "tasks-new", variant: "Primary", to: Route::Tasks {} }
        td { "horae_pat_{prefix}…" }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn renders() {
        let _ = rsx! { p { "Test text is not counted" } };
    }
}
