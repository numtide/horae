#[component]
fn Page() -> Element {
    let mut open = use_signal(|| false);
    let mut day = use_signal(|| 0);
    rsx! {
        // next line flagged
        Modal { on_dismiss: move |_| open.set(false), "Body" }
        // Any closure-valued prop, whatever its name, and closures behind a block or parentheses.
        Calendar {
            // next line flagged
            select_day: move |d| day.set(d),
            // next line flagged
            drag_commit: (move |_| open.set(false)),
        }
        for row in rows() {
            MenuItem {
                // next line flagged
                onclick: {
                    let id = row.id;
                    move |_| day.set(id)
                },
                "Pick"
            }
        }
    }
}
