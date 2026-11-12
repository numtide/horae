use std::collections::HashSet;

use dioxus::prelude::*;
use horae_core::importers::harvest::types::{PersonErrorKind, SourceKind};

use super::missing_person_row::{MissingPersonRow, UNAVAILABLE};
use crate::models::MissingPerson;
use crate::models::user::{Activation, AddedPeople, NewPerson};
use crate::route::Route;
use crate::server_fns;

/// The Harvest people whose records a run could not attach, with a way to add
/// each to Horae in place (spec 019 FR-018).
#[component]
pub(super) fn MissingPeople(
    people: Vec<MissingPerson>,
    omitted: u64,
    source: Option<SourceKind>,
) -> Element {
    // Emails, as `key` normalizes them, added from this page, and those that
    // turned out to have an account already (another tab, or a reload), or
    // that the server refused.
    let mut added = use_signal(HashSet::<String>::new);
    let mut present = use_signal(HashSet::<String>::new);
    let mut unavailable = use_signal(HashSet::<String>::new);
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let key = |email: &str| email.trim().to_lowercase();
    let on_added = use_callback(move |result: AddedPeople| {
        added
            .write()
            .extend(result.added.iter().map(|user| key(&user.email)));
        present
            .write()
            .extend(result.present.iter().map(|email| key(email)));
        unavailable
            .write()
            .extend(result.unavailable.iter().map(|email| key(email)));
    });
    let outcome = move |email: Option<&str>| {
        let email = key(email?);
        if added.read().contains(&email) {
            Some("Added")
        } else if present.read().contains(&email) {
            Some("Already in Horae")
        } else if unavailable.read().contains(&email) {
            Some(UNAVAILABLE)
        } else {
            None
        }
    };
    let pending: Vec<NewPerson> = people
        .iter()
        .filter(|person| person.kind == PersonErrorKind::UnmatchedEmail)
        .filter_map(|person| {
            let email = person.email.clone()?;
            outcome(Some(&email)).is_none().then(|| NewPerson {
                name: person.name.clone().unwrap_or_default(),
                email,
            })
        })
        .collect();
    let add_all = {
        let pending = pending.clone();
        move |_| {
            let pending = pending.clone();
            async move {
                if *busy.peek() || pending.is_empty() {
                    return;
                }
                busy.set(true);
                error.set(None);
                match server_fns::add_harvest_people(pending, Activation::Deactivated).await {
                    Ok(result) => on_added.call(result),
                    Err(e) => error.set(Some(e.to_string())),
                }
                busy.set(false);
            }
        }
    };
    if people.is_empty() {
        return rsx! {};
    }
    let preview_again = match source {
        Some(SourceKind::Csv) => Route::HarvestCsv {},
        _ => Route::HarvestOverview {
            outcome: Default::default(),
        },
    };
    rsx! {
        section { class: "card p-0 overflow-hidden mt-4", aria_label: "People not imported",
            div { class: "flex items-center gap-3 flex-wrap p-4 bg-secondary",
                h2 { class: "text-sm font-semibold", "People not imported" }
                span { class: "badge badge-warning badge-sm", "{people.len()}" }
                div { class: "flex-1" }
                if !pending.is_empty() {
                    button {
                        r#type: "button",
                        class: "btn btn-secondary btn-sm",
                        "data-testid": "add-all-people".to_owned(),
                        disabled: busy(),
                        onclick: add_all,
                        "Add all as deactivated ({pending.len()})"
                    }
                }
            }
            p { class: "px-4 pt-3 text-sm text-secondary",
                "An added person's email must match their Harvest email. Someone who already has an account under another address needs that address changed in Harvest or in Horae. Added people are deactivated unless you choose otherwise, and cannot sign in until activated."
            }
            if let Some(message) = error() {
                div { class: "alert alert-danger m-4", role: "alert", "{message}" }
            }
            if !added.read().is_empty() || !present.read().is_empty() {
                div { class: "alert alert-success m-4", role: "status",
                    if !added.read().is_empty() { "Added {added.read().len()} people. " }
                    if !present.read().is_empty() { "{present.read().len()} already had an account and were left as they are. " }
                    "Their entries import with the next run. "
                    Link { to: preview_again, "data-testid": "preview-again", "Start a new preview" }
                }
            }
            div { class: "overflow-x-auto border-t",
                table { class: "table",
                    thead {
                        tr {
                            th { class: "whitespace-nowrap", "Person" }
                            th { class: "whitespace-nowrap", "Problem" }
                            th { class: "whitespace-nowrap", "Entries" }
                        }
                    }
                    tbody {
                        for person in people.iter() {
                            MissingPersonRow {
                                key: "{person.email.as_deref().or(person.name.as_deref()).unwrap_or_default()}",
                                person: person.clone(),
                                outcome: outcome(person.email.as_deref()),
                                busy,
                                on_added,
                            }
                        }
                    }
                }
            }
            if omitted > 0 {
                div { class: "p-4 border-t text-faint text-sm", "{omitted} more people are not listed. Download the errors for all of them." }
            }
        }
    }
}
