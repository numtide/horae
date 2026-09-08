use dioxus::prelude::*;

use crate::models::{Project, Task};
use crate::server_fns;

/// New-time selectors share the server's eligibility view. Editing keeps the
/// existing, read-only identity even when its context has since been archived.
#[component]
pub fn ProjectTaskPicker(
    mut project: Signal<String>,
    mut task: Signal<String>,
    mut projects: Resource<Result<Vec<Project>, ServerFnError>>,
    mut tasks: Resource<Result<Vec<Task>, ServerFnError>>,
    #[props(default = false)] disabled: bool,
) -> Element {
    let contexts = use_resource(|| async move { server_fns::list_time_entry_contexts().await });
    // The sidebar outlives project management pages: refresh names on opening
    // the picker so a newly created/enabled task is visible without a reload.
    use_effect(move || {
        projects.restart();
        tasks.restart();
    });
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

    rsx! {
        if let Some(error) = error {
            div { class: "alert alert-danger", role: "alert", "Could not load projects and tasks: {error}" }
        }
        select {
            class: "form-select",
            "aria-label": "Project",
            value: "{project}",
            disabled: disabled || available.is_none(),
            onchange: move |e| { project.set(e.value()); task.set(String::new()); },
            if disabled {
                option { value: "{project}",
                    {project_list.and_then(|ps| ps.iter().find(|p| Some(p.id) == selected_project)).map(|p| p.name.clone()).unwrap_or_else(&*project)}
                }
            } else {
                option { value: "", "Select project…" }
                if let (Some(projects), Some(contexts)) = (project_list, available) {
                    for p in projects.iter().filter(|p| contexts.iter().any(|c| c.project_id == p.id)) {
                        option { value: "{p.id}", "{p.name}" }
                    }
                }
            }
        }
        select {
            class: "form-select",
            "aria-label": "Task",
            value: "{task}",
            disabled: disabled || available.is_none() || selected_project.is_none(),
            onchange: move |e| task.set(e.value()),
            if disabled {
                option { value: "{task}",
                    {task_list.and_then(|ts| ts.iter().find(|t| Some(t.id) == task.read().parse().ok())).map(|t| t.name.clone()).unwrap_or_else(&*task)}
                }
            } else {
                option { value: "", "Select task…" }
                if let (Some(tasks), Some(contexts)) = (task_list, available) {
                    for t in tasks.iter().filter(|t| contexts.iter().any(|c| Some(c.project_id) == selected_project && c.task_id == t.id)) {
                        option { value: "{t.id}", "{t.name}" }
                    }
                    if selected_project.is_some() && !contexts.iter().any(|c| Some(c.project_id) == selected_project) {
                        option { value: "", "No enabled tasks" }
                    }
                }
            }
        }
    }
}
