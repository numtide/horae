use std::collections::HashMap;

use dioxus::prelude::*;
use tracing::error;
use uuid::Uuid;

use crate::components::project_task_picker::ProjectTaskPicker;
use crate::server_fns;

/// The running timer, owned by the app shell so everything that can change it
/// reads the same state: the rail renders from it, and a page that starts a
/// timer refreshes it without knowing the rail exists.
#[derive(Clone, Copy)]
pub struct RunningTimer {
    entry: Resource<Result<Option<crate::models::TimeEntry>, ServerFnError>>,
    changes: Signal<u64>,
}

impl RunningTimer {
    /// Invalidate after any change to time entries.
    ///
    /// This is the single call sites make: it re-reads the timer and, through
    /// [`Self::changes`], the entry lists that follow it. Pages must not restart
    /// their own entry resource instead — a mutation can delete or alter the
    /// running entry, and refreshing only the page leaves the rail counting
    /// against a row that is gone.
    pub fn refresh(&mut self) {
        self.entry.restart();
        *self.changes.write() += 1;
    }

    /// Bumped on every start or stop attempt, refusals included: a refusal
    /// usually means the entry changed underneath.
    ///
    /// Reading it subscribes the caller, so a resource that reads it re-runs on
    /// those events — which is how a page's entry list follows a timer started
    /// from the rail, which knows nothing about the page. Pages track this
    /// rather than the timer resource itself, whose pending-to-resolved step on
    /// mount would cost them a second fetch of data that has not changed.
    pub fn changes(&self) -> u64 {
        *self.changes.read()
    }
}

/// Call once, above every component that reads or changes the timer.
pub fn use_running_timer_provider() {
    let entry = use_resource(|| async move { server_fns::get_current_timer().await });
    let changes = use_signal(|| 0u64);
    use_context_provider(|| RunningTimer { entry, changes });
}

/// The shared timer. Panics if no provider is above the caller, which would
/// be a wiring mistake rather than a runtime condition.
pub fn use_running_timer() -> RunningTimer {
    use_context::<RunningTimer>()
}

/// The sidebar timer. Idle shows a "Start timer" button; picking a project/task
/// starts a running timer; while running it shows the live elapsed time, the
/// project, and a Stop button. It lives in the sidebar so it's reachable from
/// every page (Harvest-style), not just the timesheet.
#[component]
pub fn TimerWidget() -> Element {
    // A 1s tick re-renders this component so the running display counts up.
    let mut tick = use_signal(|| 0u64);
    let _tick = *tick.read();
    use_hook(|| {
        spawn(async move {
            loop {
                #[cfg(feature = "web")]
                gloo_timers::future::TimeoutFuture::new(1_000).await;
                #[cfg(feature = "server")]
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                tick += 1;
            }
        });
    });

    let mut timer = use_running_timer();
    let timer_resource = timer.entry;
    let projects = use_resource(|| async move { server_fns::list_projects(None, true).await });

    let mut picking = use_signal(|| false);
    let mut selected_project = use_signal(String::new);
    let mut selected_task = use_signal(String::new);
    let mut notes = use_signal(String::new);
    let mut action_error = use_signal(|| None::<String>);
    let mut saving = use_signal(|| false);

    // Task names are shared by the running label and the project-scoped picker.
    let all_tasks = use_resource(|| async move { server_fns::list_tasks().await });

    // Only resource changes rebuild these lookups, not the one-second tick.
    let project_names = use_memo(move || -> HashMap<Uuid, String> {
        projects
            .read()
            .as_ref()
            .and_then(|r| r.as_ref().ok())
            .map(|ps| ps.iter().map(|p| (p.id, p.name.clone())).collect())
            .unwrap_or_default()
    });

    let task_names = use_memo(move || -> HashMap<Uuid, String> {
        all_tasks
            .read()
            .as_ref()
            .and_then(|r| r.as_ref().ok())
            .map(|ts| ts.iter().map(|t| (t.id, t.name.clone())).collect())
            .unwrap_or_default()
    });

    let current_timer = timer_resource
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .cloned()
        .flatten();
    let is_running = current_timer.is_some();

    // Elapsed = time since it started, plus any minutes already banked on the entry.
    let (hours, minutes, seconds) = match current_timer
        .as_ref()
        .and_then(|e| e.started_at.map(|s| (e, s)))
    {
        Some((entry, started_at)) => {
            let elapsed = (chrono::Utc::now() - started_at).num_seconds().max(0) as u64;
            let total = elapsed + entry.minutes as u64 * 60;
            (total / 3600, (total % 3600) / 60, total % 60)
        }
        None => (0, 0, 0),
    };

    // The kit labels the running pill "Project · Task"; fall back to whichever
    // half resolves.
    let running_label = current_timer.as_ref().map(|e| {
        let project_names = project_names.read();
        let task_names = task_names.read();
        match (project_names.get(&e.project_id), task_names.get(&e.task_id)) {
            (Some(project), Some(task)) => format!("{project} · {task}"),
            (Some(project), None) => project.clone(),
            (None, Some(task)) => task.clone(),
            (None, None) => "Running".to_string(),
        }
    });

    let handle_start = move |_| {
        if saving() {
            return;
        }
        let proj = selected_project.read().clone();
        let task = selected_task.read().clone();
        if proj.is_empty() || task.is_empty() {
            action_error.set(Some("Select a project and task.".to_string()));
            return;
        }
        let note = notes.read().trim().to_string();
        let note = (!note.is_empty()).then_some(note);
        saving.set(true);
        action_error.set(None);
        spawn(async move {
            match server_fns::start_timer(proj, task, note).await {
                Ok(_) => {
                    picking.set(false);
                    notes.set(String::new());
                    selected_project.set(String::new());
                    selected_task.set(String::new());
                    timer.refresh();
                }
                Err(e) => {
                    error!("Start timer error: {e}");
                    action_error.set(Some(e.to_string()));
                }
            }
            saving.set(false);
        });
    };

    let entry_id_for_stop = current_timer.as_ref().map(|e| e.id.to_string());
    let handle_stop = move |_| {
        if saving() {
            return;
        }
        if let Some(eid) = entry_id_for_stop.clone() {
            saving.set(true);
            action_error.set(None);
            spawn(async move {
                if let Err(e) = server_fns::stop_timer(eid).await {
                    error!("Stop timer error: {e}");
                    action_error.set(Some(e.to_string()));
                }
                // Re-read even after a refusal: the entry is usually already
                // gone, and skipping this leaves the rail ticking against a
                // timer the server has dropped.
                timer.refresh();
                saving.set(false);
            });
        }
    };

    rsx! {
        div { class: "sidebar-timer-wrap",
            if let Some(message) = action_error() {
                div { class: "alert alert-danger", role: "alert", "{message}" }
            }
            if is_running {
                div { class: "sidebar-timer-live",
                    span { class: "sidebar-timer-dot", "aria-hidden": "true" }
                    div { class: "sidebar-timer-info",
                        div { class: "sidebar-timer-time", "{hours}:{minutes:02}:{seconds:02}" }
                        // The collapsed rail's 44px chip has no room for seconds.
                        div { class: "sidebar-timer-time-short", "{hours}:{minutes:02}" }
                        div { class: "sidebar-timer-proj",
                            {running_label.unwrap_or_else(|| "Running".into())}
                        }
                    }
                    button {
                        class: "sidebar-timer-stop",
                        disabled: saving(),
                        "aria-label": "Stop timer",
                        title: "Stop timer",
                        onclick: handle_stop,
                        span { class: "sidebar-timer-stop-square", "aria-hidden": "true" }
                    }
                }
            } else {
                button {
                    class: "sidebar-timer",
                    onclick: move |_| picking.set(true),
                    span { class: "sidebar-timer-icon" }
                    span { class: "sidebar-timer-label", "Start timer" }
                }
                if picking() {
                    div { class: "menu-overlay", onclick: move |_| picking.set(false) }
                    div { class: "sidebar-timer-pop menu",
                        div { class: "sidebar-timer-pop-head",
                            span { "Start timer" }
                            button {
                                class: "sidebar-timer-pop-close",
                                "aria-label": "Close",
                                onclick: move |_| picking.set(false),
                                "×"
                            }
                        }
                        div { class: "sidebar-timer-pop-body",
                            label { class: "form-label", "Project / Task" }
                            ProjectTaskPicker { project: selected_project, task: selected_task, projects, tasks: all_tasks }
                            textarea {
                                class: "form-input form-textarea",
                                placeholder: "Notes (optional)",
                                value: "{notes}",
                                oninput: move |e| notes.set(e.value()),
                            }
                            div { class: "sidebar-timer-form-actions",
                                button { class: "btn btn-primary", disabled: saving(), onclick: handle_start, "Start timer" }
                                button {
                                    class: "btn btn-ghost",
                                    onclick: move |_| picking.set(false),
                                    "Cancel"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
