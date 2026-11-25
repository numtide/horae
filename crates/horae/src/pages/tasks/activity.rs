use super::*;
use crate::components::confirm_dialog::{ConfirmDialog, ConfirmTone};
use crate::components::menu::{MenuDivider, MenuItem, RowActions};
use crate::models::task::TaskCatalogEntry;

/// The task whose archive or restore confirmation is open.
#[derive(Clone, PartialEq)]
pub(super) struct ActivityTarget {
    pub task: TaskCatalogEntry,
    pub requester: PermissionRequester,
}

/// One task's "Actions" menu: Edit, then Archive or Restore.
#[component]
pub(super) fn TaskActions(
    task: TaskCatalogEntry,
    on_edit: Callback<TaskCatalogEntry>,
    on_activity: Callback<TaskCatalogEntry>,
) -> Element {
    let tr = crate::i18n::use_tr();
    let id = task.id;
    let active = task.active;
    let edited = task.clone();
    let edit = use_callback(move |_: MouseEvent| on_edit.call(edited.clone()));
    let changed = task.clone();
    let change = use_callback(move |_: MouseEvent| on_activity.call(changed.clone()));
    rsx! {
        RowActions { id: "task-actions-{id}", name: task.name.clone(),
            MenuItem { id: "tasks-edit-{id}", onclick: edit, {tr.action_edit()} }
            MenuDivider {}
            MenuItem { id: "task-activity-{id}", onclick: change,
                if active { {tr.action_archive()} } else { {tr.action_restore()} }
            }
        }
    }
}

/// Confirms archiving or restoring one task. Archiving carries the task out of
/// every project, and its toast has no Undo yet, so it asks first.
#[component]
pub(super) fn ActivityDialog(
    mut target: Signal<Option<ActivityTarget>>,
    on_changed: EventHandler<String>,
    on_denied: EventHandler<()>,
) -> Element {
    let tr = crate::i18n::use_tr();
    let mut busy = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let current = target();
    let active = current.as_ref().is_none_or(|open| open.task.active);
    let confirm = use_callback(move |()| {
        let Some(ActivityTarget { task, requester }) = target.peek().clone() else {
            return;
        };
        if busy() {
            return;
        }
        busy.set(true);
        error.set(None);
        spawn(async move {
            let result =
                server_fns::set_task_active(task.id.to_string(), !task.active, requester).await;
            busy.set(false);
            match result {
                Ok(_) => {
                    target.set(None);
                    on_changed.call(if task.active {
                        tr.tasks_archived_notice()
                    } else {
                        tr.tasks_restored_notice()
                    });
                }
                Err(ServerFnError::ServerError {
                    code: UNAUTHORIZED | FORBIDDEN,
                    ..
                }) => {
                    target.set(None);
                    on_denied.call(());
                }
                Err(ServerFnError::ServerError {
                    code: BAD_REQUEST | CONFLICT,
                    message,
                    ..
                }) => error.set(Some(message)),
                Err(_) => error.set(Some(tr.tasks_activity_failed())),
            }
        });
    });
    let keep = use_callback(move |()| {
        if !busy() {
            error.set(None);
            target.set(None);
        }
    });
    rsx! {
        ConfirmDialog { id: "task-activity-dialog", labelledby: "task-activity-title",
            open: current.is_some(), busy: busy(), tone: ConfirmTone::Neutral,
            focus_fallback: Some("tasks-title"),
            title: if active { tr.tasks_archive_title() } else { tr.tasks_restore_title() },
            confirm_label: if active { tr.tasks_archive_confirm() } else { tr.tasks_restore_confirm() },
            cancel_label: tr.tasks_activity_keep(),
            on_confirm: confirm, on_dismiss: keep,
            p {
                if active { {tr.tasks_archive_body()} } else { {tr.tasks_restore_body()} }
            }
            if let Some(message) = error() {
                p { class: "alert alert-danger", role: "alert", "{message}" }
            }
        }
    }
}
