use std::collections::HashSet;

use dioxus::prelude::*;
use horae_core::money::format_cents_plain;
use horae_core::project::{BudgetMode, RateMode};
use horae_core::types::ProjectType;
use uuid::Uuid;

use crate::components::controls::Checkbox;
use crate::components::form::{FormGroup, Input};
use crate::models::project_creation::{
    CreationOptions, CreationPerson, CreationSearch, ProjectForm, ProjectMemberInput,
    ReportVisibility, TaskAccess,
};
use crate::server_fns;

use super::FormRow;

#[component]
pub(super) fn Visibility(mut form: Signal<ProjectForm>) -> Element {
    rsx! {
        FormRow { label: "Report visibility",
            fieldset { class: "border-0 p-0 m-0 flex flex-col gap-2", aria_label: "Report visibility",
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
        }
    }
}

#[component]
pub(super) fn Team(
    mut form: Signal<ProjectForm>,
    mut options: Signal<CreationOptions>,
    mut busy: Signal<bool>,
) -> Element {
    let mut query = use_signal(String::new);
    let mut error = use_signal(|| None::<String>);
    let people = use_resource(move || {
        let query = query();
        async move {
            let mut search = CreationSearch::default();
            search.people.query = query;
            server_fns::project_creation_options(search).await
        }
    });
    let choices = people
        .read()
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .map(|result| result.people.clone())
        .unwrap_or_default();
    rsx! {
        section { class: "bg-secondary border rounded-xl mt-4", aria_labelledby: "np-team-heading",
            div { class: "flex flex-wrap items-baseline gap-3 py-4 px-5 border-b",
                h2 { id: "np-team-heading", class: "text-xl font-semibold", "Team" }
                span { class: "text-xs text-subtle", "{form.read().team.len()} people" }
                span { class: "text-xs text-faint ml-auto", "Check = manages this project" }
            }
            for member in form.read().team.clone() { MemberRow { key: "{member.user_id}", form, options, id: member.user_id } }
            if form.read().team.is_empty() { p { class: "text-sm text-subtle px-5", "No teammates selected yet." } }
            div { class: "p-5",
                if let Some(message) = error() { p { class: "text-sm text-danger", role: "alert", "{message}" } }
                FormGroup { label: "Search teammates", id: "np-team-search",
                    Input { id: "np-team-search", value: query(), oninput: move |event: FormEvent| query.set(event.value()) }
                }
                div { class: "flex flex-wrap items-center gap-3",
                    select { id: "np-add-person", class: "form-select", aria_label: "Add a teammate", disabled: busy(),
                        onchange: move |event| {
                            let Ok(id) = Uuid::parse_str(&event.value()) else { return; };
                            if let Some(Ok(result)) = &*people.read()
                                && let Some(person) = result.people.iter().find(|person| person.id == id)
                            {
                                match add_members(&mut form.write(), std::slice::from_ref(person)) {
                                    Ok(()) => {
                                        if !options.peek().people.iter().any(|existing| existing.id == id) { options.write().people.push(person.clone()); }
                                        error.set(None);
                                    }
                                    Err(message) => error.set(Some(message.into())),
                                }
                            }
                        },
                        option { value: "", selected: true, "Add a teammate…" }
                        for person in choices {
                            option { key: "{person.id}", value: "{person.id}", disabled: form.read().team.iter().any(|member| member.user_id == person.id), "{person.name}" }
                        }
                    }
                    button { r#type: "button", class: "btn btn-secondary", disabled: busy(), onclick: move |_| {
                        if busy() { return; }
                        busy.set(true);
                        error.set(None);
                        spawn(async move {
                            let result = async {
                                let mut all = Vec::new();
                                let mut search = CreationSearch::default();
                                loop {
                                    let result = server_fns::project_creation_options(search.clone()).await.map_err(|error| error.to_string())?;
                                    all.extend(result.people);
                                    if all.len() > 500 { return Err("A project can have at most 500 people. Select teammates individually.".into()); }
                                    if !result.more_people { break; }
                                    search.people.offset += 50;
                                }
                                add_members(&mut form.write(), &all).map_err(str::to_string)?;
                                let mut catalog = options.write();
                                for person in all {
                                    if !catalog.people.iter().any(|existing| existing.id == person.id) { catalog.people.push(person); }
                                }
                                Ok::<_, String>(())
                            }.await;
                            if let Err(message) = result { error.set(Some(message)); }
                            busy.set(false);
                        });
                    }, if busy() { "Loading everyone…" } else { "Add everyone" } }
                }
                match &*people.read() {
                    Some(Err(problem)) => rsx! { p { class: "text-sm text-danger", role: "alert", "Could not search teammates: {problem}" } },
                    Some(Ok(result)) if result.more_people => rsx! { p { class: "form-hint", "Showing 50 matches. Refine your search, or use Add everyone for the whole active team." } },
                    _ => rsx! {},
                }
            }
        }
    }
}

#[component]
fn MemberRow(mut form: Signal<ProjectForm>, options: Signal<CreationOptions>, id: Uuid) -> Element {
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
        div { class: "px-5 py-3 border-b",
            div { class: "flex items-center gap-3",
                Checkbox { checked: member.manager, compact: true, label: "{name} manages this project", onclick: move |_| { if let Some(member) = form.write().team.iter_mut().find(|member| member.user_id == id) { member.manager = !member.manager; } } }
                span { class: "flex-1 text-sm font-semibold", "{name}" }
                button { r#type: "button", class: "btn btn-ghost btn-sm", aria_label: "Remove {name} from project", onclick: move |_| remove_member(&mut form.write(), id), "Remove" }
            }
            div { class: "grid md:grid-cols-3 gap-3 mt-3",
                if form.read().project_type == ProjectType::TimeAndMaterials && form.read().rate_mode == RateMode::Person {
                    FormGroup { label: "Billable rate ({billing_currency}/h)", id: "np-person-rate-{id}",
                        Input { id: "np-person-rate-{id}", value: member.billable_rate, oninput: move |event: FormEvent| { if let Some(member) = form.write().team.iter_mut().find(|member| member.user_id == id) { member.billable_rate = event.value(); } } }
                        if let Some(rate) = person.as_ref().and_then(|person| person.billable_rate_cents) {
                            p { class: "form-hint", "Profile: {org_currency} {format_cents_plain(rate)}/h. Rates are not converted between currencies." }
                        } else { p { class: "form-hint", "No profile rate. Leave blank to use the client's default rate." } }
                    }
                }
                if options.read().can_edit_private_settings {
                    FormGroup { label: "Cost rate ({org_currency}/h) · admins only", id: "np-cost-rate-{id}",
                        Input { id: "np-cost-rate-{id}", value: member.cost_rate, oninput: move |event: FormEvent| { if let Some(member) = form.write().team.iter_mut().find(|member| member.user_id == id) { member.cost_rate = event.value(); } } }
                        if let Some(rate) = person.as_ref().and_then(|person| person.cost_rate_cents) {
                            p { class: "form-hint", "Profile: {org_currency} {format_cents_plain(rate)}/h. A project override does not change the profile." }
                        } else { p { class: "form-hint", "No profile cost rate. Add an optional project-only rate." } }
                    }
                }
                if form.read().budget_mode == BudgetMode::HoursPerPerson {
                    FormGroup { label: "Budget hours", id: "np-person-budget-{id}",
                        Input { id: "np-person-budget-{id}", value: member.budget, oninput: move |event: FormEvent| { if let Some(member) = form.write().team.iter_mut().find(|member| member.user_id == id) { member.budget = event.value(); } } }
                    }
                }
            }
        }
    }
}

fn add_members(form: &mut ProjectForm, people: &[CreationPerson]) -> Result<(), &'static str> {
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
            manager: false,
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
    fn adding_everyone_deduplicates_without_replacing_project_overrides() {
        let person = CreationPerson {
            id: Uuid::now_v7(),
            name: "Member".into(),
            billable_rate_cents: None,
            cost_rate_cents: None,
        };
        let mut form = ProjectForm::default();
        add_members(&mut form, &[person.clone(), person.clone()]).unwrap();
        form.team[0].manager = true;
        form.team[0].cost_rate = "0".into();
        add_members(&mut form, &[person]).unwrap();
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
        assert!(add_members(&mut form, &people).is_err());
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
        add_members(&mut form, std::slice::from_ref(&person)).unwrap();
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
