#![cfg(feature = "server")]

//! Render the production approval page with controlled record responses.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use dioxus::core::Mutation;
use dioxus::prelude::*;
use futures_util::FutureExt;
use horae_core::types::OrgRole;
use uuid::Uuid;

#[path = "../src/models/approval.rs"]
pub mod approval_model;
#[path = "../src/components/avatar.rs"]
pub mod avatar;
#[path = "../src/components/badge.rs"]
pub mod badge;
#[path = "../src/components/controls.rs"]
pub mod controls;
#[path = "../src/components/table.rs"]
pub mod table;
#[path = "../src/models/user.rs"]
pub mod user_model;
mod components {
    pub use super::{avatar, badge, controls, table};
}
#[path = "../src/pages/approvals.rs"]
mod approvals;

#[derive(Clone, Default)]
struct Probe {
    rows: Vec<approval_model::ApprovalSummary>,
    manager: bool,
    pending: bool,
    error: Option<String>,
    directory_reads: Rc<Cell<usize>>,
    actions: Rc<RefCell<Vec<Vec<String>>>>,
}

fn is_manager(me: &Resource<Result<user_model::User, ServerFnError>>) -> bool {
    matches!(&*me.read(), Some(Ok(user)) if user.is_manager_or_above())
}

fn loaded<T>(
    state: &Option<Result<T, ServerFnError>>,
    render: impl FnOnce(&T) -> Element,
) -> Element {
    match state {
        Some(Ok(value)) => render(value),
        Some(Err(error)) => rsx! { div { class: "alert alert-danger", "{error}" } },
        None => rsx! { div { class: "text-muted text-sm", "Loading…" } },
    }
}

fn app(probe: Probe) -> Element {
    use_context_provider(|| probe);
    rsx! { approvals::Approvals {} }
}

fn settle(dom: &mut VirtualDom) -> Vec<Mutation> {
    let mut edits = Vec::new();
    for _ in 0..20 {
        if dom.wait_for_work().now_or_never().is_none() {
            return edits;
        }
        edits.extend(dom.render_immediate_to_vec().edits);
    }
    panic!("approval resources did not settle");
}

fn render(probe: &Probe) -> (VirtualDom, Vec<Mutation>) {
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    let mut edits = dom.rebuild_to_vec().edits;
    edits.extend(settle(&mut dom));
    (dom, edits)
}

fn row(name: &str) -> approval_model::ApprovalSummary {
    serde_json::from_value(serde_json::json!({
        "approval": {"id":Uuid::now_v7(),"org_id":Uuid::now_v7(),"user_id":Uuid::now_v7(),
            "period_start":"2026-09-21","period_end":"2026-09-27","state":"submitted",
            "submitted_at":"2026-09-27T12:00:00Z","approved_by":null,"approved_at":null},
        "user_name":name,"total_minutes":90,"billable_minutes":60
    }))
    .unwrap()
}

#[tokio::test]
async fn approval_names_render_from_rows_without_directory_and_escape_markup() {
    let probe = Probe {
        manager: true,
        rows: vec![row("Álvaro <script> & 王"), row("Álvaro <script> & 王")],
        ..Default::default()
    };
    let (dom, _) = render(&probe);
    let html = dioxus::ssr::render(&dom);
    assert_eq!(probe.directory_reads.get(), 0);
    assert_eq!(
        html.matches("Álvaro &#60;script&#62; &#38; 王").count(),
        2,
        "{html}"
    );
    assert!(!html.contains("<script>"));
    assert!(!html.contains(&probe.rows[0].approval.user_id.to_string()));
    assert!(html.contains("1:30"));
    assert!(probe.actions.borrow().is_empty());
}

#[tokio::test]
async fn approval_loading_error_empty_and_denied_states_need_no_directory() {
    for (pending, error, manager, expected) in [
        (true, None, true, "Loading"),
        (
            false,
            Some("Approvals unavailable".to_owned()),
            true,
            "Approvals unavailable",
        ),
        (false, None, true, "No approvals found"),
        (false, None, false, "Manager or admin access is required"),
    ] {
        let probe = Probe {
            pending,
            error,
            manager,
            ..Default::default()
        };
        let (dom, _) = render(&probe);
        assert!(dioxus::ssr::render(&dom).contains(expected));
        assert_eq!(probe.directory_reads.get(), 0);
        assert!(probe.actions.borrow().is_empty());
    }
}

#[tokio::test]
async fn single_approval_action_keeps_the_record_id_not_the_person_label() {
    dioxus::html::set_event_converter(Box::new(dioxus_html::SerializedHtmlEventConverter));
    let probe = Probe {
        manager: true,
        rows: vec![row("Same name")],
        ..Default::default()
    };
    let (mut dom, edits) = render(&probe);
    let clicks: Vec<_> = edits
        .into_iter()
        .filter_map(|edit| match edit {
            Mutation::NewEventListener { name, id } if name == "click" => Some(id),
            _ => None,
        })
        .collect();
    // Three status filters, then the row's Reopen/Approve and bulk Approve.
    assert_eq!(clicks.len(), 6);
    let event = Event::new(
        Rc::new(PlatformEventData::new(Box::<
            dioxus_html::SerializedMouseData,
        >::default())) as Rc<dyn std::any::Any>,
        true,
    );
    dom.runtime().handle_event("click", event, clicks[4]);
    settle(&mut dom);
    assert_eq!(
        *probe.actions.borrow(),
        vec![vec![probe.rows[0].approval.id.to_string()]]
    );
    assert_eq!(probe.directory_reads.get(), 0);
}

pub mod server_fns {
    use super::*;

    pub async fn get_me() -> Result<user_model::User, ServerFnError> {
        let probe = use_context::<Probe>();
        Ok(user_model::User {
            id: Uuid::now_v7(),
            org_id: Uuid::now_v7(),
            name: "Reviewer".into(),
            email: "reviewer@example.test".into(),
            oidc_subject: None,
            org_role: if probe.manager {
                OrgRole::Manager
            } else {
                OrgRole::Member
            },
            cost_rate_cents: None,
            billable_rate_cents: None,
            active: true,
            created_at: chrono::Utc::now(),
        })
    }

    pub async fn list_approvals(
        _: Option<String>,
    ) -> Result<Vec<approval_model::ApprovalSummary>, ServerFnError> {
        let probe = use_context::<Probe>();
        if probe.pending {
            std::future::pending::<()>().await;
        }
        if let Some(error) = probe.error {
            return Err(ServerFnError::new(error));
        }
        Ok(probe.rows)
    }

    pub async fn list_users(_: bool) -> Result<Vec<user_model::UserListItem>, ServerFnError> {
        let probe = use_context::<Probe>();
        probe.directory_reads.set(probe.directory_reads.get() + 1);
        Err(ServerFnError::new("Directory must not be needed"))
    }

    pub async fn approve_submission(id: String) -> Result<approval_model::Approval, ServerFnError> {
        let probe = use_context::<Probe>();
        probe.actions.borrow_mut().push(vec![id]);
        Ok(probe.rows[0].approval.clone())
    }

    pub async fn approve_submissions(ids: Vec<String>) -> Result<usize, ServerFnError> {
        let probe = use_context::<Probe>();
        let count = ids.len();
        probe.actions.borrow_mut().push(ids);
        Ok(count)
    }

    pub async fn reject_submission(id: String) -> Result<(), ServerFnError> {
        use_context::<Probe>().actions.borrow_mut().push(vec![id]);
        Ok(())
    }
}
