use dioxus::prelude::*;
use uuid::Uuid;

use crate::components::select_field::SelectField;
use crate::models::people::PeopleCursor;
use crate::models::permission_editor::PermissionRequester;
use crate::models::scoped_time::{TimesheetPeopleQuery, TimesheetPerson};
use crate::server_fns;

/// Navigation labels only; loading the selected sheet independently checks scope.
#[component]
pub(super) fn PersonPicker(
    requester: PermissionRequester,
    selected: TimesheetPerson,
    disabled: bool,
    on_selected: EventHandler<Uuid>,
) -> Element {
    let query = use_signal(String::new);
    let mut cursors = use_signal(|| (String::new(), vec![None::<PeopleCursor>]));
    let requested = use_memo(move || {
        let search = query().trim().to_owned();
        let pages = cursors.read();
        TimesheetPeopleQuery {
            after: if pages.0 == search {
                pages.1.last().cloned().flatten()
            } else {
                None
            },
            search,
            user_id: None,
        }
    });
    let mut page = use_resource(move || {
        let request = requested();
        async move {
            let result = server_fns::list_timesheet_people(request.clone())
                .await
                .and_then(|page| {
                    if page.requester == requester {
                        Ok(page)
                    } else {
                        Err(ServerFnError::new(
                            "Your session changed. Reload before choosing a person.",
                        ))
                    }
                });
            (request, result)
        }
    });
    let current = use_memo(move || {
        if page.state() != UseResourceState::Ready {
            return None;
        }
        page.read()
            .as_ref()
            .filter(|(request, _)| request == &requested())
            .map(|(_, result)| result.clone())
    });
    let ready = matches!(&*current.read(), Some(Ok(_)));
    let mut options = vec![(selected.id.to_string(), selected.name.clone())];
    if let Some(Ok(page)) = current.read().as_ref() {
        options.extend(
            page.people
                .iter()
                .filter(|person| person.id != selected.id)
                .map(|person| (person.id.to_string(), person.name.clone())),
        );
    }
    rsx! {
        fieldset { class: "border-0 p-0 m-0 min-w-0", disabled,
            label { r#for: "timesheet-person", class: "form-label", "Timesheet for" }
            SelectField {
                id: "timesheet-person", label: "person", options,
                selected: selected.id.to_string(), query: Some(query),
                pending: !ready || disabled,
                onselect: move |value: String| {
                    if !disabled && ready && let Ok(id) = value.parse::<Uuid>() && id != selected.id {
                        on_selected.call(id);
                    }
                },
                if let Some(Err(error)) = current.read().as_ref() {
                    p { class: "text-sm text-danger p-2", role: "alert", "{error}" }
                    button { r#type: "button", class: "btn btn-secondary btn-sm", onclick: move |_| page.restart(), "Retry people" }
                }
                if let Some(Ok(value)) = current.read().as_ref() {
                    if value.people.is_empty() {
                        p { class: "text-sm text-subtle p-2", role: "status", "No matching people." }
                    }
                    if cursors.read().0 == requested().search && cursors.read().1.len() > 1 {
                        button { r#type: "button", class: "btn btn-ghost btn-sm", disabled,
                            onclick: move |_| { if !disabled { cursors.write().1.pop(); } }, "Previous people"
                        }
                    }
                    if let Some(next) = value.next_after.clone() {
                        button { r#type: "button", class: "btn btn-ghost btn-sm", disabled,
                            onclick: move |_| {
                                if disabled { return; }
                                let search = requested().search;
                                let mut pages = cursors.write();
                                if pages.0 != search { *pages = (search, vec![None]); }
                                pages.1.push(Some(next.clone()));
                            }, "Next people"
                        }
                    }
                }
            }
        }
    }
}
