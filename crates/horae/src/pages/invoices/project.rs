use super::preparation::{PrepareInvoice, ProjectSource};
use super::recovery::{RecoveryGate, RecoveryStorage};
use super::*;

#[component]
pub fn NewProjectInvoice(id: Uuid) -> Element {
    rsx! { for id in [id] {
        RecoveryGate { key: "{id}", AuthorizedProjectInvoice { id } }
    } }
}

#[component]
fn AuthorizedProjectInvoice(id: Uuid) -> Element {
    let storage = use_context::<RecoveryStorage>();
    rsx! { if storage.ready() { ProjectInvoiceContent { id } } }
}

#[component]
fn ProjectInvoiceContent(id: Uuid) -> Element {
    let mut project =
        use_resource(move || async move { server_fns::get_project_details(id.to_string()).await });
    let busy = use_signal(|| false);
    let navigator = use_navigator();
    rsx! {
        div {
            div { class: "page-header",
                h1 { class: "page-title", "New invoice" }
                button { r#type: "button", class: "btn btn-secondary min-h-control", disabled: busy(),
                    onclick: move |_| { if !busy() { navigator.push(Route::ProjectDetail { id }); } },
                    "Cancel"
                }
            }
            if project.state()() != UseResourceState::Ready {
                p { role: "status", "Loading invoice project…" }
            } else {
                match &*project.read() {
                    Some(Ok(project)) => rsx! { PrepareInvoice {
                        client_opts: Vec::new(), busy,
                        project: ProjectSource {
                            project_id: project.id, project_name: project.name.clone(),
                            client_id: project.client_id, client_name: project.client_name.clone(),
                        },
                        oncreated: move |id| { navigator.push(Route::InvoiceDetail { id }); },
                    } },
                    Some(Err(error)) => rsx! {
                        p { role: "alert", class: "text-danger", "Could not load invoice project: {error}" }
                        button { r#type: "button", class: "btn btn-secondary min-h-control", onclick: move |_| project.restart(), "Retry project" }
                    },
                    None => rsx! {},
                }
            }
        }
    }
}
