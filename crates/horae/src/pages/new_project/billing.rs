use dioxus::prelude::*;
use horae_core::money::{format_cents_plain, parse_cents};
use horae_core::project::{BudgetMode, MonthlyFeeDay, RateMode};
use horae_core::types::ProjectType;
use uuid::Uuid;

use crate::components::controls::Checkbox;
use crate::components::form::Input;
use crate::components::icons::NavIcon;
use crate::components::select_field::SelectField;
use crate::models::project_creation::{
    CreationOptions, FeeMode, MilestoneInput, ProjectForm, ProjectFormField,
};

use super::FormRow;

fn billing_error_id(current: Option<ProjectFormField>, field: ProjectFormField) -> Option<String> {
    (current == Some(field)).then(|| "np-billing-field-error".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_errors_target_a_control_that_can_be_corrected() {
        for field in [
            ProjectFormField::BudgetAlert,
            ProjectFormField::BudgetAlertAt,
        ] {
            let mut dom = VirtualDom::new_with_props(
                |field: ProjectFormField| {
                    let form = use_signal(|| ProjectForm {
                        budget_mode: BudgetMode::TotalHours,
                        budget_alert: true,
                        budget_alert_at: "101".into(),
                        ..Default::default()
                    });
                    rsx! { Budget {
                        form, currency: "EUR", email_available: field == ProjectFormField::BudgetAlertAt,
                        invalid_field: Some(field), error_message: Some("Correct this setting".to_owned()),
                    } }
                },
                field,
            );
            dom.rebuild_in_place();
            let html = dioxus::ssr::render(&dom);
            let id = super::super::field_id(field);
            let control = html
                .split('<')
                .find(|tag| tag.contains(&format!("id=\"{id}\"")))
                .unwrap()
                .split('>')
                .next()
                .unwrap();
            assert!(
                !control.contains("disabled"),
                "Rejected control must remain editable: {control}"
            );
            assert!(control.contains("aria-invalid=\"true\""));
            assert!(control.contains("aria-describedby=\"np-billing-field-error\""));
            assert!(html.contains("Correct this setting</p>"));
        }
    }
}

#[component]
pub(super) fn Billing(
    mut form: Signal<ProjectForm>,
    options: Signal<CreationOptions>,
    #[props(default)] invalid_field: Option<ProjectFormField>,
    #[props(default)] error_message: Option<String>,
) -> Element {
    let error_id = |field| billing_error_id(invalid_field, field);
    let error_for = |field| {
        (invalid_field == Some(field))
            .then(|| error_message.clone())
            .flatten()
    };
    let project_type = form.read().project_type;
    let legacy = form.read().rate_mode == RateMode::Legacy;
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
            fieldset { class: "np-types grid grid-cols-3 gap-3 border-0 p-0 m-0", aria_label: "Project type", disabled: legacy,
                for (kind, icon, hint) in [
                    (ProjectType::TimeAndMaterials, "time", "Bill by the hour at your team's billable rates."),
                    (ProjectType::FixedFee, "invoices", "Bill a set fee, in one go or across a schedule."),
                    (ProjectType::NonBillable, "nonbillable", "Track internal work without billing a client."),
                ] {
                    label { class: if project_type == kind { "np-type relative border rounded-xl p-4 bg-choice-selected cursor-pointer" } else { "np-type relative border rounded-xl p-4 bg-secondary cursor-pointer" },
                        span { class: "flex items-center gap-3 text-sm font-semibold text-strong",
                            input { class: "absolute inset-0 w-full h-full opacity-0 m-0 cursor-pointer", r#type: "radio", name: "np-project-type", value: kind.to_string(), checked: project_type == kind,
                                id: if kind == ProjectType::TimeAndMaterials { "np-project-type" },
                                aria_invalid: if kind == ProjectType::TimeAndMaterials { error_id(ProjectFormField::ProjectType).map(|_| "true") },
                                aria_describedby: if kind == ProjectType::TimeAndMaterials { error_id(ProjectFormField::ProjectType) },
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
            if legacy {
                p { class: "form-hint", "Current type: {project_type.label()}. This project keeps its existing billing rules; changing to a new billing configuration requires a migration." }
            }
            if let Some(message) = error_for(ProjectFormField::ProjectType) { p { id: "np-billing-field-error", class: "text-sm text-danger", "{message}" } }
            div { class: "bg-secondary border rounded-xl p-5 mt-3 flex flex-col gap-5",
                if legacy {
                    div {
                        p { id: "np-rate-mode", class: "text-sm font-semibold m-0", "Existing rate hierarchy" }
                        p { class: "form-hint", "Task override, then person override, then project rate, then profile rate. Empty inherits; zero is an explicit rate." }
                        label { class: "block text-sm mb-2", r#for: "np-project-rate", "Project hourly rate ({currency_label})" }
                        Input { id: "np-project-rate", error_id: error_id(ProjectFormField::ProjectRate), class: "w-30 max-w-full font-mono text-right", value: form.read().project_rate.clone(), placeholder: "Inherit", oninput: move |event: FormEvent| form.write().project_rate = event.value() }
                        if let Some(message) = error_for(ProjectFormField::ProjectRate) { p { id: "np-billing-field-error", class: "text-sm text-danger", "{message}" } }
                    }
                } else if project_type == ProjectType::TimeAndMaterials {
                    fieldset { class: "border-0 p-0 m-0",
                        legend { class: "text-xs uppercase tracking-eyebrow text-faint mb-2", "Billable rates" }
                        div { class: "flex flex-col gap-2",
                            for (mode, title, hint) in [
                                (RateMode::Person, "Person hourly rate", "Use each person's project override or profile rate."),
                                (RateMode::Task, "Task hourly rate", "Use the rate set for each task on this project."),
                                (RateMode::Project, "Project hourly rate", "Apply one rate to all billable work on this project."),
                            ] {
                                div { class: "np-option flex flex-wrap items-center gap-3 p-3 bg-base border border-input rounded-btn",
                                    label { class: "flex flex-1 basis-form-select min-w-0 items-center gap-3 cursor-pointer",
                                        input { class: "choice-box radio size-4 m-0", r#type: "radio", name: "np-rate-mode", checked: form.read().rate_mode == mode,
                                            id: if mode == RateMode::Person { "np-rate-mode" },
                                            aria_invalid: if mode == RateMode::Person { error_id(ProjectFormField::RateMode).map(|_| "true") },
                                            aria_describedby: if mode == RateMode::Person { error_id(ProjectFormField::RateMode) },
                                            onchange: move |_| form.write().rate_mode = mode }
                                        span { span { class: "block text-sm", "{title}" } span { class: "block text-xs text-subtle mt-1", "{hint}" } }
                                    }
                                    if mode == RateMode::Project && form.read().rate_mode == mode {
                                        div { class: "flex items-center gap-2 max-w-full",
                                            span { class: "font-mono text-sm text-subtle", "{currency_label}" }
                                            Input { id: "np-project-rate", error_id: error_id(ProjectFormField::ProjectRate), label: "Hourly rate ({currency_label})", class: "w-30 max-w-full font-mono text-right", value: form.read().project_rate.clone(), oninput: move |event: FormEvent| form.write().project_rate = event.value() }
                                            span { class: "text-xs text-subtle whitespace-nowrap", "/ h" }
                                        }
                                    }
                                }
                            }
                            if form.read().rate_mode == RateMode::Project {
                                p { class: "form-hint m-0", "Required. Zero is an explicit rate, not a missing rate." }
                                if let Some(message) = error_for(ProjectFormField::ProjectRate) { p { id: "np-billing-field-error", class: "text-sm text-danger", "{message}" } }
                            }
                            if let Some(message) = error_for(ProjectFormField::RateMode) { p { id: "np-billing-field-error", class: "text-sm text-danger", "{message}" } }
                        }
                    }
                }
                if project_type == ProjectType::FixedFee && !legacy {
                    FeeSchedule { form, currency: currency_label.clone(), invalid_field, error_message: error_message.clone() }
                }
                Budget { form, currency: currency_label, email_available: options.read().email_available, legacy, invalid_field, error_message: error_message.clone() }
                if project_type == ProjectType::NonBillable {
                    p { class: "text-xs text-subtle m-0", "Internal work, R&D, sales. This project's time is non-billable and cannot be invoiced." }
                }
            }
        }
    }
}

#[component]
fn Budget(
    mut form: Signal<ProjectForm>,
    currency: String,
    email_available: bool,
    #[props(default)] legacy: bool,
    invalid_field: Option<ProjectFormField>,
    error_message: Option<String>,
) -> Element {
    let error_id = |field| billing_error_id(invalid_field, field);
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
    .filter(|(mode, _, _)| {
        if legacy {
            matches!(
                mode,
                BudgetMode::None | BudgetMode::TotalHours | BudgetMode::TotalFees
            )
        } else {
            mode.validate_for(form.read().project_type).is_ok()
        }
    })
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
            label { class: "block text-xs uppercase tracking-eyebrow text-faint mb-2", r#for: "np-budget-mode", "Budget" }
            div { class: "flex flex-wrap items-center gap-3",
                div { class: "w-form-select max-w-full",
                    SelectField { id: "np-budget-mode", error_id: error_id(ProjectFormField::BudgetMode), label: "Budget", options: select_options, selected,
                        onselect: move |value: String| {
                            if let Some((mode, _, _)) = choices.iter().find(|(_, key, _)| *key == value) { form.write().budget_mode = *mode; }
                        }
                    }
                }
                if matches!(mode, BudgetMode::TotalHours | BudgetMode::TotalFees) {
                    div { class: "flex items-center gap-2 max-w-full",
                        if mode == BudgetMode::TotalFees { span { class: "font-mono text-sm text-subtle", "{currency}" } }
                        Input { id: "np-budget-value", error_id: error_id(ProjectFormField::BudgetValue), label: amount_label, class: "w-40 max-w-full font-mono text-right", value: form.read().budget_value.clone(), oninput: move |event: FormEvent| form.write().budget_value = event.value() }
                        span { class: "text-xs text-subtle", if mode == BudgetMode::TotalFees { "total" } else { "hours" } }
                    }
                }
            }
            if matches!(invalid_field, Some(ProjectFormField::BudgetMode | ProjectFormField::BudgetValue)) {
                if let Some(message) = &error_message { p { id: "np-billing-field-error", class: "text-sm text-danger", "{message}" } }
            }
            if matches!(mode, BudgetMode::HoursPerTask | BudgetMode::FeesPerTask) { p { class: "form-hint", "Enter each task's budget in the Tasks section below." } }
            if mode == BudgetMode::HoursPerPerson { p { class: "form-hint", "Enter each person's budget in the Team section below." } }
            if legacy && mode == BudgetMode::TotalHours {
                p { class: "form-hint", "This project's hour budget includes billable and non-billable time, without a monthly reset." }
            }
            if mode != BudgetMode::None && !legacy {
                div { class: "flex flex-col gap-3 mt-4",
                    div { class: "flex flex-wrap items-center gap-3",
                        Checkbox { id: "np-budget-alert", error_id: error_id(ProjectFormField::BudgetAlert), checked: form.read().budget_alert, label: "Email me and project managers when the budget passes the threshold", disabled: !email_available && !form.read().budget_alert,
                            onclick: move |_| { let value = !form.read().budget_alert; form.write().budget_alert = value; }
                        }
                        if form.read().budget_alert {
                            div { class: "flex items-center gap-2",
                                Input { id: "np-alert-threshold", error_id: error_id(ProjectFormField::BudgetAlertAt), label: "Budget email threshold (%)", class: "w-16 font-mono text-right", disabled: !email_available, value: form.read().budget_alert_at.clone(), oninput: move |event: FormEvent| form.write().budget_alert_at = event.value() }
                                span { class: "text-sm text-subtle", "%" }
                            }
                        }
                    }
                    if matches!(invalid_field, Some(ProjectFormField::BudgetAlert | ProjectFormField::BudgetAlertAt)) {
                        if let Some(message) = &error_message { p { id: "np-billing-field-error", class: "text-sm text-danger", "{message}" } }
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
fn FeeSchedule(
    mut form: Signal<ProjectForm>,
    currency: String,
    invalid_field: Option<ProjectFormField>,
    error_message: Option<String>,
) -> Element {
    let error_id = |field| billing_error_id(invalid_field, field);
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
                legend { class: "text-xs uppercase tracking-eyebrow text-faint mb-2", "Project fee" }
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
                div { class: "bg-base border border-light rounded-btn mt-4",
                    for milestone in form.read().milestones.clone() {
                        Milestone { key: "{milestone.id}", form, id: milestone.id, currency: currency.clone(), invalid_field, error_message: error_message.clone() }
                    }
                    div { class: "flex flex-wrap items-center justify-between gap-3 px-3 py-2.5",
                        button { id: "np-add-milestone", class: "btn btn-ghost btn-sm", r#type: "button", disabled: form.read().milestones.len() >= 100,
                            aria_invalid: error_id(ProjectFormField::Milestones).map(|_| "true"),
                            aria_describedby: error_id(ProjectFormField::Milestones),
                            onclick: move |_| form.write().milestones.push(MilestoneInput { id: Uuid::now_v7(), name: String::new(), due_on: String::new(), amount: String::new() }), "Add milestone"
                        }
                        span { class: "text-xs text-subtle", "Total " span { class: "font-mono text-default", if let Some(total) = total { "{currency} {format_cents_plain(total)}" } else { "Enter valid amounts" } } }
                    }
                }
            } else {
                div { class: "flex flex-wrap items-center gap-2 mt-4",
                    div { class: "flex items-center gap-2 max-w-full",
                        span { class: "font-mono text-sm text-subtle", "{currency}" }
                        Input { id: "np-fee-amount", error_id: error_id(ProjectFormField::FeeAmount), label: "Fee ({currency})", class: "w-50 max-w-full font-mono text-right", value: form.read().fee_amount.clone(), oninput: move |event: FormEvent| form.write().fee_amount = event.value() }
                    }
                    if mode == FeeMode::Monthly {
                        label { class: "text-xs text-subtle", r#for: "np-monthly-day", "per month, available to invoice on" }
                        div { class: "w-50 max-w-full",
                            SelectField { id: "np-monthly-day", label: "Monthly invoice day", options: vec![("first".into(), "1st of the month".into()), ("fifteenth".into(), "15th of the month".into()), ("last".into(), "Last day of the month".into())],
                                selected: match form.read().monthly_day { MonthlyFeeDay::First => "first", MonthlyFeeDay::Fifteenth => "fifteenth", MonthlyFeeDay::Last => "last" },
                                onselect: move |value: String| form.write().monthly_day = match value.as_str() { "fifteenth" => MonthlyFeeDay::Fifteenth, "last" => MonthlyFeeDay::Last, _ => MonthlyFeeDay::First }
                            }
                        }
                    }
                }
            }
            if matches!(invalid_field, Some(ProjectFormField::FeeAmount | ProjectFormField::Milestones)) {
                if let Some(message) = error_message { p { id: "np-billing-field-error", class: "text-sm text-danger", "{message}" } }
            }
            p { class: "form-hint", "Fees are available when preparing an invoice. Creating this project does not issue or send invoices." }
        }
    }
}

#[component]
fn Milestone(
    mut form: Signal<ProjectForm>,
    id: Uuid,
    currency: String,
    invalid_field: Option<ProjectFormField>,
    error_message: Option<String>,
) -> Element {
    let error_id =
        |field| (invalid_field == Some(field)).then(|| format!("np-milestone-error-{id}"));
    let row_has_error = matches!(invalid_field,
        Some(ProjectFormField::MilestoneName(row) | ProjectFormField::MilestoneDate(row) | ProjectFormField::MilestoneAmount(row)) if row == id);
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
        div { class: "np-milestone-row grid items-center gap-3 px-3 py-2.5 border-b border-light", role: "group", aria_label: if item.name.is_empty() { "New milestone".into() } else { format!("Milestone {}", item.name) },
            Input { id: "np-milestone-name-{id}", error_id: error_id(ProjectFormField::MilestoneName(id)), label: "Milestone name", placeholder: "Milestone", class: "min-w-0 px-2.5 py-2", value: item.name.clone(), oninput: move |event: FormEvent| { if let Some(item) = form.write().milestones.iter_mut().find(|item| item.id == id) { item.name = event.value(); } } }
            Input { id: "np-milestone-date-{id}", error_id: error_id(ProjectFormField::MilestoneDate(id)), label: "Due date", class: "min-w-0 px-2.5 py-2 font-mono", kind: "date", value: item.due_on, oninput: move |event: FormEvent| { if let Some(item) = form.write().milestones.iter_mut().find(|item| item.id == id) { item.due_on = event.value(); } } }
            Input { id: "np-milestone-amount-{id}", error_id: error_id(ProjectFormField::MilestoneAmount(id)), label: "Amount ({currency})", placeholder: "0.00", class: "min-w-0 px-2.5 py-2 font-mono text-right", value: item.amount, oninput: move |event: FormEvent| { if let Some(item) = form.write().milestones.iter_mut().find(|item| item.id == id) { item.amount = event.value(); } } }
            button { class: "btn btn-ghost p-0 size-10 text-label", r#type: "button", aria_label: if item.name.is_empty() { "Remove milestone".into() } else { format!("Remove milestone {}", item.name) }, onclick: move |_| {
                form.write().milestones.retain(|item| item.id != id);
                document::eval("document.getElementById('np-add-milestone')?.focus()");
            }, "×" }
        }
        if row_has_error {
            if let Some(message) = error_message { p { id: "np-milestone-error-{id}", class: "text-sm text-danger px-3", "{message}" } }
        }
    }
}
