use super::*;

fn overview() -> project::ProjectOverview {
    let client = project::ProjectOverviewClient {
        id: Uuid::from_u128(40),
        name: "Visible client".into(),
        active: false,
    };
    project::ProjectOverview {
        requester: permission_editor::PermissionRequester {
            org_id: Uuid::from_u128(10),
            user_id: Uuid::from_u128(20),
        },
        canonical_permissions: true,
        can_create: false,
        can_import: false,
        can_change_legacy_status: false,
        projects: vec![project::ProjectOverviewRow {
            project: project::Project {
                id: Uuid::from_u128(30),
                org_id: Uuid::from_u128(10),
                client_id: client.id,
                code: None,
                name: "Visible project".into(),
                project_type: horae_core::types::ProjectType::TimeAndMaterials,
                currency: "EUR".into(),
                rate_cents: None,
                starts_on: None,
                ends_on: None,
                budget_kind: horae_core::types::BudgetKind::None,
                budget_amount_cents: None,
                budget_minutes: None,
                active: true,
                created_at: chrono::DateTime::UNIX_EPOCH,
            },
            client: Some(client),
            can_edit: true,
        }],
    }
}

fn mount(probe: &Probe) -> VirtualDom {
    let mut dom = VirtualDom::new_with_props(app, probe.clone());
    dom.rebuild_in_place();
    settle(&mut dom);
    dom
}

#[tokio::test]
async fn overview_uses_workflow_client_labels_and_binds_auxiliary_reads() {
    let probe = Probe {
        initial_path: Some("/projects".into()),
        ..Probe::default()
    };
    let response = overview();
    *probe.overview.borrow_mut() = Some(response.clone());
    let dom = mount(&probe);
    let html = dioxus::ssr::render(&dom);
    assert!(
        html.contains("Visible client") && html.contains("Visible project"),
        "{html}"
    );
    assert_eq!(*probe.client_catalog_reads.borrow(), 0);
    assert_eq!(*probe.overview_requests.borrow(), [None]);
    assert!(
        html.contains(&format!("expected_org_id={}", response.requester.org_id)),
        "{html}"
    );
    assert!(
        html.contains(&format!("expected_user_id={}", response.requester.user_id)),
        "{html}"
    );
    assert_eq!(
        *probe.auxiliary_requesters.borrow(),
        vec![Some(response.requester); 3]
    );
    assert!(
        html.contains("project-actions-"),
        "authorized editor must have its own actions: {html}"
    );
    assert!(
        !html.contains("project-bulk-menu"),
        "canonical edit is not legacy lifecycle authority: {html}"
    );
    assert!(
        !html.contains("href=\"/projects/new\"") && !html.contains("href=\"/admin/importers\""),
        "{html}"
    );
}

#[tokio::test]
async fn pending_or_denied_overview_starts_no_auxiliary_reads_or_actions() {
    let probe = Probe {
        initial_path: Some("/projects".into()),
        ..Probe::default()
    };
    let (send, receive) = oneshot::channel();
    *probe.overview_response.borrow_mut() = Some(receive);
    let mut dom = mount(&probe);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Loading projects"), "{html}");
    assert!(probe.auxiliary_requesters.borrow().is_empty());
    assert_eq!(*probe.client_catalog_reads.borrow(), 0);
    assert!(
        !html.contains("Export") && !html.contains("project-bulk-menu"),
        "{html}"
    );
    send.send(Err(ServerFnError::ServerError {
        code: 403,
        message: "private diagnostic".into(),
        details: None,
    }))
    .unwrap();
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(
        html.contains("Project access is unavailable") && html.contains("Retry access"),
        "{html}"
    );
    assert!(
        !html.contains("private diagnostic") && !html.contains("Export"),
        "{html}"
    );
    assert!(probe.auxiliary_requesters.borrow().is_empty());
}

#[tokio::test]
async fn overview_refresh_rejects_changed_account_organization_or_policy_without_old_rows() {
    for change in ["user", "organization", "policy"] {
        let probe = Probe {
            initial_path: Some("/projects".into()),
            ..Probe::default()
        };
        let initial = overview();
        *probe.overview.borrow_mut() = Some(initial.clone());
        let (deny, receive) = oneshot::channel();
        *probe.tag_response.borrow_mut() = Some(receive);
        let mut dom = mount(&probe);
        assert!(dioxus::ssr::render(&dom).contains("Visible project"));
        deny.send(Err(ServerFnError::ServerError {
            code: 403,
            message: "requester changed".into(),
            details: None,
        }))
        .unwrap();
        settle(&mut dom);
        let html = dioxus::ssr::render(&dom);
        assert!(
            html.contains("Refresh access") && !html.contains("Visible project"),
            "{html}"
        );
        assert!(
            !html.contains("Export") && !html.contains("project-actions-"),
            "{html}"
        );
        let (send, receive) = oneshot::channel();
        *probe.overview_response.borrow_mut() = Some(receive);
        click(&mut dom, "projects-refresh-access");
        let html = dioxus::ssr::render(&dom);
        assert!(
            html.contains("Loading projects") && !html.contains("Visible client"),
            "{html}"
        );
        assert_eq!(
            *probe.overview_requests.borrow(),
            [None, Some(initial.requester)]
        );
        let mut changed = initial.clone();
        match change {
            "user" => changed.requester.user_id = Uuid::now_v7(),
            "organization" => changed.requester.org_id = Uuid::now_v7(),
            _ => changed.canonical_permissions = false,
        }
        changed.projects[0].project.name = "Other account project".into();
        send.send(Ok(changed)).unwrap();
        settle(&mut dom);
        let html = dioxus::ssr::render(&dom);
        assert!(
            html.contains("Project access is unavailable"),
            "{change}: {html}"
        );
        assert!(
            !html.contains("Other account project") && !html.contains("Visible project"),
            "{change}: {html}"
        );
        assert_eq!(
            *probe.auxiliary_requesters.borrow(),
            vec![Some(initial.requester); 3]
        );
    }
}

fn target(dom: &VirtualDom, vnode: &VNode, name: &str) -> Option<dioxus::core::ElementId> {
    use dioxus::core::{AttributeValue, DynamicNode, TemplateAttribute, TemplateNode};
    for (index, path) in vnode.template.attr_paths.iter().enumerate() {
        let mut node = &vnode.template.roots[usize::from(path[0])];
        for child in &path[1..] {
            let TemplateNode::Element { children, .. } = node else {
                return None;
            };
            node = &children[usize::from(*child)];
        }
        let static_id = matches!(node, TemplateNode::Element { attrs, .. }
            if attrs.iter().any(|attr| matches!(attr, TemplateAttribute::Static { name:"id", value, .. } if *value == name)));
        let dynamic_id = vnode.dynamic_attrs[index].iter().any(|attr| {
            attr.name == "id" && matches!(&attr.value, AttributeValue::Text(value) if value == name)
        });
        if static_id || dynamic_id {
            return vnode.mounted_dynamic_attribute(index, dom);
        }
    }
    for (index, node) in vnode.dynamic_nodes.iter().enumerate() {
        let found = match node {
            DynamicNode::Component(component) => component
                .mounted_scope(index, vnode, dom)
                .and_then(|scope| scope.try_root_node())
                .and_then(|node| target(dom, node, name)),
            DynamicNode::Fragment(nodes) => nodes.iter().find_map(|node| target(dom, node, name)),
            _ => None,
        };
        if found.is_some() {
            return found;
        }
    }
    None
}

pub(super) fn click(dom: &mut VirtualDom, name: &str) {
    set_event_converter(Box::new(dioxus_html::SerializedHtmlEventConverter));
    let id = target(dom, dom.base_scope().root_node(), name).expect("missing event target");
    dom.runtime().handle_event(
        "click",
        Event::new(
            Rc::new(PlatformEventData::new(Box::<SerializedMouseData>::default()))
                as Rc<dyn std::any::Any>,
            true,
        ),
        id,
    );
    settle(dom);
}
