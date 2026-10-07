use dioxus::prelude::*;

use super::permission_editor::PermissionEditorDialog;
use crate::components::badge::Badge;
use crate::components::table::DataTable;
use crate::models::people::{PeopleActivity, PeopleCursor};
use crate::models::permission_editor::PermissionRequester;
use crate::server_fns;

#[component]
pub(super) fn CanonicalPeople(can_edit_permissions: bool, on_saved: EventHandler<bool>) -> Element {
    let mut activity = use_signal(|| PeopleActivity::Active);
    let mut cursors = use_signal(|| vec![None::<PeopleCursor>]);
    let mut requester = use_signal(|| None::<PermissionRequester>);
    let mut person = use_signal(|| None);
    let mut page = use_resource(move || async move {
        let filter = activity();
        let cursor = cursors.read().last().cloned().flatten();
        let result = async {
            let loaded = server_fns::list_people(filter, cursor.clone())
                .await
                .map_err(|_| "Could not load people. Check your access and retry.")?;
            let expected = *requester.peek();
            if expected.is_some_and(|expected| expected != loaded.requester) {
                return Err("Your session changed. Reload this page before selecting a person.");
            }
            requester.set(Some(loaded.requester));
            Ok(loaded)
        }
        .await;
        ((filter, cursor), result)
    });
    let key = (activity(), cursors.read().last().cloned().flatten());
    let response = page.read();
    let current = response.as_ref().filter(|(requested, _)| *requested == key);
    let ready = page.state()() == UseResourceState::Ready && current.is_some();
    let next = current
        .and_then(|(_, result)| result.as_ref().ok())
        .and_then(|page| page.next_after.clone());

    rsx! {
        div { class: "page-header",
            h1 { class: "page-title", "People" }
            div { class: "page-actions",
                button { id: "people-refresh", r#type: "button", class: "btn btn-secondary", disabled: !ready,
                    onclick: move |_| page.restart(), "Refresh"
                }
            }
        }
        div { class: "flex flex-wrap items-center gap-3 mb-4",
            label { r#for: "people-activity", class: "form-label", "Show" }
            select { id: "people-activity", class: "form-input", disabled: !ready,
                value: match activity() { PeopleActivity::Active => "active", PeopleActivity::Archived => "archived", PeopleActivity::All => "all" },
                onchange: move |event| {
                    let filter = match event.value().as_str() {
                        "active" => PeopleActivity::Active,
                        "archived" => PeopleActivity::Archived,
                        "all" => PeopleActivity::All,
                        _ => return,
                    };
                    cursors.set(vec![None]);
                    activity.set(filter);
                },
                option { value: "active", "Active people" }
                option { value: "archived", "Archived people" }
                option { value: "all", "All people" }
            }
        }
        if !ready {
            p { role: "status", class: "text-secondary", "Loading people…" }
        } else if let Some((_, result)) = current {
            match result {
                Err(message) => rsx! { p { role: "alert", class: "alert alert-danger", "{message}" } },
                Ok(loaded) if loaded.people.is_empty() => rsx! {
                    p { class: "text-secondary", role: "status", "No people match this filter within your access. Try another filter." }
                },
                Ok(loaded) => rsx! {
                    DataTable {
                        table {
                            thead { tr {
                                th { scope: "col", "Name" }
                                th { scope: "col", "Email" }
                                th { scope: "col", "Status" }
                                if can_edit_permissions { th { scope: "col", "Actions" } }
                            } }
                            tbody {
                                for entry in &loaded.people {
                                    tr { key: "{entry.id}",
                                        td { "{entry.name}" }
                                        td { "{entry.email}" }
                                        td {
                                            if entry.active { Badge { variant: "success", "Active" } }
                                            else { Badge { variant: "neutral", "Archived" } }
                                        }
                                        if can_edit_permissions {
                                            td { button { id: "people-permissions-{entry.id}", r#type: "button", class: "btn btn-secondary btn-sm",
                                                aria_label: "Edit permissions for {entry.name}",
                                                onclick: {
                                                    let selection = (entry.id, loaded.requester);
                                                    move |_| person.set(Some(selection))
                                                },
                                                "Permissions"
                                            } }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
            }
        }
        nav { class: "flex flex-wrap items-center gap-3 mt-4", aria_label: "People pages",
            button { id: "people-previous", r#type: "button", class: "btn btn-secondary", disabled: !ready || cursors.read().len() <= 1,
                onclick: move |_| { if cursors.read().len() > 1 { cursors.write().pop(); } }, "Previous"
            }
            button { id: "people-next", r#type: "button", class: "btn btn-secondary", disabled: !ready || next.is_none(),
                onclick: move |_| { if let Some(cursor) = next.clone() { cursors.write().push(Some(cursor)); } }, "Next"
            }
        }
        PermissionEditorDialog { person, on_saved }
    }
}
