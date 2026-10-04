#![cfg(feature = "server")]

use std::any::Any;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use dioxus::core::{AttributeValue, ElementId, Mutation};
use dioxus::prelude::*;
use dioxus_html::SerializedHtmlEventConverter;
use futures_util::FutureExt;
use tokio::sync::oneshot;
use uuid::Uuid;

#[path = "../src/components/permission_description.rs"]
pub mod permission_description;
mod components {
    pub use super::permission_description;
}
#[path = "../src/models/permission_audit.rs"]
pub mod permission_audit;
#[path = "../src/models/permission_editor.rs"]
pub mod permission_editor;
mod models {
    pub use super::{permission_audit, permission_editor};
}
#[path = "../src/pages/permission_audit.rs"]
mod page;

use permission_audit::*;
use permission_editor::PermissionRequester;
type Response = Result<AuditPage, ServerFnError>;
type Request = (Option<AuditCursor>, Option<PermissionRequester>);

#[derive(Clone, Default)]
struct Probe {
    replies: Rc<RefCell<VecDeque<oneshot::Receiver<Response>>>>,
    requests: Rc<RefCell<Vec<Request>>>,
}

impl Probe {
    fn reply(&self) -> oneshot::Sender<Response> {
        let (send, receive) = oneshot::channel();
        self.replies.borrow_mut().push_back(receive);
        send
    }
}

fn app(probe: Probe) -> Element {
    use_context_provider(|| probe);
    rsx! { page::PermissionAudit {} }
}

struct Harness {
    dom: VirtualDom,
    controls: HashMap<String, ElementId>,
}

impl Harness {
    fn new(probe: &Probe) -> Self {
        set_event_converter(Box::new(SerializedHtmlEventConverter));
        let mut dom = VirtualDom::new_with_props(app, probe.clone());
        let edits = dom.rebuild_to_vec().edits;
        let mut harness = Self {
            dom,
            controls: HashMap::new(),
        };
        harness.record(edits);
        harness.settle();
        harness
    }

    fn record(&mut self, edits: Vec<Mutation>) {
        for edit in edits {
            if let Mutation::SetAttribute {
                name: "id",
                value: AttributeValue::Text(value),
                id,
                ..
            } = edit
            {
                self.controls.insert(value, id);
            }
        }
    }

    fn settle(&mut self) {
        for _ in 0..30 {
            if self.dom.wait_for_work().now_or_never().is_none() {
                return;
            }
            let edits = self.dom.render_immediate_to_vec().edits;
            self.record(edits);
        }
        panic!("audit resource did not settle");
    }

    fn click(&mut self, name: &str) {
        let event = Event::new(
            Rc::new(PlatformEventData::new(Box::<SerializedMouseData>::default())) as Rc<dyn Any>,
            true,
        );
        self.dom
            .runtime()
            .handle_event("click", event, self.controls[name]);
        self.settle();
    }

    fn html(&self) -> String {
        dioxus::ssr::render(&self.dom)
    }
}

fn requester() -> PermissionRequester {
    PermissionRequester {
        org_id: Uuid::now_v7(),
        user_id: Uuid::now_v7(),
    }
}

fn history(requester: PermissionRequester, older: bool) -> AuditPage {
    let entry = AuditEntry {
        id: Uuid::now_v7(),
        actor: AuditPrincipal::User {
            user_id: requester.user_id,
        },
        created_at: "2026-10-04T10:00:00Z".parse().unwrap(),
        audit: HistoricalAudit::Profile(ProfileAudit {
            user_id: Uuid::now_v7(),
            previous_access_revision: 7,
            access_revision: 7,
            change: None,
        }),
    };
    AuditPage {
        requester,
        next_after: older.then_some(AuditCursor {
            created_at: entry.created_at,
            id: entry.id,
        }),
        entries: vec![entry],
    }
}

#[tokio::test]
async fn pages_bind_requester_replace_history_and_hide_stale_content_during_refresh() {
    let probe = Probe::default();
    let first = probe.reply();
    let mut h = Harness::new(&probe);
    assert_eq!(*probe.requests.borrow(), vec![(None, None)]);
    assert!(h.html().contains("Loading permission history"));
    h.click("audit-refresh");
    assert_eq!(probe.requests.borrow().len(), 1);
    let owner = requester();
    let page = history(owner, true);
    let id = page.entries[0].id.to_string();
    let cursor = page.next_after;
    first.send(Ok(page)).unwrap();
    h.settle();
    assert!(h.html().contains(&id));
    assert!(h.html().contains("No permission change"));
    let second = probe.reply();
    h.click("audit-older");
    assert!(!h.html().contains(&id));
    assert_eq!(probe.requests.borrow()[1], (cursor, Some(owner)));
    second
        .send(Ok(AuditPage {
            requester: owner,
            entries: vec![],
            next_after: None,
        }))
        .unwrap();
    h.settle();
    assert!(h.html().contains("No older permission events"));
    assert!(!h.html().contains(&id));
    let refresh = probe.reply();
    h.click("audit-newest");
    assert_eq!(probe.requests.borrow()[2], (None, Some(owner)));
    refresh
        .send(Err(ServerFnError::ServerError {
            code: 403,
            message: "private diagnostic".into(),
            details: None,
        }))
        .unwrap();
    h.settle();
    assert!(!h.html().contains("private diagnostic"));
    assert!(h.html().contains("Administrator access"));
    let retry = probe.reply();
    h.click("audit-refresh");
    retry.send(Ok(history(owner, false))).unwrap();
    h.settle();
    assert!(h.html().contains("No more permission events"));
}

#[tokio::test]
async fn different_requester_response_is_never_rendered_or_adopted() {
    let probe = Probe::default();
    let first = probe.reply();
    let mut h = Harness::new(&probe);
    let owner = requester();
    first.send(Ok(history(owner, false))).unwrap();
    h.settle();
    let refresh = probe.reply();
    h.click("audit-refresh");
    let other = history(requester(), false);
    let other_id = other.entries[0].id.to_string();
    refresh.send(Ok(other)).unwrap();
    h.settle();
    assert!(!h.html().contains(&other_id));
    assert!(h.html().contains("Administrator access"));
    let _retry = probe.reply();
    h.click("audit-refresh");
    assert_eq!(probe.requests.borrow().last().unwrap().1, Some(owner));
}

mod server_fns {
    use super::*;
    pub async fn list_permission_audit(
        after: Option<AuditCursor>,
        expected: Option<PermissionRequester>,
    ) -> Response {
        let probe = consume_context::<Probe>();
        probe.requests.borrow_mut().push((after, expected));
        let reply = probe
            .replies
            .borrow_mut()
            .pop_front()
            .expect("unexpected history request");
        reply.await.expect("resolve controlled history response")
    }
}

#[tokio::test]
async fn history_details_preserve_recorded_grants_sources_detachment_and_operator_identity() {
    use horae_core::permissions::catalog::{BuiltInProfile, Permission};
    let probe = Probe::default();
    let response = probe.reply();
    let mut h = Harness::new(&probe);
    let mut page = history(requester(), false);
    let person_id = Uuid::now_v7();
    let template_id = Uuid::now_v7();
    let project_id = Uuid::now_v7();
    let manager_id = Uuid::now_v7();
    let entry = &mut page.entries[0];
    entry.actor = AuditPrincipal::Operator {
        invocation_id: "recorded invocation".into(),
        command: "permission migration".into(),
    };
    entry.audit = HistoricalAudit::Profile(ProfileAudit {
        user_id: person_id,
        previous_access_revision: 7,
        access_revision: 8,
        change: Some(ProfileChange {
            catalog_version: 1,
            before: PersonSnapshot {
                grants: vec![Permission::TimeReadOwn],
                is_administrator: true,
                source: HistoricalSource::BuiltIn(BuiltInProfile::Administrator),
                revision: 1,
            },
            after: PersonSnapshot {
                grants: vec![Permission::TimeReadManaged],
                is_administrator: false,
                source: HistoricalSource::Template {
                    id: template_id,
                    applied_revision: 3,
                },
                revision: 2,
            },
            removed_projects: vec![RemovedRelationship {
                id: Uuid::now_v7(),
                subject_id: project_id,
                revision: 2,
            }],
            removed_people: vec![RemovedRelationship {
                id: Uuid::now_v7(),
                subject_id: manager_id,
                revision: 1,
            }],
        }),
    });
    let mut deleted = entry.clone();
    deleted.id = Uuid::now_v7();
    deleted.audit = HistoricalAudit::Template(TemplateAudit {
        previous_access_revision: 8,
        access_revision: 9,
        after: None,
        before: Some(TemplateSnapshot {
            id: template_id,
            name: "<script>not markup</script>".into(),
            catalog_version: 1,
            grants: vec![Permission::TimeReadOwn],
            revision: 3,
        }),
        detached_people: vec![DetachedPerson {
            user_id: person_id,
            catalog_version: 1,
            grants: vec![Permission::TimeReadOwn],
            is_administrator: false,
            previous_template_id: template_id,
            previous_applied_revision: 3,
            previous_revision: 2,
            revision: 3,
        }],
    });
    let mut managers = entry.clone();
    managers.id = Uuid::now_v7();
    managers.audit = HistoricalAudit::ProjectManagers(ProjectAudit {
        project_id,
        previous_access_revision: 9,
        access_revision: 10,
        change: Some(ProjectChange {
            added: vec![Designation {
                id: Uuid::now_v7(),
                manager_id,
                revision: 1,
            }],
            removed: vec![Designation {
                id: Uuid::now_v7(),
                manager_id: person_id,
                revision: 2,
            }],
        }),
    });
    page.entries.extend([deleted, managers]);
    response.send(Ok(page)).unwrap();
    h.settle();
    let html = h.html();
    for expected in [
        "Before",
        "After",
        "Administrator: Yes",
        "Administrator: No",
        "View your own time",
        "View time for managed people and projects",
        "applied revision 3",
        "Custom profile deleted",
        "Detached person",
        "individual configuration",
        "Removed managed project",
        "Removed managed person",
        "Added manager",
        "Removed manager",
        "permission migration",
        "recorded invocation",
        "UTC",
    ] {
        assert!(html.contains(expected), "missing {expected}: {html}");
    }
    assert!(html.contains(&project_id.to_string()));
    assert!(html.contains(&person_id.to_string()));
    assert!(
        html.contains("&#60;script&#62;not markup&#60;/script&#62;"),
        "{html}"
    );
    assert!(!html.contains("<script>not markup</script>"));
    assert_eq!(html.matches("<details").count(), 3);
    assert!(!html.contains("style="));
    assert!(!html.contains("90 days"));
}

#[tokio::test]
async fn initial_empty_and_failed_responses_are_distinct_and_sanitized() {
    let probe = Probe::default();
    let response = probe.reply();
    let mut h = Harness::new(&probe);
    response
        .send(Ok(AuditPage {
            requester: requester(),
            entries: vec![],
            next_after: None,
        }))
        .unwrap();
    h.settle();
    assert!(h.html().contains("No permission events recorded yet"));
    for (code, message) in [
        (401, "Sign in again"),
        (500, "Could not load permission history"),
    ] {
        let response = probe.reply();
        h.click("audit-refresh");
        response
            .send(Err(ServerFnError::ServerError {
                code,
                message: "private diagnostic".into(),
                details: None,
            }))
            .unwrap();
        h.settle();
        assert!(h.html().contains(message));
        assert!(!h.html().contains("private diagnostic"));
        assert!(!h.html().contains("No permission events recorded yet"));
    }
}
