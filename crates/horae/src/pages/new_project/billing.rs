use dioxus::prelude::*;
use horae_core::money::{format_cents_plain, parse_cents};
use horae_core::project::{BudgetMode, MonthlyFeeDay, RateMode};
use horae_core::types::ProjectType;
use uuid::Uuid;

use crate::components::controls::Checkbox;
use crate::components::form::{FormGroup, Input, Select};
use crate::components::icons::NavIcon;
use crate::models::project_creation::{CreationOptions, FeeMode, MilestoneInput, ProjectForm};

use super::FormRow;

#[component]
pub(super) fn Billing(mut form: Signal<ProjectForm>, options: Signal<CreationOptions>) -> Element {
    let project_type = form.read().project_type;
    let currency = form.read().currency.clone().or_else(|| {
        options
            .read()
            .clients
            .iter()
            .find(|client| Some(client.id) == form.read().client_id)
            .map(|client| client.currency.clone())
    });
    let currency_label = currency.unwrap_or_else(|| "Select a currency".into());
    rsx! {
        FormRow { label: "Project type", hint: "How this project bills",
            div { class: "np-types grid grid-cols-3 gap-3", role: "group", aria_label: "Project type",
                for (kind, icon, hint) in [
                    (ProjectType::TimeAndMaterials, "time", "Bill by the hour at your team's billable rates."),
                    (ProjectType::FixedFee, "invoices", "Bill a set fee, in one go or across a schedule."),
                    (ProjectType::NonBillable, "nonbillable", "Track internal work without billing a client."),
                ] {
                    label { class: if project_type == kind { "np-type relative border rounded-xl p-4 bg-choice-selected cursor-pointer" } else { "np-type relative border rounded-xl p-4 bg-secondary cursor-pointer" },
                        span { class: "flex items-center gap-3 text-sm font-semibold text-strong",
                            input { class: "absolute inset-0 w-full h-full opacity-0 m-0 cursor-pointer", r#type: "radio", name: "np-project-type", value: kind.to_string(), checked: project_type == kind,
                                onchange: move |_| {
                                    let mut data = form.write();
                                    data.project_type = kind;
                                    if data.budget_mode.validate_for(kind).is_err() { data.budget_mode = BudgetMode::None; }
                                },
                            }
                            span { "data-project-type-icon": icon, aria_hidden: "true",
                                class: if project_type == kind { "size-8 flex-none inline-flex items-center justify-center rounded-btn bg-pine text-on-pine" } else { "size-8 flex-none inline-flex items-center justify-center rounded-btn bg-tertiary text-subtle" },
                                NavIcon { name: icon }
                            }
                            "{kind.label()}"
                        }
                        span { class: "block text-xs text-subtle mt-2", "{hint}" }
                    }
                }
            }
            div { class: "bg-secondary border rounded-xl p-5 mt-3 flex flex-col gap-5",
                if project_type == ProjectType::TimeAndMaterials {
                    fieldset { class: "border-0 p-0 m-0",
                        legend { class: "text-xs uppercase tracking-wide text-faint mb-2", "Billable rates" }
                        div { class: "flex flex-col gap-2",
                            for (mode, title, hint) in [
                                (RateMode::Person, "Person hourly rate", "Use each person's project override or profile rate."),
                                (RateMode::Task, "Task hourly rate", "Use the rate set for each task on this project."),
                                (RateMode::Project, "Project hourly rate", "Apply one rate to all billable work on this project."),
                            ] {
                                div { class: "np-option flex flex-wrap items-center gap-3 p-3 bg-base border border-input rounded-btn",
                                    label { class: "flex flex-1 basis-form-select min-w-0 items-center gap-3 cursor-pointer",
                                        input { class: "choice-box radio size-4 m-0", r#type: "radio", name: "np-rate-mode", checked: form.read().rate_mode == mode, onchange: move |_| form.write().rate_mode = mode }
                                        span { span { class: "block text-sm", "{title}" } span { class: "block text-xs text-subtle mt-1", "{hint}" } }
                                    }
                                    if mode == RateMode::Project && form.read().rate_mode == mode {
                                        div { class: "flex items-center gap-2 max-w-full",
                                            span { class: "font-mono text-sm text-subtle", "{currency_label}" }
                                            Input { id: "np-project-rate", label: "Hourly rate ({currency_label})", class: "w-30 max-w-full font-mono text-right", value: form.read().project_rate.clone(), oninput: move |event: FormEvent| form.write().project_rate = event.value() }
                                            span { class: "text-xs text-subtle whitespace-nowrap", "/ h" }
                                        }
                                    }
                                }
                            }
                            if form.read().rate_mode == RateMode::Project {
                                p { class: "form-hint m-0", "Required. Zero is an explicit rate, not a missing rate." }
                            }
                        }
                    }
                }
                if project_type == ProjectType::FixedFee {
                    FeeSchedule { form, currency: currency_label.clone() }
                }
                Budget { form, currency: currency_label, email_available: options.read().email_available }
                if project_type == ProjectType::NonBillable {
                    p { class: "text-xs text-subtle m-0", "Internal work, R&D, sales. This project's time is non-billable and cannot be invoiced." }
                }
            }
        }
    }
}

#[component]
fn Budget(mut form: Signal<ProjectForm>, currency: String, email_available: bool) -> Element {
    let mode = form.read().budget_mode;
    let choices: Vec<_> = [
        (BudgetMode::None, "none", "No budget"),
        (BudgetMode::TotalHours, "total_hours", "Total project hours"),
        (BudgetMode::TotalFees, "total_fees", "Total project fees"),
        (BudgetMode::HoursPerTask, "hours_per_task", "Hours per task"),
        (BudgetMode::FeesPerTask, "fees_per_task", "Fees per task"),
        (
            BudgetMode::HoursPerPerson,
            "hours_per_person",
            "Hours per person",
        ),
    ]
    .into_iter()
    .filter(|(mode, _, _)| mode.validate_for(form.read().project_type).is_ok())
    .collect();
    let selected = choices
        .iter()
        .find(|(candidate, _, _)| *candidate == mode)
        .map(|(_, key, _)| *key)
        .unwrap_or("none");
    let select_options = choices
        .iter()
        .map(|(_, key, title)| (key.to_string(), title.to_string()))
        .collect();
    let amount_label = if mode == BudgetMode::TotalFees {
        format!("Budget ({currency})")
    } else {
        "Budget hours".into()
    };
    rsx! {
        div {
            label { class: "block text-xs uppercase tracking-wide text-faint mb-2", r#for: "np-budget-mode", "Budget" }
            div { class: "flex flex-wrap items-center gap-3",
                div { class: "w-form-select max-w-full",
                    Select { id: "np-budget-mode", options: select_options, selected,
                        onchange: move |event: FormEvent| {
                            if let Some((mode, _, _)) = choices.iter().find(|(_, key, _)| *key == event.value()) { form.write().budget_mode = *mode; }
                        }
                    }
                }
                if matches!(mode, BudgetMode::TotalHours | BudgetMode::TotalFees) {
                    div { class: "flex items-center gap-2 max-w-full",
                        if mode == BudgetMode::TotalFees { span { class: "font-mono text-sm text-subtle", "{currency}" } }
                        Input { id: "np-budget-value", label: amount_label, class: "w-40 max-w-full font-mono text-right", value: form.read().budget_value.clone(), oninput: move |event: FormEvent| form.write().budget_value = event.value() }
                        span { class: "text-xs text-subtle", if mode == BudgetMode::TotalFees { "total" } else { "hours" } }
                    }
                }
            }
            if matches!(mode, BudgetMode::HoursPerTask | BudgetMode::FeesPerTask) { p { class: "form-hint", "Enter each task's budget in the Tasks section below." } }
            if mode == BudgetMode::HoursPerPerson { p { class: "form-hint", "Enter each person's budget in the Team section below." } }
            if mode != BudgetMode::None {
                div { class: "flex flex-col gap-3 mt-4",
                    div { class: "flex flex-wrap items-center gap-3",
                        Checkbox { checked: form.read().budget_alert, label: "Email me and project managers when the budget passes the threshold", disabled: !email_available && !form.read().budget_alert,
                            onclick: move |_| { let value = !form.read().budget_alert; form.write().budget_alert = value; }
                        }
                        if form.read().budget_alert {
                            div { class: "flex items-center gap-2",
                                Input { id: "np-alert-threshold", label: "Budget email threshold (%)", class: "w-16 font-mono text-right", disabled: !email_available, value: form.read().budget_alert_at.clone(), oninput: move |event: FormEvent| form.write().budget_alert_at = event.value() }
                                span { class: "text-sm text-subtle", "%" }
                            }
                        }
                    }
                    if !email_available { p { class: "form-hint m-0", "Email delivery is not configured. Ask an administrator to configure it before enabling budget emails." } }
                    Checkbox { checked: form.read().budget_monthly, label: "Budget resets every month", onclick: move |_| { let value = !form.read().budget_monthly; form.write().budget_monthly = value; } }
                    if form.read().project_type != ProjectType::NonBillable {
                        Checkbox { checked: form.read().budget_nonbillable, label: "Include non-billable time in the budget", onclick: move |_| { let value = !form.read().budget_nonbillable; form.write().budget_nonbillable = value; } }
                    }
                }
            }
        }
    }
}

#[component]
fn FeeSchedule(mut form: Signal<ProjectForm>, currency: String) -> Element {
    let mode = form.read().fee_mode;
    let total = form
        .read()
        .milestones
        .iter()
        .try_fold(0_i64, |total, item| {
            let amount = parse_cents(&item.amount)
                .ok()
                .filter(|amount| *amount >= 0)?;
            total.checked_add(amount)
        });
    rsx! {
        div {
            fieldset { class: "border-0 p-0 m-0",
                legend { class: "text-xs uppercase tracking-wide text-faint mb-2", "Project fee" }
                div { class: "segmented flex-wrap",
                    for (choice, label) in [(FeeMode::Single, "Single fee"), (FeeMode::Milestones, "Milestones"), (FeeMode::Monthly, "Monthly")] {
                        label { class: if mode == choice { "segmented-item active inline-flex items-center gap-2 cursor-pointer" } else { "segmented-item inline-flex items-center gap-2 cursor-pointer" },
                            input { r#type: "radio", name: "np-fee-mode", checked: mode == choice, onchange: move |_| form.write().fee_mode = choice }
                            "{label}"
                        }
                    }
                }
            }
            if mode == FeeMode::Milestones {
                div { class: "bg-base border rounded-btn mt-4",
                    for milestone in form.read().milestones.clone() {
                        Milestone { key: "{milestone.id}", form, id: milestone.id, currency: currency.clone() }
                    }
                    div { class: "flex flex-wrap items-center justify-between gap-3 p-3",
                        button { class: "btn btn-ghost btn-sm", r#type: "button", disabled: form.read().milestones.len() >= 100,
                            onclick: move |_| form.write().milestones.push(MilestoneInput { id: Uuid::now_v7(), name: String::new(), due_on: String::new(), amount: String::new() }), "Add milestone"
                        }
                        span { class: "text-xs text-subtle", "Total " span { class: "font-mono text-default", if let Some(total) = total { "{currency} {format_cents_plain(total)}" } else { "Enter valid amounts" } } }
                    }
                }
            } else {
                div { class: "flex flex-wrap items-center gap-2 mt-4",
                    div { class: "flex items-center gap-2 max-w-full",
                        span { class: "font-mono text-sm text-subtle", "{currency}" }
                        Input { id: "np-fee-amount", label: "Fee ({currency})", class: "w-50 max-w-full font-mono text-right", value: form.read().fee_amount.clone(), oninput: move |event: FormEvent| form.write().fee_amount = event.value() }
                    }
                    if mode == FeeMode::Monthly {
                        label { class: "text-xs text-subtle", r#for: "np-monthly-day", "per month, available to invoice on" }
                        div { class: "w-50 max-w-full",
                            Select { id: "np-monthly-day", options: vec![("first".into(), "1st of the month".into()), ("fifteenth".into(), "15th of the month".into()), ("last".into(), "Last day of the month".into())],
                                selected: match form.read().monthly_day { MonthlyFeeDay::First => "first", MonthlyFeeDay::Fifteenth => "fifteenth", MonthlyFeeDay::Last => "last" },
                                onchange: move |event: FormEvent| form.write().monthly_day = match event.value().as_str() { "fifteenth" => MonthlyFeeDay::Fifteenth, "last" => MonthlyFeeDay::Last, _ => MonthlyFeeDay::First }
                            }
                        }
                    }
                }
            }
            p { class: "form-hint", "Fees are available when preparing an invoice. Creating this project does not issue or send invoices." }
        }
    }
}

#[component]
fn Milestone(mut form: Signal<ProjectForm>, id: Uuid, currency: String) -> Element {
    let Some(item) = form
        .read()
        .milestones
        .iter()
        .find(|item| item.id == id)
        .cloned()
    else {
        return rsx! {};
    };
    rsx! {
        div { class: "grid md:grid-cols-3 gap-3 p-3 border-b",
            FormGroup { label: "Milestone name", id: "np-milestone-name-{id}",
                Input { id: "np-milestone-name-{id}", value: item.name.clone(), oninput: move |event: FormEvent| { if let Some(item) = form.write().milestones.iter_mut().find(|item| item.id == id) { item.name = event.value(); } } }
            }
            FormGroup { label: "Due date", id: "np-milestone-date-{id}",
                Input { id: "np-milestone-date-{id}", kind: "date", value: item.due_on, oninput: move |event: FormEvent| { if let Some(item) = form.write().milestones.iter_mut().find(|item| item.id == id) { item.due_on = event.value(); } } }
            }
            FormGroup { label: "Amount ({currency})", id: "np-milestone-amount-{id}",
                Input { id: "np-milestone-amount-{id}", value: item.amount, oninput: move |event: FormEvent| { if let Some(item) = form.write().milestones.iter_mut().find(|item| item.id == id) { item.amount = event.value(); } } }
            }
            button { class: "btn btn-ghost btn-sm", r#type: "button", aria_label: "Remove milestone {item.name}", onclick: move |_| form.write().milestones.retain(|item| item.id != id), "Remove milestone" }
        }
    }
}
