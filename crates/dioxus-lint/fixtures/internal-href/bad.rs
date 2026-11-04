#[component]
fn Settings() -> Element {
    rsx! {
        // next line flagged
        a { class: "inline-flex", href: "/admin/audit", "View audit log" }
    }
}
