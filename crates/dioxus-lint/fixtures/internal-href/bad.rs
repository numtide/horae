#[component]
fn Settings() -> Element {
    rsx! {
        // next line flagged
        a { class: "inline-flex", href: "/admin/audit", "View audit log" }
    }
}

fn Audit() -> Element {
    dioxus::prelude::rsx! {
        // next line flagged
        a { href: "/admin/audit", "Audit" }
    }
}
