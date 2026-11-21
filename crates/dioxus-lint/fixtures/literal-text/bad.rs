#[component]
fn Tasks(count: usize, busy: bool) -> Element {
    rsx! {
        // next line flagged
        h1 { "Tasks" }
        // next line flagged
        p { "{count} tasks" }
        nav {
            // next line flagged
            aria_label: "Task pages",
        }
        PageHeader {
            // next line flagged
            title: "No tasks yet",
        }
        if busy {
            // next line flagged
            span { "Saving…" }
        }
        // next line flagged
        Button { label: "API" }
        // next line flagged
        Button { label: "{count} selected" }
        // next line flagged
        p { {"Hello"} }
        input {
            // next line flagged
            placeholder: if busy { "Add tags…" } else { "" },
        }
    }
}

#[cfg(not(test))]
fn Shipped() -> Element {
    rsx! {
        // next line flagged
        p { "Production code" }
    }
}
