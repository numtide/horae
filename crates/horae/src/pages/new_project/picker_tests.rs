use dioxus::dioxus_core::NoOpMutations;
use futures_util::FutureExt;

use super::*;
use crate::models::project_creation::{CreationClient, CreationPerson, CreationTask, ProjectForm};

fn catalog() -> CreationOptions {
    CreationOptions {
        organization_currency: "EUR".into(),
        can_edit_cost_rates: true,
        can_edit_private_notes: true,
        email_available: false,
        clients: vec![CreationClient {
            id: Uuid::now_v7(),
            name: "Default client".into(),
            currency: "EUR".into(),
            active: true,
            default_rate_cents: None,
        }],
        tasks: vec![CreationTask {
            id: Uuid::now_v7(),
            name: "Default task".into(),
            billable: true,
            default_rate_cents: None,
            default_rate_currency: None,
        }],
        common_tasks: Vec::new(),
        people: vec![CreationPerson {
            id: Uuid::now_v7(),
            name: "Default teammate".into(),
            billable_rate_cents: None,
            cost_rate_cents: None,
        }],
        more_clients: false,
        more_tasks: false,
        more_people: false,
        previous_code: None,
        suggested_code: None,
    }
}

fn pickers() -> Element {
    let form = use_signal(ProjectForm::default);
    // Basics falls back to the editor's options while its search is unresolved.
    let options = use_signal(|| CreationOptions {
        clients: Vec::new(),
        ..catalog()
    });
    let defaults = use_signal(catalog);
    let managers = use_signal(|| None);
    let busy = use_signal(|| false);
    let archived_ids = use_signal(Default::default);
    let on_edit = use_callback(|_: ProtectedProjectField| {});
    rsx! {
        Basics { form, options, defaults, notes_access: ProjectFieldAccess::Editable, on_edit }
        Tasks { form, options, defaults, billable_access: ProjectFieldAccess::Editable, on_edit, archived_ids, retained_ids: Vec::new() }
        Team { form, managers, options, defaults, busy, billable_access: ProjectFieldAccess::Editable, cost_access: ProjectFieldAccess::Editable, on_edit }
    }
}

// Without a request context a catalog request cannot complete, so the default
// choices render only when the pickers reuse the editor's loaded catalog.
#[tokio::test]
async fn pickers_show_the_loaded_catalog_without_requesting_it_again() {
    let mut dom = VirtualDom::new(pickers);
    dom.rebuild_in_place();
    for _ in 0..20 {
        if dom.wait_for_work().now_or_never().is_none() {
            break;
        }
        dom.render_immediate(&mut NoOpMutations);
    }
    let html = dioxus::ssr::render(&dom);
    for name in ["Default client", "Default task", "Default teammate"] {
        assert!(html.contains(name), "{name} missing from {html}");
    }
    assert!(!html.contains("Searching tasks"), "{html}");
}
