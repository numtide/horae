use std::collections::HashSet;

use crate::models::project_creation::ProtectedProjectField;
use dioxus::prelude::*;
use horae_core::money::format_cents_plain;
use horae_core::project::{BudgetMode, RateMode};
use horae_core::types::ProjectType;
use uuid::Uuid;

use crate::components::avatar::{Avatar, first_initial};
use crate::components::controls::Checkbox;
use crate::components::form::Input;
use crate::components::icons::NavIcon;
use crate::components::select_field::SelectField;
use crate::models::project_creation::{
    CreationOptions, CreationPerson, ProjectEditorContext, ProjectFieldAccess, ProjectForm,
    ProjectFormField, ProjectManagerSelection, ProjectMemberInput, ReportVisibility, TaskAccess,
};
use crate::models::project_managers::ProjectManager;

use super::FormRow;

#[component]
pub(super) fn Visibility(mut form: Signal<ProjectForm>, #[props(default)] legacy: bool) -> Element {
    rsx! {
        FormRow { label: "Report visibility",
            fieldset { class: "border-0 p-0 m-0 flex flex-col gap-2", aria_label: "Report visibility", disabled: legacy,
                for (value, label, hint) in [
                    (ReportVisibility::Managers, "Managers only", "Organization managers and this project's leads can see progress."),
                    (ReportVisibility::ProjectMembers, "Everyone on the project", "Project members can also see progress, without private rates or costs."),
                ] {
                    label { class: "np-option flex items-center gap-3 p-3 bg-base border border-input rounded-btn cursor-pointer",
                        input { class: "choice-box radio size-4 m-0", r#type: "radio", name: "np-visibility", checked: form.read().report_visibility == value, onchange: move |_| form.write().report_visibility = value }
                        span { span { class: "block text-sm", "{label}" } span { class: "block text-xs text-subtle mt-1", "{hint}" } }
                    }
                }
            }
            if legacy { p { class: "form-hint", "This project keeps its existing member visibility. Changing to the new visibility configuration requires a migration." } }
        }
    }
}

#[component]
pub(super) fn Team(
    mut form: Signal<ProjectForm>,
    mut managers: Signal<Option<ProjectManagerSelection>>,
    mut options: Signal<CreationOptions>,
    mut busy: Signal<bool>,
    #[props(default)] editor_context: Option<ProjectEditorContext>,
    billable_access: ProjectFieldAccess,
    cost_access: ProjectFieldAccess,
    on_edit: EventHandler<ProtectedProjectField>,
    #[props(default)] retained_managers: Vec<ProjectManager>,
    #[props(default)] inactive_ids: Vec<Uuid>,
    #[props(default)] invalid_field: Option<ProjectFormField>,
    #[props(default)] error_message: Option<String>,
) -> Element {
    // Keep identities after an uncheck or tracking removal so local edits remain reversible.
    let mut manager_people = use_signal(|| retained_managers);
    let inactive_for_toggle = inactive_ids.clone();
    let on_toggle_manager = use_callback(move |id: Uuid| {
        if busy() {
            return;
        }
        let known = manager_people.peek().iter().any(|person| person.id == id);
        if inactive_for_toggle.contains(&id) && (!known || managers.peek().is_none()) {
            return;
        }
        if managers.peek().is_some() && !known {
            let Some(person) = options
                .peek()
                .people
                .iter()
                .find(|person| person.id == id)
                .cloned()
            else {
                return;
            };
            manager_people.write().push(ProjectManager {
                id,
                name: person.name,
                active: true,
            });
        }
        toggle_manager(&mut form.write(), managers.write().as_mut(), id);
    });
    let query = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);
    let mut people = use_resource(move || {
        let query = query();
        async move { super::catalog::people(editor_context, query).await }
    });
    let pending = people.state()() != UseResourceState::Ready;
    let choices = people
        .read()
        .as_ref()
        .filter(|_| !pending)
        .and_then(|result| result.as_ref().ok())
        .map(|result| result.people.clone())
        .unwrap_or_default();
    let choices = std::iter::once((String::new(), "Add a teammate…".into()))
        .chain(
            choices
                .into_iter()
                .map(|person| (person.id.to_string(), person.name)),
        )
        .collect();
    let disabled_people = std::iter::once(String::new())
        .chain(
            form.read()
                .team
                .iter()
                .map(|member| member.user_id.to_string()),
        )
        .collect();
    rsx! {
        section { class: "bg-secondary border rounded-xl mt-4", aria_labelledby: "np-team-heading",
            div { class: "flex flex-wrap items-baseline gap-3 py-4 px-5 border-b",
                h2 { id: "np-team-heading", class: "text-xl font-semibold m-0", "Team" }
                span { class: "text-xs text-subtle", "{form.read().team.len()} people" }
                span { class: "text-xs text-label ml-auto", "Check = manages this project" }
            }
            for member in form.read().team.clone() {
                MemberRow {
                    key: "{member.user_id}", form, options, billable_access, cost_access, on_edit,
                    id: member.user_id, inactive: inactive_ids.contains(&member.user_id),
                    manager_editable: !inactive_ids.contains(&member.user_id) || (managers.read().is_some() && manager_people.read().iter().any(|person| person.id == member.user_id)),
                    on_toggle_manager, invalid_field, error_message: error_message.clone(),
                }
            }
            for person in manager_people.read().iter().filter(|person| !form.read().team.iter().any(|member| member.user_id == person.id)).cloned() {
                div { key: "{person.id}", class: "np-assignment-row grid items-center gap-4 px-5 py-3 border-b border-light min-w-0",
                    Checkbox {
                        id: "np-person-manager-{person.id}",
                        checked: managers.read().as_ref().is_some_and(|selection| selection.manager_ids.contains(&person.id)),
                        compact: true, label: "{person.name} manages this project",
                        onclick: move |_| on_toggle_manager.call(person.id),
                    }
                    div { class: "flex items-center gap-4 min-w-0",
                        Avatar { initials: first_initial(&person.name), size: "project" }
                        div { class: "min-w-0",
                            div { class: "text-sm text-strong truncate", title: "{person.name}", "{person.name}" }
                            div { class: "text-xs text-subtle", "Not on the tracking team" }
                            if !person.active { div { class: "text-xs text-subtle", "Inactive" } }
                        }
                    }
                }
            }
            if form.read().team.is_empty() { p { class: "text-sm text-subtle px-5", "No teammates selected yet." } }
            div { class: "px-5 py-3",
                p { class: "form-hint mt-0",
                    if form.read().rate_mode == RateMode::Legacy {
                        "Person rates apply after task overrides and before the project and profile rates. Blank costs inherit the profile cost. Rates are not converted between currencies."
                    } else {
                        "Blank billable rates inherit the person's profile, then the client's default. Blank costs inherit only the profile cost. Rates are not converted between currencies. Overrides affect only this project."
                    }
                }
                if let Some(message) = error() { p { class: "text-sm text-danger", role: "alert", "{message}" } }
                div { class: "flex flex-wrap items-center gap-3",
                    div { class: "flex-1 basis-assignment-picker min-w-0",
                        SelectField { id: "np-add-person", label: "teammate", options: choices,
                            selected: "", query: Some(query), disabled_values: disabled_people, pending,
                            onselect: move |value: String| {
                                if busy() || people.state()() != UseResourceState::Ready { return; }
                                let Ok(id) = Uuid::parse_str(&value) else { return; };
                                if let Some(Ok(result)) = &*people.read()
                                    && let Some(person) = result.people.iter().find(|person| person.id == id)
                                {
                                    match add_members(&mut form.write(), std::slice::from_ref(person), managers.peek().as_ref()) {
                                        Ok(()) => {
                                            if !options.peek().people.iter().any(|existing| existing.id == id) { options.write().people.push(person.clone()); }
                                            error.set(None);
                                        }
                                        Err(message) => error.set(Some(message.into())),
                                    }
                                }
                            },
                            if !pending {
                                match &*people.read() {
                                    Some(Err(problem)) => rsx! {
                                        p { class: "text-sm text-danger p-2", role: "alert", "Could not search teammates: {problem}" }
                                        button { r#type: "button", class: "btn btn-secondary btn-sm", onclick: move |_| people.restart(), "Retry teammate search" }
                                    },
                                    Some(Ok(result)) if result.people.is_empty() => rsx! { p { class: "text-sm text-subtle p-2 m-0", role: "status", "No matching teammates." } },
                                    Some(Ok(result)) if result.more_people => rsx! { p { class: "form-hint p-2", "Showing 50 matches. Refine your search, or use Add everyone for the whole active team." } },
                                    _ => rsx! {},
                                }
                            }
                        }
                    }
                    button { id: "np-add-everyone", r#type: "button", class: "btn btn-secondary", disabled: busy(), onclick: move |_| {
                        if busy() { return; }
                        busy.set(true);
                        error.set(None);
                        spawn(async move {
                            let result = async {
                                let all = super::catalog::all_people(editor_context).await.map_err(|error| error.to_string())?;
                                add_members(&mut form.write(), &all, managers.peek().as_ref()).map_err(str::to_string)?;
                                let mut catalog = options.write();
                                for person in all {
                                    if !catalog.people.iter().any(|existing| existing.id == person.id) { catalog.people.push(person); }
                                }
                                Ok::<_, String>(())
                            }.await;
                            if let Err(message) = result { error.set(Some(message)); }
                            busy.set(false);
                        });
                    }, NavIcon { name: "users" } if busy() { "Loading everyone…" } else { "Add everyone" } }
                }
            }
        }
    }
}

#[component]
fn MemberRow(
    mut form: Signal<ProjectForm>,
    options: Signal<CreationOptions>,
    billable_access: ProjectFieldAccess,
    cost_access: ProjectFieldAccess,
    on_edit: EventHandler<ProtectedProjectField>,
    id: Uuid,
    #[props(default)] inactive: bool,
    manager_editable: bool,
    on_toggle_manager: EventHandler<Uuid>,
    invalid_field: Option<ProjectFormField>,
    error_message: Option<String>,
) -> Element {
    let Some(member) = form
        .read()
        .team
        .iter()
        .find(|member| member.user_id == id)
        .cloned()
    else {
        return rsx! {};
    };
    let person = options
        .read()
        .people
        .iter()
        .find(|person| person.id == id)
        .cloned();
    let name = person
        .as_ref()
        .map(|person| person.name.clone())
        .unwrap_or_else(|| format!("Unavailable teammate ({id})"));
    let org_currency = options.read().organization_currency.clone();
    let billing_currency = form
        .read()
        .currency
        .clone()
        .or_else(|| {
            options
                .read()
                .clients
                .iter()
                .find(|client| Some(client.id) == form.read().client_id)
                .map(|client| client.currency.clone())
        })
        .unwrap_or_else(|| "project currency".into());
    rsx! {
        div { class: "np-assignment-row grid items-center gap-4 px-5 py-3 border-0 border-b border-light m-0 min-w-0",
            Checkbox { id: "np-person-manager-{id}", checked: member.manager, compact: true, disabled: !manager_editable, label: "{name} manages this project", onclick: move |_| on_toggle_manager.call(id) }
            div { class: "flex items-center gap-4 min-w-0",
                Avatar { initials: first_initial(&name), size: "project" }
                div { class: "min-w-0",
                    div { class: "text-sm text-strong truncate", title: "{name}", "{name}" }
                    if inactive { div { class: "text-xs text-subtle", "Inactive · tracking settings read only" } }
                    div { class: "text-xs text-subtle", if member.manager { "Project manager" } else { "Project member" } }
                }
            }
            fieldset { class: "np-row-controls flex flex-wrap items-center gap-4 min-w-0 border-0 p-0 m-0", aria_label: "Tracking settings for {name}", disabled: inactive,
                if billable_access != ProjectFieldAccess::Withheld && ((form.read().project_type == ProjectType::TimeAndMaterials && form.read().rate_mode == RateMode::Person) || form.read().rate_mode == RateMode::Legacy) {
                    label { class: "flex items-center gap-2 text-xs text-subtle", r#for: "np-person-rate-{id}", oninput: move |_| on_edit.call(ProtectedProjectField::PersonRate(id)),
                        "bill"
                        Input { class: "w-30 max-w-full font-mono text-right", id: "np-person-rate-{id}", disabled: billable_access != ProjectFieldAccess::Editable, label: "Billable rate for {name} ({billing_currency}/h)", value: member.billable_rate,
                            error_id: (invalid_field == Some(ProjectFormField::PersonRate(id))).then(|| format!("np-person-error-{id}")),
                            placeholder: if org_currency == billing_currency { person.as_ref().and_then(|person| person.billable_rate_cents).map(format_cents_plain).unwrap_or_else(|| "Inherit".into()) } else { "Inherit".into() },
                            oninput: move |event: FormEvent| { if let Some(member) = form.write().team.iter_mut().find(|member| member.user_id == id) { member.billable_rate = event.value(); } }
                        }
                        "{billing_currency}/h"
                    }
                }
                if cost_access != ProjectFieldAccess::Withheld {
                    label { class: "flex items-center gap-2 text-xs text-subtle", r#for: "np-cost-rate-{id}", oninput: move |_| on_edit.call(ProtectedProjectField::CostRate(id)),
                        "cost"
                        Input { class: "w-30 max-w-full font-mono text-right", id: "np-cost-rate-{id}", disabled: cost_access != ProjectFieldAccess::Editable, label: "Cost rate for {name} ({org_currency}/h)", value: member.cost_rate,
                            error_id: (invalid_field == Some(ProjectFormField::CostRate(id))).then(|| format!("np-person-error-{id}")),
                            placeholder: person.as_ref().and_then(|person| person.cost_rate_cents).map(format_cents_plain).unwrap_or_else(|| "Inherit".into()),
                            oninput: move |event: FormEvent| { if let Some(member) = form.write().team.iter_mut().find(|member| member.user_id == id) { member.cost_rate = event.value(); } }
                        }
                        "{org_currency}/h"
                    }
                }
                if form.read().budget_mode == BudgetMode::HoursPerPerson {
                    label { class: "flex items-center gap-2 text-xs text-subtle", r#for: "np-person-budget-{id}", oninput: move |_| on_edit.call(ProtectedProjectField::Budget),
                        "budget"
                        Input { class: "w-30 max-w-full font-mono text-right", id: "np-person-budget-{id}", label: "Budget hours for {name}", value: member.budget,
                            error_id: (invalid_field == Some(ProjectFormField::PersonBudget(id))).then(|| format!("np-person-error-{id}")),
                            oninput: move |event: FormEvent| { if let Some(member) = form.write().team.iter_mut().find(|member| member.user_id == id) { member.budget = event.value(); } } }
                        "h"
                    }
                }
            }
            button { id: "np-person-remove-{id}", r#type: "button", class: "np-row-remove btn btn-ghost p-0 size-8 text-label", disabled: inactive, aria_label: "Remove {name} from project",
                aria_invalid: (invalid_field == Some(ProjectFormField::Person(id))).then_some("true"),
                aria_describedby: (invalid_field == Some(ProjectFormField::Person(id))).then(|| format!("np-person-error-{id}")),
                onclick: move |_| {
                    remove_member(&mut form.write(), id);
                    document::eval("document.getElementById('np-add-person')?.focus()");
                }, "×" }
        }
        if matches!(invalid_field, Some(ProjectFormField::Person(row) | ProjectFormField::PersonRate(row) | ProjectFormField::CostRate(row) | ProjectFormField::PersonBudget(row)) if row == id) {
            p { id: "np-person-error-{id}", class: "text-sm text-danger px-5",
                "{error_message.as_deref().unwrap_or_default()}"
                if invalid_field == Some(ProjectFormField::Person(id)) { " Remove this person and select an available teammate below." }
            }
        }
    }
}

fn toggle_manager(
    form: &mut ProjectForm,
    managers: Option<&mut ProjectManagerSelection>,
    id: Uuid,
) {
    let member = form.team.iter_mut().find(|member| member.user_id == id);
    let selected = if let Some(managers) = managers {
        let selected = !managers.manager_ids.contains(&id);
        if selected {
            managers.manager_ids.push(id);
            managers.manager_ids.sort_unstable();
        } else {
            managers.manager_ids.retain(|manager| *manager != id);
        }
        selected
    } else if let Some(member) = &member {
        !member.manager
    } else {
        return;
    };
    if let Some(member) = member {
        member.manager = selected;
    }
}

fn add_members(
    form: &mut ProjectForm,
    people: &[CreationPerson],
    managers: Option<&ProjectManagerSelection>,
) -> Result<(), &'static str> {
    let mut ids: HashSet<_> = form.team.iter().map(|member| member.user_id).collect();
    let additions: Vec<_> = people
        .iter()
        .filter(|person| ids.insert(person.id))
        .collect();
    if form.team.len() + additions.len() > 500 {
        return Err("A project can have at most 500 people.");
    }
    form.team
        .extend(additions.into_iter().map(|person| ProjectMemberInput {
            user_id: person.id,
            manager: managers.is_some_and(|selection| selection.manager_ids.contains(&person.id)),
            billable_rate: String::new(),
            cost_rate: String::new(),
            budget: String::new(),
        }));
    Ok(())
}

fn remove_member(form: &mut ProjectForm, id: Uuid) {
    form.team.retain(|member| member.user_id != id);
    for task in &mut form.tasks {
        if let TaskAccess::Restricted { user_ids } = &mut task.access {
            user_ids.retain(|user_id| *user_id != id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::project_creation::{
        CreationPerson, ProjectForm, ProjectTaskInput, TaskAccess, TaskSource,
    };
    use uuid::Uuid;

    #[test]
    fn manager_checkbox_updates_complete_selection_without_dropping_outside_managers() {
        let person = CreationPerson {
            id: Uuid::now_v7(),
            name: "Teammate".into(),
            billable_rate_cents: None,
            cost_rate_cents: None,
        };
        let outside = Uuid::now_v7();
        let mut selection = ProjectManagerSelection {
            expected_access_revision: 12,
            manager_ids: vec![outside],
        };
        let mut form = ProjectForm::default();
        add_members(&mut form, std::slice::from_ref(&person), Some(&selection)).unwrap();
        toggle_manager(&mut form, Some(&mut selection), person.id);
        assert!(form.team[0].manager);
        assert!(selection.manager_ids.contains(&person.id));
        assert!(selection.manager_ids.contains(&outside));
        assert_eq!(selection.manager_ids.len(), 2);
        toggle_manager(&mut form, Some(&mut selection), person.id);
        assert!(!form.team[0].manager);
        assert_eq!(selection.manager_ids, vec![outside]);
        assert_eq!(selection.expected_access_revision, 12);
    }

    #[test]
    fn tracking_removal_and_readdition_keep_independent_manager_intent() {
        let person = CreationPerson {
            id: Uuid::now_v7(),
            name: "Retained manager".into(),
            billable_rate_cents: None,
            cost_rate_cents: None,
        };
        let mut selection = ProjectManagerSelection {
            expected_access_revision: 12,
            manager_ids: vec![person.id],
        };
        let mut form = ProjectForm::default();
        add_members(&mut form, std::slice::from_ref(&person), Some(&selection)).unwrap();
        assert!(
            form.team[0].manager,
            "adding a retained manager must reflect the designation"
        );
        remove_member(&mut form, person.id);
        assert!(form.team.is_empty());
        assert_eq!(selection.manager_ids, vec![person.id]);
        add_members(&mut form, std::slice::from_ref(&person), Some(&selection)).unwrap();
        assert!(form.team[0].manager);
        toggle_manager(&mut form, Some(&mut selection), person.id);
        remove_member(&mut form, person.id);
        add_members(&mut form, &[person], Some(&selection)).unwrap();
        assert!(
            !form.team[0].manager,
            "an explicit uncheck must survive removal and readdition"
        );
        assert!(selection.manager_ids.is_empty());
    }

    #[test]
    fn legacy_manager_checkbox_needs_no_canonical_selection() {
        let person = CreationPerson {
            id: Uuid::now_v7(),
            name: "Legacy teammate".into(),
            billable_rate_cents: None,
            cost_rate_cents: None,
        };
        let mut form = ProjectForm::default();
        add_members(&mut form, std::slice::from_ref(&person), None).unwrap();
        toggle_manager(&mut form, None, person.id);
        assert!(form.team[0].manager);
        toggle_manager(&mut form, None, person.id);
        assert!(!form.team[0].manager);
    }

    #[test]
    fn adding_everyone_deduplicates_without_replacing_project_overrides() {
        let person = CreationPerson {
            id: Uuid::now_v7(),
            name: "Member".into(),
            billable_rate_cents: None,
            cost_rate_cents: None,
        };
        let mut form = ProjectForm::default();
        add_members(&mut form, &[person.clone(), person.clone()], None).unwrap();
        form.team[0].manager = true;
        form.team[0].cost_rate = "0".into();
        add_members(&mut form, &[person], None).unwrap();
        assert_eq!(form.team.len(), 1);
        assert!(form.team[0].manager);
        assert_eq!(form.team[0].cost_rate, "0");
    }

    #[test]
    fn exceeding_the_team_limit_does_not_partially_add_people() {
        let people: Vec<_> = (0..501)
            .map(|_| CreationPerson {
                id: Uuid::now_v7(),
                name: "Member".into(),
                billable_rate_cents: None,
                cost_rate_cents: None,
            })
            .collect();
        let mut form = ProjectForm::default();
        assert!(add_members(&mut form, &people, None).is_err());
        assert!(form.team.is_empty());
    }

    #[test]
    fn removing_a_member_keeps_an_empty_restricted_task_restricted() {
        let person = CreationPerson {
            id: Uuid::now_v7(),
            name: "Member".into(),
            billable_rate_cents: None,
            cost_rate_cents: None,
        };
        let mut form = ProjectForm::default();
        add_members(&mut form, std::slice::from_ref(&person), None).unwrap();
        form.tasks.push(ProjectTaskInput {
            id: Uuid::now_v7(),
            source: TaskSource::New {
                name: "Task".into(),
            },
            billable: true,
            rate: String::new(),
            budget: String::new(),
            access: TaskAccess::Restricted {
                user_ids: vec![person.id],
            },
        });
        remove_member(&mut form, person.id);
        assert!(form.team.is_empty());
        assert_eq!(
            form.tasks[0].access,
            TaskAccess::Restricted { user_ids: vec![] }
        );
    }
}
