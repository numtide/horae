use dioxus::prelude::*;

use super::{loaded, run_action};
use crate::components::badge::Badge;
use crate::components::form::{FormCard, FormGroup, Input, Select};
use crate::components::table::DataTable;
use crate::server_fns;

#[component]
pub fn AdminUsers() -> Element {
    let mut users = use_resource(|| async move { server_fns::list_users(true).await });
    let tasks = use_resource(|| async move { server_fns::list_tasks().await });

    let mut show_user_form = use_signal(|| false);
    let mut user_email = use_signal(String::new);
    let mut user_name = use_signal(String::new);
    let mut user_role = use_signal(|| "member".to_string());
    let user_error = use_signal(|| None::<String>);
    let mut row_error = use_signal(|| None::<String>);

    let mut show_task_form = use_signal(|| false);
    let mut task_name = use_signal(String::new);
    let mut task_billable = use_signal(|| true);
    let task_error = use_signal(|| None::<String>);

    rsx! {
        div {
            // ── Users section ───────────────────────────────────────────
            div { class: "page-header",
                h1 { class: "page-title", "User Management" }
                div { class: "page-actions",
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| show_user_form.set(!show_user_form()),
                        if show_user_form() { "Cancel" } else { "Invite User" }
                    }
                }
            }

            // Row actions sit outside the create form, where `user_error` is the
            // only banner, so their refusals need one of their own.
            if let Some(err) = row_error() {
                div { class: "alert alert-danger", "{err}" }
            }

            if show_user_form() {
                FormCard { title: "New User", error: user_error,
                    FormGroup { label: "Email", id: "user-email",
                        Input {
                            id: "user-email",
                            kind: "email",
                            placeholder: "user@example.com",
                            value: "{user_email}",
                            oninput: move |e: FormEvent| user_email.set(e.value()),
                        }
                    }
                    FormGroup { label: "Name", id: "user-name",
                        Input {
                            id: "user-name",
                            placeholder: "Full name",
                            value: "{user_name}",
                            oninput: move |e: FormEvent| user_name.set(e.value()),
                        }
                    }
                    FormGroup { label: "Role", id: "user-role",
                        Select {
                            id: "user-role",
                            options: vec![
                                ("member".to_string(), "Member".to_string()),
                                ("manager".to_string(), "Manager".to_string()),
                                ("admin".to_string(), "Admin".to_string()),
                            ],
                            selected: user_role(),
                            onchange: move |e: FormEvent| user_role.set(e.value()),
                        }
                    }
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| {
                            let e = user_email();
                            let n = user_name();
                            let r = user_role();
                            run_action(server_fns::create_user(e, n, r), users, user_error, move || {
                                user_email.set(String::new());
                                user_name.set(String::new());
                                user_role.set("member".to_string());
                                row_error.set(None);
                                show_user_form.set(false);
                            });
                        },
                        "Create User"
                    }
                }
            }

            div { class: "card",
                {loaded(&*users.read(), |user_list| rsx! {
                        DataTable {
                            table {
                                thead {
                                    tr {
                                        th { "Name" }
                                        th { "Email" }
                                        th { "Role" }
                                        th { "Status" }
                                        th { "Actions" }
                                    }
                                }
                                tbody {
                                    for user in user_list.iter() {
                                        {
                                            let uid = user.id.to_string();
                                            let uid2 = uid.clone();
                                            let current_role = user.org_role.to_string();
                                            let is_active = user.active;
                                            rsx! {
                                                tr { key: "{uid}",
                                                    td { "{user.name}" }
                                                    td { "{user.email}" }
                                                    td {
                                                        select {
                                                            class: "form-input py-1 px-2",
                                                            style: "width: auto; font-size: 0.8125rem;",
                                                            value: "{current_role}",
                                                            onchange: {
                                                                let uid = uid.clone();
                                                                move |e: Event<FormData>| {
                                                                    let uid = uid.clone();
                                                                    let new_role = e.value();
                                                                    spawn(async move {
                                                                        match server_fns::set_user_role(uid, new_role).await {
                                                                            Ok(_) => row_error.set(None),
                                                                            Err(e) => row_error.set(Some(e.to_string())),
                                                                        }
                                                                        // Refetch either way; a refusal reverts the control.
                                                                        users.restart();
                                                                    });
                                                                }
                                                            },
                                                            option { value: "member", "Member" }
                                                            option { value: "manager", "Manager" }
                                                            option { value: "admin", "Admin" }
                                                        }
                                                    }
                                                    td {
                                                        if is_active {
                                                            Badge { variant: "success", "Active" }
                                                        } else {
                                                            Badge { variant: "neutral", "Inactive" }
                                                        }
                                                    }
                                                    td {
                                                        button {
                                                            class: if is_active { "btn btn-secondary btn-sm" } else { "btn btn-primary btn-sm" },
                                                            onclick: {
                                                                let uid = uid2.clone();
                                                                move |_| {
                                                                    let uid = uid.clone();
                                                                    let new_active = !is_active;
                                                                    spawn(async move {
                                                                        match server_fns::set_user_active(uid, new_active).await {
                                                                            Ok(_) => row_error.set(None),
                                                                            Err(e) => row_error.set(Some(e.to_string())),
                                                                        }
                                                                        users.restart();
                                                                    });
                                                                }
                                                            },
                                                            if is_active { "Deactivate" } else { "Activate" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                })}
            }

            // ── Tasks section ───────────────────────────────────────────
            div { class: "mt-8",
                div { class: "page-header",
                    h2 { class: "page-title text-xl", "Tasks" }
                    div { class: "page-actions",
                        button {
                            class: "btn btn-primary",
                            onclick: move |_| show_task_form.set(!show_task_form()),
                            if show_task_form() { "Cancel" } else { "Add Task" }
                        }
                    }
                }

                if show_task_form() {
                    FormCard { title: "New Task", error: task_error,
                        FormGroup { label: "Name", id: "task-name",
                            Input {
                                id: "task-name",
                                placeholder: "Task name",
                                value: "{task_name}",
                                oninput: move |e: FormEvent| task_name.set(e.value()),
                            }
                        }
                        div { class: "form-group",
                            label { class: "form-label flex items-center gap-2",
                                input {
                                    r#type: "checkbox",
                                    checked: task_billable(),
                                    onchange: move |e| task_billable.set(e.checked()),
                                }
                                "Billable by default"
                            }
                        }
                        button {
                            class: "btn btn-primary",
                            onclick: move |_| {
                                let n = task_name();
                                let b = task_billable();
                                run_action(server_fns::create_task(n, b, None), tasks, task_error, move || {
                                    task_name.set(String::new());
                                    task_billable.set(true);
                                    show_task_form.set(false);
                                });
                            },
                            "Create Task"
                        }
                    }
                }

                div { class: "card",
                    {loaded(&*tasks.read(), |task_list| {
                        if task_list.is_empty() {
                            return rsx! {
                                p { class: "text-muted text-sm p-5", "No tasks defined yet." }
                            };
                        }
                        rsx! {
                            DataTable {
                                table {
                                    thead {
                                        tr {
                                            th { "Name" }
                                            th { "Billable Default" }
                                            th { "Active" }
                                        }
                                    }
                                    tbody {
                                        for task in task_list.iter() {
                                            tr { key: "{task.id}",
                                                td { "{task.name}" }
                                                td {
                                                    if task.billable_default {
                                                        Badge { variant: "success", "Yes" }
                                                    } else {
                                                        Badge { variant: "neutral", "No" }
                                                    }
                                                }
                                                td {
                                                    if task.active {
                                                        Badge { variant: "success", "Active" }
                                                    } else {
                                                        Badge { variant: "neutral", "Inactive" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    })}
                }
            }
        }
    }
}
