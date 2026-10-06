use super::overview_tests::click;
use super::*;

fn view() -> project::ProjectDetailView {
    project::ProjectDetailView {
        requester: permission_editor::PermissionRequester {
            org_id: Uuid::from_u128(10),
            user_id: Uuid::from_u128(20),
        },
        canonical_permissions: true,
        project: project::ProjectDetails {
            id: Uuid::from_u128(1),
            name: "Visible project".into(),
            code: Some("PROJECT-1".into()),
            client_name: "Visible client".into(),
            currency: "EUR".into(),
            task_rate_currency: None,
            starts_on: None,
            ends_on: None,
            tags: vec![],
            admin_notes: None,
        },
        can_edit: true,
        team: vec![project::ProjectDetailIdentity {
            id: Uuid::from_u128(30),
            name: "Visible teammate".into(),
        }],
        tasks: vec![project::ProjectDetailIdentity {
            id: Uuid::from_u128(40),
            name: "Visible task".into(),
        }],
    }
}

fn probe() -> Probe {
    let probe = Probe {
        initial_path: Some(format!("/projects/{}", Uuid::from_u128(1))),
        ..Probe::default()
    };
    *probe.detail_view.borrow_mut() = Some(view());
    probe
}

fn mount(probe: &Probe) -> VirtualDom {
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    dom
}

#[tokio::test]
async fn detail_uses_bound_labels_and_existing_editor_without_directory_or_duplicate_forms() {
    let probe = probe();
    let dom = mount(&probe);
    let html = dioxus::ssr::render(&dom);
    for label in [
        "Visible project",
        "Visible client",
        "Visible teammate",
        "Visible task",
        "Edit project",
    ] {
        assert!(html.contains(label), "{html}");
    }
    for label in [
        "Assign User",
        "Enable task",
        "Create and enable task",
        "Fee balances",
        "project-task-rate",
    ] {
        assert!(!html.contains(label), "{html}");
    }
    assert!(
        html.contains(&format!("href=\"/projects/{}/edit\"", Uuid::from_u128(1))),
        "{html}"
    );
    assert_eq!(*probe.detail_view_requests.borrow(), [None]);
    assert!(probe.auxiliary_requesters.borrow().is_empty());
}

#[tokio::test]
async fn pending_and_denied_detail_never_mount_sensitive_children_or_render_raw_errors() {
    let probe = probe();
    let (send, receive) = oneshot::channel();
    *probe.detail_view_response.borrow_mut() = Some(receive);
    let mut dom = mount(&probe);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Loading project details"), "{html}");
    assert!(
        !html.contains("Visible teammate") && !html.contains("Edit project"),
        "{html}"
    );
    assert!(probe.auxiliary_requesters.borrow().is_empty());
    send.send(Err(ServerFnError::new("Private database diagnostic")))
        .unwrap();
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(
        html.contains("Project details are unavailable") && html.contains("Retry details"),
        "{html}"
    );
    assert!(
        !html.contains("Private database diagnostic") && !html.contains("Visible project"),
        "{html}"
    );
}

#[tokio::test]
async fn detail_refresh_keeps_initial_identity_and_rejects_wrong_project_or_policy() {
    for change in ["user", "organization", "policy", "project"] {
        let probe = probe();
        let initial = view();
        let mut dom = mount(&probe);
        let (send, receive) = oneshot::channel();
        *probe.detail_view_response.borrow_mut() = Some(receive);
        click(&mut dom, "project-detail-refresh");
        let html = dioxus::ssr::render(&dom);
        assert!(
            html.contains("Loading project details") && !html.contains("Visible teammate"),
            "{html}"
        );
        let mut changed = initial.clone();
        match change {
            "user" => changed.requester.user_id = Uuid::now_v7(),
            "organization" => changed.requester.org_id = Uuid::now_v7(),
            "policy" => changed.canonical_permissions = false,
            _ => changed.project.id = Uuid::now_v7(),
        }
        changed.project.name = "Other account project".into();
        send.send(Ok(changed)).unwrap();
        settle(&mut dom);
        let html = dioxus::ssr::render(&dom);
        assert!(
            html.contains("Project details are unavailable"),
            "{change}: {html}"
        );
        assert!(
            !html.contains("Other account project") && !html.contains("Visible teammate"),
            "{change}: {html}"
        );
        assert!(
            !html.contains("Edit project") && !html.contains("Fee balances"),
            "{html}"
        );
        click(&mut dom, "project-detail-retry");
        assert_eq!(
            *probe.detail_view_requests.borrow(),
            [None, Some(initial.requester), Some(initial.requester)]
        );
        assert!(dioxus::ssr::render(&dom).contains("Visible teammate"));
    }
}

#[tokio::test]
async fn detail_refresh_removes_edit_affordance_without_losing_authorized_labels() {
    let probe = probe();
    let mut dom = mount(&probe);
    probe.detail_view.borrow_mut().as_mut().unwrap().can_edit = false;
    click(&mut dom, "project-detail-refresh");
    let html = dioxus::ssr::render(&dom);
    assert!(
        html.contains("Visible teammate") && !html.contains("Edit project"),
        "{html}"
    );
}

#[tokio::test]
async fn late_fee_access_denial_hides_loaded_detail_and_retry_keeps_original_identity() {
    for status in [
        axum::http::StatusCode::UNAUTHORIZED,
        axum::http::StatusCode::FORBIDDEN,
    ] {
        let probe = probe();
        probe
            .detail_view
            .borrow_mut()
            .as_mut()
            .unwrap()
            .canonical_permissions = false;
        let (send, receive) = oneshot::channel();
        *probe.fee_response.borrow_mut() = Some(receive);
        let mut dom = mount(&probe);
        let html = dioxus::ssr::render(&dom);
        assert!(
            html.contains("Visible teammate") && html.contains("Edit project"),
            "{html}"
        );
        assert!(html.contains("Loading fee balances"), "{html}");
        assert_eq!(
            *probe.auxiliary_requesters.borrow(),
            [Some(view().requester)]
        );

        send.send(Err(ServerFnError::ServerError {
            code: status.as_u16(),
            message: "Private access diagnostic".into(),
            details: None,
        }))
        .unwrap();
        settle(&mut dom);
        let html = dioxus::ssr::render(&dom);
        assert!(
            html.contains("Project details are unavailable"),
            "{status}: {html}"
        );
        for hidden in [
            "Visible project",
            "Visible teammate",
            "Edit project",
            "Fee balances",
            "Private access diagnostic",
        ] {
            assert!(!html.contains(hidden), "{status}: {html}");
        }

        click(&mut dom, "project-detail-retry");
        assert_eq!(
            *probe.detail_view_requests.borrow(),
            [None, Some(view().requester)]
        );
        assert_eq!(
            *probe.auxiliary_requesters.borrow(),
            [Some(view().requester), Some(view().requester)]
        );
        assert!(dioxus::ssr::render(&dom).contains("Visible teammate"));
    }
}
