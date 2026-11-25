use dioxus::prelude::*;

use crate::components::avatar::{Avatar, Chip};
use crate::components::badge::Badge;
use crate::components::button::{Button, ButtonVariant, IconButton, SplitButton};
use crate::components::card::{Card, MetricCard};
use crate::components::combobox::{ComboOption, Combobox};
use crate::components::controls::{Checkbox, Radio, Segmented, Toggle};
use crate::components::date_picker::DatePicker;
use crate::components::form::{FormGroup, Input, Select, Textarea};
use crate::components::menu::{Menu, MenuDivider, MenuItem, RowActions};
use crate::components::nav::NavItem;
use crate::components::page_header::PageHeader;
use crate::components::screen_state::{
    EmptyState, ErrorState, LOAD_FAILED, Loading, NoAccess, Skeleton,
};
use crate::components::table::{BulkBar, DataTable, NameCell, TableMessage, TableSkeleton};
use crate::components::toast::Toast;

/// Number, client, issued, amount, status and its badge variant.
const GALLERY_INVOICES: [(&str, &str, &str, &str, &str, &str); 3] = [
    (
        "INV-0043",
        "Acme Corp",
        "18 Nov 2026",
        "EUR 1,235.00",
        "Sent",
        "info",
    ),
    (
        "INV-0042",
        "TechStart",
        "12 Nov 2026",
        "EUR 310.00",
        "Overdue",
        "danger",
    ),
    (
        "INV-0041",
        "Northwind",
        "2 Nov 2026",
        "EUR 4,000.00",
        "Paid",
        "success",
    ),
];

const GALLERY_CLIENTS: [(&str, &str); 4] = [
    ("Acme Corp", "128:30"),
    ("TechStart", "71:15"),
    ("Northwind", "40:00"),
    ("Globex", "12:00"),
];

/// A living gallery of the Horae component kit, mirroring the design system's
/// "Components" sheet. Also serves as a smoke test that every component renders.
#[component]
pub fn Gallery() -> Element {
    let noop = use_callback(|_| {});
    let noop_click = use_callback(|_: MouseEvent| {});
    let mut segment = use_signal(|| "Week".to_string());
    let mut billable = use_signal(|| true);
    let mut agreed = use_signal(|| false);
    let mut plan = use_signal(|| "Manager".to_string());
    let mut combo = use_signal(String::new);
    let mut picked = use_signal(crate::components::today::today);

    rsx! {
        div {
            PageHeader { title: "Component Library" }

            // ── Buttons ──────────────────────────────────────────────────
            section { class: "gallery-section",
                h2 { class: "gallery-heading", "Buttons" }
                div { class: "gallery-row",
                    Button { label: "Primary" }
                    Button { variant: ButtonVariant::Secondary, label: "Secondary" }
                    Button { variant: ButtonVariant::Accent, label: "Send invoice" }
                    Button { variant: ButtonVariant::Danger, label: "Delete…" }
                    Button { variant: ButtonVariant::DangerSolid, label: "Delete" }
                    Button { variant: ButtonVariant::Ghost, label: "Ghost" }
                }
                div { class: "gallery-row",
                    Button { small: true, label: "Small" }
                    Button { variant: ButtonVariant::Secondary, small: true, label: "Small" }
                    Button { disabled: true, label: "Disabled" }
                    Button { variant: ButtonVariant::Secondary, label: "Start timer",
                        span { aria_hidden: "true", "▶" }
                    }
                    IconButton { label: "Start timer", "▶" }
                    SplitButton { label: "Submit week" }
                }
            }

            // ── Status pills ─────────────────────────────────────────────
            section { class: "gallery-section",
                h2 { class: "gallery-heading", "Status pills" }
                div { class: "gallery-row",
                    Badge { variant: "success", "Approved" }
                    Badge { variant: "info", "Synced" }
                    Badge { variant: "warning", "Awaiting" }
                    Badge { variant: "danger", "Overdue" }
                    Badge { variant: "neutral", "Draft" }
                }
            }

            // ── Inputs & fields ──────────────────────────────────────────
            section { class: "gallery-section",
                h2 { class: "gallery-heading", "Inputs & fields" }
                div { class: "gallery-row",
                    div { class: "gallery-field",
                        FormGroup { label: "Default", hint: "A short helper line.",
                            Input { placeholder: "casey@example.com" }
                        }
                    }
                    div { class: "gallery-field",
                        FormGroup { label: "Numeric",
                            Input { kind: "number", value: "128" }
                        }
                    }
                    div { class: "gallery-field",
                        FormGroup { label: "Read only",
                            Input { value: "INV-2026-0007", readonly: true }
                        }
                    }
                    div { class: "gallery-field",
                        FormGroup { label: "Disabled",
                            Input { placeholder: "Unavailable", disabled: true }
                        }
                    }
                    div { class: "gallery-field",
                        FormGroup { label: "Dropdown",
                            Select {
                                selected: plan(),
                                options: vec![
                                    ("Member".to_string(), "Member".to_string()),
                                    ("Manager".to_string(), "Manager".to_string()),
                                    ("Admin".to_string(), "Admin".to_string()),
                                ],
                                onchange: move |e: FormEvent| plan.set(e.value()),
                            }
                        }
                    }
                }
                div { class: "max-w-md",
                    FormGroup { label: "Notes",
                        Textarea { placeholder: "Kickoff call with Acme…" }
                    }
                }
            }

            // ── Toggles & segments ───────────────────────────────────────
            section { class: "gallery-section",
                h2 { class: "gallery-heading", "Toggles & segments" }
                div { class: "gallery-row",
                    Segmented {
                        items: vec!["Day".to_string(), "Week".to_string(), "Calendar".to_string()],
                        active: segment(),
                        onselect: move |v| segment.set(v),
                    }
                    Toggle {
                        on: billable(),
                        label: "Billable",
                        onclick: move |_| billable.set(!billable()),
                    }
                }
                div { class: "gallery-row",
                    Checkbox {
                        checked: agreed(),
                        label: "I agree",
                        onclick: move |_| agreed.set(!agreed()),
                    }
                    Radio { selected: true, label: "Solid" }
                    Radio { selected: false, label: "Split" }
                }
            }

            // ── Dropdown menu ────────────────────────────────────────────
            section { class: "gallery-section",
                h2 { class: "gallery-heading", "Dropdown menu" }
                div { class: "gallery-row",
                    Menu { id: "gallery-actions-menu", label: "Actions",
                        MenuItem { onclick: move |_| {}, "Edit" }
                        MenuItem { selected: true, onclick: move |_| {}, "Pin" }
                        MenuDivider {}
                        MenuItem { onclick: move |_| {}, "Archive" }
                        MenuItem { danger: true, onclick: move |_| {}, "Delete" }
                        MenuItem { disabled: true, "Unavailable" }
                    }
                    Combobox {
                        options: vec![
                            ComboOption::grouped("1", "Numtide", "Active clients"),
                            ComboOption::grouped("2", "Accur8 Software", "Active clients"),
                            ComboOption::grouped("3", "Golem SBB", "Archived clients"),
                        ],
                        value: combo(),
                        placeholder: "Filter by client",
                        all_label: "All clients",
                        onselect: move |v| combo.set(v),
                    }
                }
            }

            // ── Date picker ──────────────────────────────────────────────
            section { class: "gallery-section",
                h2 { class: "gallery-heading", "Date picker" }
                div { class: "gallery-row",
                    DatePicker {
                        selected: picked(),
                        week: true,
                        onpick: move |d| picked.set(d),
                    }
                    DatePicker {
                        selected: picked(),
                        onpick: move |d| picked.set(d),
                    }
                }
            }

            // ── Nav item ─────────────────────────────────────────────────
            section { class: "gallery-section",
                h2 { class: "gallery-heading", "Nav item" }
                div { class: "flex flex-col gap-1 gallery-navcol",
                    NavItem { icon: "◷", label: "Timesheet", active: true }
                    NavItem { icon: "▤", label: "Approvals" }
                    NavItem { icon: "◑", label: "Reports" }
                }
            }

            // ── Avatar & chips ───────────────────────────────────────────
            section { class: "gallery-section",
                h2 { class: "gallery-heading", "Avatar & chips" }
                div { class: "gallery-row",
                    Avatar { initials: "LE", size: "sm" }
                    Avatar { initials: "LE" }
                    Avatar { initials: "LE", size: "lg" }
                    Avatar { initials: "", empty: true }
                    Chip { label: "Lars Ericsson" }
                    Chip { label: "Casey Rivera" }
                    Chip { label: "Time & Materials", plain: true }
                    Chip { label: "Manager", variant: "success" }
                }
            }

            // ── Cards ────────────────────────────────────────────────────
            section { class: "gallery-section",
                h2 { class: "gallery-heading", "Cards" }
                div { class: "gallery-row",
                    MetricCard { label: "Hours this week", value: "128.5", delta: "+12%", direction: "up" }
                    MetricCard { label: "Unbilled", value: "$8,240", delta: "-3%", direction: "down" }
                    Card { title: "Frontend engineering",
                        p { class: "text-muted text-sm", "Time & materials · EUR" }
                    }
                }
            }

            // ── Table & rows ─────────────────────────────────────────────
            section { class: "gallery-section",
                h2 { class: "gallery-heading", "Table & rows" }
                p { class: "gallery-label", "Ruled (default): bold name with a secondary line, mono numbers, a badge, ⋮ for row actions" }
                DataTable { label: "Invoices",
                    table { class: "name-table", aria_label: "Invoices",
                        thead {
                            tr {
                                th { scope: "col", "Number" }
                                th { scope: "col", "Issued" }
                                th { scope: "col", class: "text-right", "Amount" }
                                th { scope: "col", "Status" }
                                th { scope: "col", aria_label: "Actions" }
                            }
                        }
                        tbody {
                            for (number, client, issued, amount, status, variant) in GALLERY_INVOICES {
                                tr { key: "{number}",
                                    NameCell { secondary: rsx! { "{client}" }, "{number}" }
                                    td { class: "text-mono text-secondary whitespace-nowrap", "{issued}" }
                                    td { class: "text-mono text-right whitespace-nowrap", "{amount}" }
                                    td { Badge { variant, "{status}" } }
                                    td { class: "text-right",
                                        RowActions { id: "gallery-row-{number}", name: number.to_string(),
                                            MenuItem { onclick: noop_click, "Edit" }
                                            MenuDivider {}
                                            MenuItem { danger: true, onclick: noop_click, "Delete" }
                                        }
                                    }
                                }
                            }
                        }
                        tfoot {
                            tr {
                                th { scope: "row", colspan: "2", "Total" }
                                td { class: "text-mono text-right whitespace-nowrap", "EUR 5,545.00" }
                                td { colspan: "2" }
                            }
                        }
                    }
                }
                p { class: "gallery-label", "Striped, with a selected row, sticky header and the bulk bar (Clients, Tasks, People, Approvals)" }
                DataTable { label: "Clients", striped: true, sticky_header: true,
                    table { aria_label: "Clients",
                        thead {
                            tr {
                                th { scope: "col", "Client" }
                                th { scope: "col", class: "text-right", "Hours" }
                            }
                        }
                        tbody {
                            for (index, (client, hours)) in GALLERY_CLIENTS.iter().enumerate() {
                                tr { key: "{client}", class: if index == 1 { "selected" } else { "" },
                                    NameCell { "{client}" }
                                    td { class: "text-mono text-right", "{hours}" }
                                }
                            }
                        }
                    }
                }
                div { class: "gallery-bulk",
                    BulkBar { count: 1, on_clear: noop,
                        Button { variant: ButtonVariant::Secondary, small: true, label: "Archive" }
                        Button { variant: ButtonVariant::Secondary, small: true, label: "Export" }
                    }
                }
                p { class: "gallery-label", "Compact (detailed reports, importer errors, invoice lines), busy while reloading" }
                DataTable { label: "Invoice lines", compact: true, busy: true,
                    table { aria_label: "Invoice lines",
                        thead {
                            tr {
                                th { scope: "col", "Description" }
                                th { scope: "col", class: "text-right", "Hours" }
                                th { scope: "col", class: "text-right", "Amount" }
                            }
                        }
                        tbody {
                            for (client, hours) in GALLERY_CLIENTS {
                                tr { key: "{client}",
                                    td { "{client}" }
                                    td { class: "text-mono text-right", "{hours}" }
                                    td { class: "text-mono text-right", "EUR 100.00" }
                                }
                            }
                        }
                    }
                }
                p { class: "gallery-label", "Flush inside a card" }
                Card { title: "History", flush: true,
                    DataTable { flush: true,
                        table {
                            thead { tr { th { scope: "col", "When" } th { scope: "col", "Result" } } }
                            tbody {
                                tr { td { class: "text-mono text-secondary", "21 Nov 18:04" } td { Badge { variant: "success", "Done" } } }
                                tr { td { class: "text-mono text-secondary", "19 Nov 09:12" } td { Badge { variant: "warning", "3 errors" } } }
                            }
                        }
                    }
                }
                p { class: "gallery-label", "States inside the frame: loading, empty or no match, error" }
                div { class: "gallery-row items-start",
                    for state in ["loading", "empty", "error"] {
                        div { key: "{state}", class: "flex-1",
                            DataTable {
                                table {
                                    thead { tr { th { scope: "col", "Client" } th { scope: "col", class: "text-right", "Hours" } } }
                                    tbody {
                                        match state {
                                            "loading" => rsx! { TableSkeleton { columns: 2, label: "Loading clients…" } },
                                            "empty" => rsx! {
                                                TableMessage { columns: 2,
                                                    EmptyState { icon: "clients", inset: true, title: "No clients match «nort»",
                                                        body: "Change the search or the filters.",
                                                        actions: rsx! { Button { variant: ButtonVariant::Secondary, small: true, label: "Clear filters" } },
                                                    }
                                                }
                                            },
                                            _ => rsx! {
                                                TableMessage { columns: 2,
                                                    ErrorState { inset: true, title: "Could not load clients", body: LOAD_FAILED, onretry: noop }
                                                }
                                            },
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ── Toast ────────────────────────────────────────────────────
            section { class: "gallery-section",
                h2 { class: "gallery-heading", "Toast" }
                div { class: "gallery-row",
                    Toast { message: "Invoice sent to Acme.", variant: "success", icon: "✓" }
                    Toast { message: "Timer still running.", variant: "warning", icon: "⏱" }
                    Toast { message: "Draft saved.", dismissible: true }
                }
            }

            // ── Empty, error and loading states ──────────────────────────
            section { class: "gallery-section",
                h2 { class: "gallery-heading", "Empty, error and loading states" }
                div { class: "grid gap-4 lg:grid-cols-2",
                    EmptyState { icon: "timesheet", title: "No time entries yet",
                        body: "Start a timer or add an entry to see it here.",
                        actions: rsx! { button { r#type: "button", class: "btn btn-primary", "Add entry" } },
                    }
                    EmptyState { icon: "approvals", title: "Nothing to approve", success: true,
                        body: "No timesheets are waiting for approval this week.",
                    }
                    ErrorState { title: "Could not load projects", body: LOAD_FAILED,
                        request: "list_projects · 503", onretry: noop,
                    }
                    NoAccess {}
                }
                div { class: "flex flex-col gap-4 mt-4",
                    Loading { label: "Loading projects…" }
                    Skeleton { rows: 4, columns: 5, label: "Loading projects…" }
                }
            }

            section { class: "gallery-section",
                h2 { class: "gallery-heading", "Inline banners" }
                div { class: "flex flex-col gap-3 max-w-lg",
                    div { class: "banner banner-info",
                        span { class: "banner-icon", "ⓘ" }
                        div { class: "banner-body",
                            div { class: "banner-title", "Read-only connection" }
                            div { class: "banner-detail", "Horae only ever reads from Harvest — nothing is written back." }
                        }
                    }
                    div { class: "banner banner-warning",
                        span { class: "banner-icon", "⚠" }
                        div { class: "banner-body",
                            div { class: "banner-title", "Submit by Sunday 23:00" }
                            div { class: "banner-detail", "Timesheets not submitted for approval are flagged to your manager." }
                        }
                        div { class: "banner-action", button { class: "btn btn-secondary btn-sm", "Dismiss" } }
                    }
                    div { class: "banner banner-success",
                        span { class: "banner-icon", "✓" }
                        div { class: "banner-body",
                            div { class: "banner-title", "Import committed" }
                            div { class: "banner-detail", "6,940 entries added to Fieldstone Studio." }
                        }
                    }
                    div { class: "banner banner-danger",
                        span { class: "banner-icon", "✕" }
                        div { class: "banner-body",
                            div { class: "banner-title", "Sync failed" }
                            div { class: "banner-detail", "Check the Harvest token and try again." }
                        }
                    }
                }
            }

            section { class: "gallery-section",
                h2 { class: "gallery-heading", "Collapsible" }
                div { class: "collapse open max-w-md",
                    button { class: "collapse-head",
                        span { class: "collapse-caret", "›" }
                        span { class: "collapse-label", "Engineering" }
                        span { class: "collapse-count", "12" }
                    }
                    div { class: "collapse-body text-sm text-secondary", "Twelve entries grouped under this client." }
                }
            }

            section { class: "gallery-section",
                h2 { class: "gallery-heading", "Dropzone & files" }
                div { class: "flex flex-col gap-3 max-w-md",
                    div { class: "dropzone",
                        div { class: "text-2xl", "↥" }
                        div { class: "text-sm", "Drop a CSV here or click to choose" }
                        div { class: "text-xs text-faint", "Detailed time report · up to 20 MB" }
                    }
                    div { class: "file-chip",
                        span { class: "file-chip-icon", "▤" }
                        span { "harvest-export.csv" }
                        span { class: "file-chip-size", "1.2 MB" }
                        button { class: "file-chip-remove", "✕" }
                    }
                }
            }

            section { class: "gallery-section",
                h2 { class: "gallery-heading", "Integration" }
                div { class: "integration max-w-md",
                    div { class: "integration-logo harvest", "H" }
                    div { class: "integration-body",
                        div { class: "integration-name", "Harvest" }
                        div { class: "integration-meta", "Connected · read-only" }
                    }
                    span { class: "badge badge-success", "Active" }
                    button { class: "btn btn-secondary btn-sm", "Manage" }
                }
            }

            section { class: "gallery-section",
                h2 { class: "gallery-heading", "Input group & chip input" }
                div { class: "flex flex-col gap-3 max-w-md",
                    div { class: "input-group",
                        span { class: "input-group-addon prefix", "horae.app/" }
                        input { class: "input-group-field", value: "fieldstone" }
                    }
                    div { class: "input-group",
                        input { class: "input-group-field", value: "95.00" }
                        span { class: "input-group-addon suffix", "EUR / h" }
                    }
                    div { class: "chip-input",
                        span { class: "chip",
                            "design"
                            button { class: "chip-input-x", "✕" }
                        }
                        span { class: "chip",
                            "billable"
                            button { class: "chip-input-x", "✕" }
                        }
                        input { class: "chip-input-field", placeholder: "Add tag…" }
                    }
                }
            }

            section { class: "gallery-section",
                h2 { class: "gallery-heading", "Counter tile" }
                div { class: "counter-tile max-w-md",
                    div { class: "counter-total", "6,940" }
                    div { class: "counter-total-label", "Records imported" }
                    div { class: "counter-breakdown",
                        div { class: "counter-stat created",
                            div { class: "counter-stat-value", "6,904" }
                            div { class: "counter-stat-label", "Created" }
                        }
                        div { class: "counter-stat",
                            div { class: "counter-stat-value", "24" }
                            div { class: "counter-stat-label", "Updated" }
                        }
                        div { class: "counter-stat",
                            div { class: "counter-stat-value", "12" }
                            div { class: "counter-stat-label", "Skipped" }
                        }
                        div { class: "counter-stat errored",
                            div { class: "counter-stat-value", "0" }
                            div { class: "counter-stat-label", "Errored" }
                        }
                    }
                }
            }
        }
    }
}
