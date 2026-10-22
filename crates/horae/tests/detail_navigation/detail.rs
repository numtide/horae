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
        client_id: Uuid::from_u128(50),
        project_type: horae_core::types::ProjectType::FixedFee,
        active: true,
        can_edit: true,
        can_open_client: true,
        can_change_status: false,
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

/// The detail Edit link, which returns the editor to this same project's detail.
fn edit_href(project: Uuid) -> String {
    format!("href=\"/projects/{project}/edit?return_to=/projects/{project}\"")
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
    assert!(html.contains(&edit_href(Uuid::from_u128(1))), "{html}");
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

/// Rendered menu item labels, so closed-dialog copy cannot satisfy an action check.
fn menu_items(html: &str) -> Vec<String> {
    html.split("role=\"menuitem\"")
        .skip(1)
        .filter_map(|item| {
            let (_, rest) = item.split_once('>')?;
            let (label, _) = rest.split_once("</button>")?;
            let mut label = label.to_string();
            while let Some(start) = label.find("<!--") {
                let end = start + label[start..].find("-->")? + 3;
                label.replace_range(start..end, "");
            }
            Some(label.trim().to_string())
        })
        .collect()
}

/// The only policy that offers status changes: `set_project_active` checks the legacy role.
fn legacy_probe() -> Probe {
    let probe = probe();
    {
        let mut view = probe.detail_view.borrow_mut();
        let view = view.as_mut().unwrap();
        view.canonical_permissions = false;
        view.can_change_status = true;
    }
    probe
}

#[tokio::test]
async fn header_shows_real_identity_and_only_the_offered_actions() {
    let project = Uuid::from_u128(1);
    let legacy = legacy_probe();
    let dom = mount(&legacy);
    let html = dioxus::ssr::render(&dom);
    for expected in [
        "[PROJECT-1] Visible project".to_string(),
        "Fixed Fee".into(),
        "EUR".into(),
        "Back to Projects".into(),
        "href=\"/projects\"".into(),
        format!("href=\"/clients/{}\"", Uuid::from_u128(50)),
        edit_href(project),
        "project-detail-actions-trigger".into(),
    ] {
        assert!(html.contains(&expected), "missing {expected}: {html}");
    }
    assert_eq!(menu_items(&html), ["Archive"], "{html}");
    for hidden in ["Reactivate", "Archived"] {
        assert!(!html.contains(hidden), "{hidden}: {html}");
    }

    // Policy one offers no role-derived status change, even with edit authority.
    let canonical = probe();
    let html = dioxus::ssr::render(&mount(&canonical));
    assert!(
        html.contains("[PROJECT-1] Visible project") && html.contains(&edit_href(project)),
        "{html}"
    );
    assert!(!html.contains("project-detail-actions"), "{html}");
    assert!(menu_items(&html).is_empty(), "{html}");

    let restricted = legacy_probe();
    {
        let mut view = restricted.detail_view.borrow_mut();
        let view = view.as_mut().unwrap();
        view.project.code = None;
        view.active = false;
        view.can_open_client = false;
        view.can_change_status = false;
        view.can_edit = false;
    }
    let html = dioxus::ssr::render(&mount(&restricted));
    assert!(
        html.contains("Visible project") && !html.contains("[PROJECT-1]"),
        "{html}"
    );
    assert!(
        html.contains("Visible client") && html.contains("Archived"),
        "{html}"
    );
    for hidden in [
        "href=\"/clients/".to_string(),
        // Any edit link, with or without a return target.
        format!("href=\"/projects/{project}/edit"),
        "project-detail-actions-trigger".into(),
    ] {
        assert!(!html.contains(&hidden), "{hidden}: {html}");
    }
    assert!(menu_items(&html).is_empty(), "{html}");
    assert!(html.contains("href=\"/projects\""), "{html}");
}

#[tokio::test]
async fn archived_project_offers_reactivation_and_keeps_saved_information() {
    let legacy = legacy_probe();
    {
        let mut view = legacy.detail_view.borrow_mut();
        let view = view.as_mut().unwrap();
        view.active = false;
        view.project.tags = vec!["retained-tag".into()];
        view.project.admin_notes = Some("Retained private note".into());
        view.project.starts_on = chrono::NaiveDate::from_ymd_opt(2026, 9, 1);
    }
    let html = dioxus::ssr::render(&mount(&legacy));
    for expected in [
        "Archived",
        "Project information",
        "retained-tag",
        "Retained private note",
        "01 Sep 2026",
    ] {
        assert!(html.contains(expected), "missing {expected}: {html}");
    }
    assert_eq!(menu_items(&html), ["Reactivate"], "{html}");
}

#[tokio::test]
async fn back_navigation_stays_available_while_loading_and_after_denial() {
    let probe = probe();
    let (send, receive) = oneshot::channel();
    *probe.detail_view_response.borrow_mut() = Some(receive);
    let mut dom = mount(&probe);
    let html = dioxus::ssr::render(&dom);
    assert!(
        html.contains("Back to Projects") && !html.contains("project-detail-actions"),
        "{html}"
    );
    send.send(Err(ServerFnError::new("denied"))).unwrap();
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(
        html.contains("Back to Projects") && !html.contains("project-detail-actions"),
        "{html}"
    );
}
