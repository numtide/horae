#[component]
fn Settings() -> Element {
    rsx! {
        Link { to: Route::PermissionAudit {}, "View audit log" }
        a { href: "/api/reports/export/csv", "Export CSV" }
        a { href: "/clients", target: "_blank", rel: "noopener", "Clients in a new tab" }
        a { href: "https://example.com", "External" }
    }
}
