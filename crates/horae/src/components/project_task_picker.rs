use dioxus::prelude::*;

use crate::components::select_field::SelectField;
use crate::models::{Project, Task};
use crate::server_fns;

/// A task option that cannot be picked: the project has no enabled task.
const NO_TASKS: &str = "none";

/// New-time selectors share the server's eligibility view. Editing keeps the
/// existing, read-only identity even when its context has since been archived.
#[component]
pub fn ProjectTaskPicker(
    mut project: Signal<String>,
    mut task: Signal<String>,
    projects: Resource<Result<Vec<Project>, ServerFnError>>,
    tasks: Resource<Result<Vec<Task>, ServerFnError>>,
    #[props(default = false)] disabled: bool,
) -> Element {
    let tr = crate::i18n::use_tr();
    let contexts = use_resource(|| async move { server_fns::list_time_entry_contexts().await });
    let project_query = use_signal(String::new);
    let task_query = use_signal(String::new);
    let pick_project = use_callback(move |value: String| {
        project.set(value);
        task.set(String::new());
    });
    let pick_task = use_callback(move |value: String| task.set(value));
    use_effect(move || {
        if disabled {
            return;
        }
        let read = contexts.read();
        let Some(Ok(contexts)) = read.as_ref() else {
            return;
        };
        let selected_project = project.read().parse().ok();
        let selected_task = task.read().parse().ok();
        if !contexts
            .iter()
            .any(|c| Some(c.project_id) == selected_project)
        {
            if !project.read().is_empty() {
                project.set(String::new());
            }
            if !task.read().is_empty() {
                task.set(String::new());
            }
        } else if !contexts
            .iter()
            .any(|c| Some(c.project_id) == selected_project && Some(c.task_id) == selected_task)
            && !task.read().is_empty()
        {
            task.set(String::new());
        }
    });

    let context_read = contexts.read();
    let available = context_read.as_ref().and_then(|r| r.as_ref().ok());
    let project_read = projects.read();
    let task_read = tasks.read();
    let project_list = project_read.as_ref().and_then(|r| r.as_ref().ok());
    let task_list = task_read.as_ref().and_then(|r| r.as_ref().ok());
    let selected_project = project.read().parse().ok();
    let error = context_read
        .as_ref()
        .and_then(|r| r.as_ref().err())
        .or_else(|| project_read.as_ref().and_then(|r| r.as_ref().err()))
        .or_else(|| task_read.as_ref().and_then(|r| r.as_ref().err()));

    // Editing shows only the entry's own project and task.
    let project_options: Vec<(String, String)> = if disabled {
        let name = project_list
            .and_then(|ps| ps.iter().find(|p| Some(p.id) == selected_project))
            .map(|p| p.name.clone())
            .unwrap_or_else(|| project.cloned());
        vec![(project(), name)]
    } else {
        match (project_list, available) {
            (Some(projects), Some(contexts)) => projects
                .iter()
                .filter(|p| contexts.iter().any(|c| c.project_id == p.id))
                .map(|p| (p.id.to_string(), p.name.clone()))
                .collect(),
            _ => Vec::new(),
        }
    };
    let task_options: Vec<(String, String)> = if disabled {
        let selected_task = task.read().parse().ok();
        let name = task_list
            .and_then(|ts| ts.iter().find(|t| Some(t.id) == selected_task))
            .map(|t| t.name.clone())
            .unwrap_or_else(|| task.cloned());
        vec![(task(), name)]
    } else {
        match (task_list, available) {
            (Some(tasks), Some(contexts)) => {
                let enabled: Vec<(String, String)> = tasks
                    .iter()
                    .filter(|t| {
                        contexts
                            .iter()
                            .any(|c| Some(c.project_id) == selected_project && c.task_id == t.id)
                    })
                    .map(|t| (t.id.to_string(), t.name.clone()))
                    .collect();
                if enabled.is_empty() && selected_project.is_some() {
                    vec![(NO_TASKS.to_owned(), tr.timer_no_enabled_tasks())]
                } else {
                    enabled
                }
            }
            _ => Vec::new(),
        }
    };

    rsx! {
        if let Some(error) = error {
            div { class: "alert alert-danger", role: "alert", {tr.timer_contexts_failed(error.to_string())} }
        }
        label { class: "sr-only", r#for: "timer-project", {tr.timer_project()} }
        SelectField {
            id: "timer-project",
            label: tr.timer_project(),
            options: project_options,
            selected: project(),
            query: (!disabled).then_some(project_query),
            disabled: disabled || available.is_none(),
            onselect: pick_project,
        }
        label { class: "sr-only", r#for: "timer-task", {tr.timer_task()} }
        SelectField {
            id: "timer-task",
            label: tr.timer_task(),
            options: task_options,
            selected: task(),
            query: (!disabled).then_some(task_query),
            disabled_values: vec![NO_TASKS.to_owned()],
            disabled: disabled || available.is_none() || selected_project.is_none(),
            onselect: pick_task,
        }
    }
}
