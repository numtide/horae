use dioxus::prelude::*;
use horae_core::permissions::catalog::Permission;

use crate::components::permission_description::{permission_description, profile_label};
use crate::models::permission_audit::*;
use crate::models::permission_editor::PermissionRequester;
use crate::server_fns;

const FORBIDDEN: u16 = 403;
const UNAUTHORIZED: u16 = 401;
const REFRESH_ID: &str = "audit-refresh";
const NEWEST_ID: &str = "audit-newest";
const OLDER_ID: &str = "audit-older";

#[component]
pub fn PermissionAudit() -> Element {
    let mut after = use_signal(|| None::<AuditCursor>);
    let mut requester = use_signal(|| None::<PermissionRequester>);
    let mut history = use_resource(move || {
        let cursor = after();
        let expected = *requester.peek();
        async move {
            let result = server_fns::list_permission_audit(cursor, expected)
                .await
                .and_then(|page| {
                    if expected.is_some_and(|identity| identity != page.requester) {
                        return Err(ServerFnError::ServerError {
                            code: FORBIDDEN,
                            message: "The active account changed.".into(),
                            details: None,
                        });
                    }
                    if expected.is_none() {
                        requester.set(Some(page.requester));
                    }
                    Ok(page)
                });
            (cursor, result)
        }
    });
    let response = history.read();
    let pending = history.state()() != UseResourceState::Ready
        || response
            .as_ref()
            .is_none_or(|(cursor, _)| *cursor != after());
    let current = response
        .as_ref()
        .filter(|_| !pending)
        .map(|(_, result)| result);
    let next = current
        .and_then(|result| result.as_ref().ok())
        .and_then(|page| page.next_after);

    rsx! {
        section { aria_labelledby: "audit-title", class: "min-w-0",
            div { class: "flex flex-wrap items-center justify-between gap-3 mb-2",
                h1 { id: "audit-title", class: "text-2xl font-semibold", "Audit log" }
                button { id: REFRESH_ID, r#type: "button", class: "btn btn-secondary btn-sm", disabled: pending,
                    onclick: move |_| {
                        if history.state()() == UseResourceState::Ready { history.restart(); }
                    },
                    "Refresh history"
                }
            }
            p { class: "text-sm text-secondary mb-5",
                "Permission changes, newest first. Expand an event to see its recorded details. Times are shown in UTC."
            }
            if pending {
                p { class: "text-sm text-secondary", role: "status", "Loading permission history…" }
            } else if let Some(Ok(page)) = current {
                if page.entries.is_empty() {
                    p { class: "text-sm text-secondary", role: "status",
                        if after().is_some() { "No older permission events." }
                        else { "No permission events recorded yet. Changes to profiles and project managers will appear here." }
                    }
                } else {
                    div { class: "table-container rounded-xl bg-secondary",
                        table { aria_label: "Permission change history",
                            thead { tr {
                                th { scope: "col", "When" }
                                th { scope: "col", "Event" }
                                th { scope: "col", "Actor" }
                            } }
                            tbody {
                                for entry in &page.entries {
                                    tr { key: "{entry.id}",
                                        td { class: "text-xs font-mono text-secondary",
                                            time { datetime: entry.created_at.to_rfc3339(), {entry.created_at.format("%Y-%m-%d %H:%M:%S UTC").to_string()} }
                                        }
                                        td { class: "wrap-anywhere",
                                            details {
                                                summary { class: "cursor-pointer", "{event_label(&entry.audit)}" }
                                                div { class: "text-sm mt-3 flex flex-col gap-3",
                                                    p { class: "text-xs text-secondary", "Receipt: {entry.id}" }
                                                    {audit_details(&entry.audit)}
                                                }
                                            }
                                        }
                                        td { class: "text-xs text-secondary wrap-anywhere", {principal(&entry.actor)} }
                                    }
                                }
                            }
                        }
                    }
                    if next.is_none() {
                        p { class: "text-sm text-secondary mt-3", role: "status", "No more permission events." }
                    }
                }
            } else if let Some(Err(error)) = current {
                p { class: "text-sm text-danger", role: "alert", "{error_message(error)}" }
            }
            div { class: "flex flex-wrap gap-3 mt-4",
                button { id: NEWEST_ID, r#type: "button", class: "btn btn-secondary btn-sm",
                    disabled: pending || after().is_none(),
                    onclick: move |_| {
                        if history.state()() == UseResourceState::Ready && after.peek().is_some() { after.set(None); }
                    }, "Newest events"
                }
                button { id: OLDER_ID, r#type: "button", class: "btn btn-secondary btn-sm",
                    disabled: pending || next.is_none(),
                    onclick: move |_| {
                        if history.state()() == UseResourceState::Ready && let Some(cursor) = next {
                            after.set(Some(cursor));
                        }
                    }, "Older events"
                }
            }
        }
    }
}

fn error_message(error: &ServerFnError) -> &'static str {
    match error {
        ServerFnError::ServerError {
            code: UNAUTHORIZED, ..
        } => "Sign in again to view permission history.",
        ServerFnError::ServerError {
            code: FORBIDDEN, ..
        } => {
            "Administrator access is required for permission history. If you changed accounts, reopen this page."
        }
        _ => "Could not load permission history. Refresh history to try again.",
    }
}

fn event_label(audit: &HistoricalAudit) -> &'static str {
    match audit {
        HistoricalAudit::Profile(audit) if audit.change.is_none() => "No permission change",
        HistoricalAudit::Profile(_) => "Person permissions changed",
        HistoricalAudit::ProjectManagers(audit) if audit.change.is_none() => {
            "No project manager change"
        }
        HistoricalAudit::ProjectManagers(_) => "Project managers changed",
        HistoricalAudit::Template(audit) => match (&audit.before, &audit.after) {
            (None, Some(_)) => "Custom profile created",
            (Some(_), None) => "Custom profile deleted",
            (before, after) if before == after => "No custom profile change",
            _ => "Custom profile changed",
        },
    }
}

fn principal(actor: &AuditPrincipal) -> Element {
    match actor {
        AuditPrincipal::User { user_id } => rsx! { "User: {user_id}" },
        AuditPrincipal::Operator {
            invocation_id,
            command,
        } => rsx! {
            p { "Operator: {command}" }
            p { "Invocation: {invocation_id}" }
        },
    }
}

fn grants_list(grants: &[Permission]) -> Element {
    rsx! {
        if grants.is_empty() { p { "No configured permissions" } }
        else { ul { class: "pl-5", for grant in grants { li { "{permission_description(*grant)}" } } } }
    }
}

fn person_snapshot(label: &str, snapshot: &PersonSnapshot) -> Element {
    let source = match &snapshot.source {
        HistoricalSource::BuiltIn(profile) => profile_label(*profile).to_owned(),
        HistoricalSource::Template {
            id,
            applied_revision,
        } => format!("Custom profile {id}, applied revision {applied_revision}"),
        HistoricalSource::Individual => "Individual configuration".into(),
    };
    rsx! {
        div {
            p { class: "font-semibold", "{label}" }
            p { "Source: {source}" }
            p { "Administrator: " if snapshot.is_administrator { "Yes" } else { "No" } }
            p { "Person revision: {snapshot.revision}" }
            {grants_list(&snapshot.grants)}
        }
    }
}

fn template_snapshot(label: &str, snapshot: &TemplateSnapshot) -> Element {
    rsx! {
        div {
            p { class: "font-semibold", "{label}: {snapshot.name}" }
            p { "Profile: {snapshot.id}" }
            p { "Revision: {snapshot.revision}; permission catalog: {snapshot.catalog_version}" }
            {grants_list(&snapshot.grants)}
        }
    }
}

fn audit_details(audit: &HistoricalAudit) -> Element {
    match audit {
        HistoricalAudit::Profile(audit) => rsx! {
            p { "Person: {audit.user_id}" }
            p { "Access revision: {audit.previous_access_revision} → {audit.access_revision}" }
            if let Some(change) = &audit.change {
                p { "Permission catalog: {change.catalog_version}" }
                {person_snapshot("Before", &change.before)}
                {person_snapshot("After", &change.after)}
                for relation in &change.removed_projects {
                    p { "Removed managed project: {relation.subject_id} (relationship {relation.id}, revision {relation.revision})" }
                }
                for relation in &change.removed_people {
                    p { "Removed managed person: {relation.subject_id} (relationship {relation.id}, revision {relation.revision})" }
                }
            }
        },
        HistoricalAudit::Template(audit) => rsx! {
            p { "Access revision: {audit.previous_access_revision} → {audit.access_revision}" }
            if let Some(before) = &audit.before { {template_snapshot("Before", before)} }
            if let Some(after) = &audit.after { {template_snapshot("After", after)} }
            for person in &audit.detached_people {
                div {
                    p { class: "font-semibold", "Detached person: {person.user_id}" }
                    p { "Source: custom profile {person.previous_template_id}, applied revision {person.previous_applied_revision} → individual configuration" }
                    p { "Person revision: {person.previous_revision} → {person.revision}; permission catalog: {person.catalog_version}" }
                    p { "Retained Administrator: " if person.is_administrator { "Yes" } else { "No" } }
                    {grants_list(&person.grants)}
                }
            }
        },
        HistoricalAudit::ProjectManagers(audit) => rsx! {
            p { "Project: {audit.project_id}" }
            p { "Access revision: {audit.previous_access_revision} → {audit.access_revision}" }
            if let Some(change) = &audit.change {
                for manager in &change.added {
                    p { "Added manager: {manager.manager_id} (designation {manager.id}, revision {manager.revision})" }
                }
                for manager in &change.removed {
                    p { "Removed manager: {manager.manager_id} (designation {manager.id}, revision {manager.revision})" }
                }
            }
        },
    }
}
