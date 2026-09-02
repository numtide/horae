use dioxus::prelude::*;

use crate::components::avatar::{Avatar, Chip};
use crate::components::badge::Badge;
use crate::components::button::{Button, IconButton, SplitButton};
use crate::components::card::{Card, MetricCard};
use crate::components::combobox::{ComboOption, Combobox};
use crate::components::controls::{Checkbox, Radio, Segmented, Toggle};
use crate::components::date_picker::DatePicker;
use crate::components::form::{FormGroup, Input, Select, Textarea};
use crate::components::menu::{Menu, MenuDivider, MenuItem};
use crate::components::nav::NavItem;
use crate::components::table::DataTable;
use crate::components::toast::Toast;

/// A living gallery of the Horae component kit, mirroring the design system's
/// "Components" sheet. Also serves as a smoke test that every component renders.
#[component]
pub fn Gallery() -> Element {
    let mut segment = use_signal(|| "Week".to_string());
    let mut billable = use_signal(|| true);
    let mut agreed = use_signal(|| false);
    let mut plan = use_signal(|| "Manager".to_string());
    let mut combo = use_signal(String::new);
    let mut picked = use_signal(|| chrono::Utc::now().date_naive());

    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "Component Library" }
            }

            // ── Buttons ──────────────────────────────────────────────────
            section { class: "gallery-section",
                h2 { class: "gallery-heading", "Buttons" }
                div { class: "gallery-row",
                    Button { variant: "primary", "Primary" }
                    Button { variant: "solid", "Submit week" }
                    Button { variant: "secondary", "Secondary" }
                    Button { variant: "accent", "Send invoice" }
                    Button { variant: "danger", "Delete" }
                    Button { variant: "ghost", "Ghost" }
                }
                div { class: "gallery-row",
                    Button { variant: "primary", size: "sm", "Small" }
                    Button { variant: "primary", disabled: true, "Disabled" }
                    IconButton { label: "Start timer", "▶" }
                    SplitButton { label: "Generate PDF" }
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
                    Menu { label: "Actions",
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
                DataTable {
                    table {
                        thead {
                            tr {
                                th { "Teammate" }
                                th { "Project" }
                                th { class: "text-right", "Hours" }
                                th { "Status" }
                                th { class: "text-right", "Action" }
                            }
                        }
                        tbody {
                            tr {
                                td { Chip { label: "Lars Ericsson" } }
                                td { "Acme redesign" }
                                td { class: "text-mono text-right", "12.0" }
                                td { Badge { variant: "success", "Approved" } }
                                td { class: "text-right",
                                    Button { variant: "ghost", size: "sm", "Reopen" }
                                }
                            }
                            tr {
                                td { Chip { label: "Casey Rivera" } }
                                td { "Globex API" }
                                td { class: "text-mono text-right", "6.5" }
                                td { Badge { variant: "warning", "Awaiting" } }
                                td { class: "text-right",
                                    Button { variant: "primary", size: "sm", "Approve" }
                                }
                            }
                        }
                    }
                }
            }

            // ── Toast & empty state ──────────────────────────────────────
            section { class: "gallery-section",
                h2 { class: "gallery-heading", "Toast & empty state" }
                div { class: "gallery-row",
                    Toast { message: "Invoice sent to Acme.", variant: "success", icon: "✓" }
                    Toast { message: "Timer still running.", variant: "warning", icon: "⏱" }
                    Toast { message: "Draft saved.", dismissible: true }
                }
                div { class: "empty-state max-w-md",
                    div { class: "empty-state-icon", "🗂" }
                    div { class: "empty-state-title", "No time entries yet" }
                    p { class: "empty-state-text", "Start a timer or add an entry to see it here." }
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
