use super::*;
use overview_tests::click;

const NOT_FOUND: u16 = 404;
/// Markers for the legacy client mutation controls on the list.
const LIST_ACTIONS: [&str; 3] = [
    "id=\"clients-new\"",
    "id=\"client-edit-",
    "id=\"client-activity-",
];

pub(super) fn requester() -> permission_editor::PermissionRequester {
    permission_editor::PermissionRequester {
        org_id: Uuid::from_u128(10),
        user_id: Uuid::from_u128(20),
    }
}

pub(super) fn client_record(id: Uuid) -> client::Client {
    client::Client {
        id,
        org_id: Uuid::nil(),
        name: format!("Client-{}", id.as_u128()),
        currency: "EUR".into(),
        address: Some(format!("Address-{}", id.as_u128())),
        // Set, so "Not set" can only refer to the default rate.
        tax_id: Some(format!("Tax-{}", id.as_u128())),
        active: true,
        created_at: chrono::DateTime::UNIX_EPOCH,
    }
}

/// The backend's legacy-only management rule: policy one never manages yet.
pub(super) fn overview(probe: &Probe) -> client::ClientOverview {
    client::ClientOverview {
        requester: requester(),
        canonical_permissions: probe.canonical,
        can_read: true,
        can_manage: !probe.member && !probe.canonical,
        clients: vec![client::ClientSummary {
            client: client_record(Uuid::from_u128(1)),
            active_projects: 1,
            total_projects: 1,
            project_currencies: vec!["USD".into()],
        }],
    }
}

fn respond<T>(slot: &DeferredResponse<T>, value: Result<T, ServerFnError>) {
    let (send, receive) = oneshot::channel();
    assert!(send.send(value).is_ok());
    *slot.borrow_mut() = Some(receive);
}

fn mount(probe: &Probe) -> VirtualDom {
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    dom
}

fn list(probe: Probe) -> Probe {
    Probe {
        initial_path: Some("/clients".into()),
        ..probe
    }
}

fn detail(probe: Probe) -> Probe {
    Probe {
        initial_path: Some(format!("/clients/{}", Uuid::from_u128(1))),
        ..probe
    }
}

fn has_list_actions(html: &str) -> bool {
    LIST_ACTIONS.iter().any(|marker| html.contains(marker))
}

#[tokio::test]
async fn legacy_manager_list_keeps_rows_and_management_actions() {
    let dom = mount(&list(Probe::default()));
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Client-1"), "{html}");
    for marker in LIST_ACTIONS {
        assert!(html.contains(marker), "missing {marker}: {html}");
    }
    assert!(html.contains("href=\"/admin/importers\""), "{html}");
}

#[tokio::test]
async fn legacy_member_and_policy_one_readers_get_rows_without_legacy_writes() {
    for probe in [
        Probe {
            member: true,
            ..Probe::default()
        },
        // `get_me` still reports a legacy Admin role here; it must not grant writes.
        Probe {
            canonical: true,
            ..Probe::default()
        },
    ] {
        let probe = list(probe);
        let dom = mount(&probe);
        let html = dioxus::ssr::render(&dom);
        assert!(html.contains("Client-1"), "{html}");
        assert!(!has_list_actions(&html), "{html}");
        assert!(!html.contains("href=\"/admin/importers\""), "{html}");
        assert!(probe.client_status_changes.borrow().is_empty());
    }
}

#[tokio::test]
async fn denied_list_is_neither_empty_nor_populated() {
    let probe = list(Probe {
        canonical: true,
        ..Probe::default()
    });
    let mut denied = overview(&probe);
    // Fail closed even if a denied response were to carry rows.
    denied.can_read = false;
    respond(&probe.client_overview_response, Ok(denied));
    let dom = mount(&probe);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Client access is unavailable"), "{html}");
    for hidden in ["Client-1", "No clients yet", "Active clients"] {
        assert!(!html.contains(hidden), "shown {hidden}: {html}");
    }
    assert!(!has_list_actions(&html), "{html}");
}

#[tokio::test]
async fn readable_empty_list_is_a_genuine_empty_state() {
    let probe = list(Probe {
        canonical: true,
        ..Probe::default()
    });
    let mut empty = overview(&probe);
    empty.clients.clear();
    respond(&probe.client_overview_response, Ok(empty));
    let dom = mount(&probe);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("No clients yet"), "{html}");
    assert!(!html.contains("Client access is unavailable"), "{html}");
    assert!(!has_list_actions(&html), "{html}");
}

#[tokio::test]
async fn list_loading_and_errors_never_claim_empty_or_denied() {
    let probe = list(Probe::default());
    let (send, receive) = oneshot::channel();
    *probe.client_overview_response.borrow_mut() = Some(receive);
    let mut dom = mount(&probe);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Loading clients"), "{html}");
    assert!(!has_list_actions(&html), "{html}");
    send.send(Err(ServerFnError::new("Clients unavailable")))
        .unwrap();
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Could not load clients"), "{html}");
    for hidden in ["No clients yet", "Client access is unavailable", "Client-1"] {
        assert!(!html.contains(hidden), "shown {hidden}: {html}");
    }
    assert!(!has_list_actions(&html), "{html}");
    click(&mut dom, "clients-retry");
    assert!(dioxus::ssr::render(&dom).contains("Client-1"));
    // A failed load binds nothing, so the retry is still unbound.
    assert_eq!(*probe.client_overview_requests.borrow(), [None, None]);
}

#[tokio::test]
async fn pending_refresh_withholds_previous_manager_controls_and_editor() {
    let probe = list(Probe::default());
    let id = Uuid::from_u128(1);
    let mut dom = mount(&probe);
    click(&mut dom, &format!("client-edit-{id}"));
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Save changes"), "{html}");
    let (send, receive) = oneshot::channel();
    *probe.client_overview_response.borrow_mut() = Some(receive);
    click(&mut dom, &format!("client-activity-{id}"));
    assert_eq!(*probe.client_status_changes.borrow(), [(id, false)]);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Loading clients"), "{html}");
    for hidden in [
        "Client-1",
        "Address-1",
        "Save changes",
        "href=\"/admin/importers\"",
    ] {
        assert!(!html.contains(hidden), "stale {hidden}: {html}");
    }
    assert!(!has_list_actions(&html), "{html}");
    assert_eq!(
        *probe.client_overview_requests.borrow(),
        [None, Some(requester())]
    );
    send.send(Ok(overview(&probe))).unwrap();
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(
        html.contains("Client-1") && has_list_actions(&html),
        "{html}"
    );
    assert!(!html.contains("Save changes"), "editor reopened: {html}");
}

#[tokio::test]
async fn list_refresh_rejects_changed_account_organization_or_policy() {
    for change in ["user", "organization", "policy"] {
        let probe = list(Probe::default());
        let id = Uuid::from_u128(1);
        let mut dom = mount(&probe);
        let (send, receive) = oneshot::channel();
        *probe.client_overview_response.borrow_mut() = Some(receive);
        click(&mut dom, &format!("client-activity-{id}"));
        let mut changed = overview(&probe);
        match change {
            "user" => changed.requester.user_id = Uuid::now_v7(),
            "organization" => changed.requester.org_id = Uuid::now_v7(),
            _ => {
                changed.canonical_permissions = true;
                changed.can_manage = false;
            }
        }
        changed.clients[0].client.name = "Other account client".into();
        send.send(Ok(changed)).unwrap();
        settle(&mut dom);
        let html = dioxus::ssr::render(&dom);
        assert!(html.contains("Could not load clients"), "{change}: {html}");
        for hidden in ["Other account client", "Client-1"] {
            assert!(!html.contains(hidden), "{change} shown {hidden}: {html}");
        }
        assert!(!has_list_actions(&html), "{change}: {html}");
    }
}

#[tokio::test]
async fn policy_one_rate_reader_sees_rate_without_management_or_invoices() {
    let probe = detail(Probe {
        canonical: true,
        ..Probe::default()
    });
    let dom = mount(&probe);
    let html = dioxus::ssr::render(&dom);
    for expected in ["Client-1", "Address-1", "Default rate", "EUR 0.00 / h"] {
        assert!(html.contains(expected), "missing {expected}: {html}");
    }
    // The editor's dialog title is always mounted; the button is the second mention.
    assert_eq!(html.matches("Edit client").count(), 1, "{html}");
    for prefix in ["/projects/new/client/", "/invoices/new/client/"] {
        assert!(!html.contains(prefix), "{html}");
    }
    assert!(!html.contains("Invoices"), "{html}");
    assert!(probe.client_invoice_requests.borrow().is_empty());
    assert_eq!(*probe.client_view_requests.borrow(), [None]);
    // Project context stays reachable without client management.
    assert!(html.contains("href=\"/projects/client/"), "{html}");
}

#[tokio::test]
async fn policy_one_detail_distinguishes_restricted_from_unset_rate() {
    for (billing, rate_shown, unset_shown) in [
        (None, false, false),
        (
            Some(client::ClientBilling {
                default_rate_cents: None,
            }),
            true,
            true,
        ),
    ] {
        let probe = detail(Probe {
            canonical: true,
            ..Probe::default()
        });
        let id = Uuid::from_u128(1);
        respond(
            &probe.client_view_response,
            Ok(client::ClientDetailView {
                requester: requester(),
                canonical_permissions: true,
                can_manage: false,
                details: client::ClientDetails {
                    client: client_record(id),
                    billing,
                },
            }),
        );
        let dom = mount(&probe);
        let html = dioxus::ssr::render(&dom);
        assert!(html.contains("Address-1"), "{html}");
        assert_eq!(html.contains("Default rate"), rate_shown, "{html}");
        assert_eq!(html.contains("Not set"), unset_shown, "{html}");
        assert!(!html.contains("/ h"), "{html}");
        assert!(!html.contains("Invoices"), "{html}");
        assert!(probe.client_invoice_requests.borrow().is_empty());
    }
}

#[tokio::test]
async fn hidden_or_missing_client_detail_reveals_no_identity() {
    let probe = detail(Probe {
        canonical: true,
        ..Probe::default()
    });
    respond(
        &probe.client_view_response,
        Err(ServerFnError::ServerError {
            code: NOT_FOUND,
            message: "Client not found".into(),
            details: None,
        }),
    );
    let dom = mount(&probe);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Could not load this client"), "{html}");
    for hidden in ["Client-1", "Address-1", "Default rate", "Invoices"] {
        assert!(!html.contains(hidden), "shown {hidden}: {html}");
    }
    assert!(probe.client_invoice_requests.borrow().is_empty());
}

#[tokio::test]
async fn client_project_context_keeps_its_existing_reader() {
    let id = Uuid::from_u128(1);
    let probe = Probe {
        canonical: true,
        initial_path: Some(format!("/projects/client/{id}")),
        ..Probe::default()
    };
    mount(&probe);
    assert!(probe.client_requests.borrow().contains(&id));
    assert!(probe.client_view_requests.borrow().is_empty());
    assert!(probe.client_overview_requests.borrow().is_empty());
}
