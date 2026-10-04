use dioxus::prelude::*;
use uuid::Uuid;

use crate::components::select_field::SelectField;
use crate::models::scoped_time::{TimesheetPolicy, TimesheetTrackingOption, VisibleTimeEntry};

pub(super) fn eligible(options: &[TimesheetTrackingOption], project: Uuid, task: Uuid) -> bool {
    options
        .iter()
        .any(|option| option.project_id == project && option.task_id == task)
}

/// Advisory controls only. The server rechecks scope, tracking and locks atomically.
pub(super) fn editable(
    policy: Option<TimesheetPolicy>,
    options: &[TimesheetTrackingOption],
    entry: &VisibleTimeEntry,
) -> bool {
    policy.is_some()
        && entry.state == horae_core::types::EntryState::Open
        && (policy == Some(TimesheetPolicy::LegacyOwn)
            || eligible(options, entry.project_id, entry.task_id))
}

pub(super) fn stoppable(
    options: &[TimesheetTrackingOption],
    entry: &VisibleTimeEntry,
    requester: Option<Uuid>,
) -> bool {
    requester.is_some()
        && entry.is_running
        && entry.state == horae_core::types::EntryState::Open
        && (requester == Some(entry.user_id) || eligible(options, entry.project_id, entry.task_id))
}

#[component]
pub(super) fn TrackingPicker(
    options: Vec<TimesheetTrackingOption>,
    mut project: Signal<String>,
    mut task: Signal<String>,
    disabled: bool,
) -> Element {
    let project_query = use_signal(String::new);
    let task_query = use_signal(String::new);
    let mut projects = Vec::new();
    for option in &options {
        let id = option.project_id.to_string();
        if !projects.iter().any(|(value, _)| value == &id) {
            projects.push((id, option.project_name.clone()));
        }
    }
    let tasks = options
        .iter()
        .filter(|option| option.project_id.to_string() == project())
        .map(|option| (option.task_id.to_string(), option.task_name.clone()))
        .collect();
    rsx! {
        fieldset { class: "border-0 p-0 m-0 min-w-0", disabled,
            div { class: "grid gap-2",
                label { r#for: "timesheet-project", class: "form-label", "Project" }
                SelectField {
                    id: "timesheet-project", label: "project", options: projects,
                    selected: project(), query: Some(project_query), pending: disabled,
                    onselect: move |value: String| {
                        if disabled { return; }
                        let next = options.iter().find(|option| option.project_id.to_string() == value);
                        if let Some(next) = next {
                            project.set(value);
                            task.set(next.task_id.to_string());
                        }
                    },
                }
                label { r#for: "timesheet-task", class: "form-label", "Task" }
                SelectField {
                    id: "timesheet-task", label: "task", options: tasks,
                    selected: task(), query: Some(task_query), pending: disabled,
                    onselect: move |value: String| { if !disabled { task.set(value); } },
                }
            }
        }
    }
}
