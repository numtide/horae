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

/// A legacy overview whose project names interleave across two clients that
/// share a name, plus a case-only variant and a project without client details.
fn same_name_overview() -> project::ProjectOverview {
    let template = overview().projects.remove(0);
    let row = |project: u128, client: u128, name: &str, client_name: Option<&str>| {
        let mut row = template.clone();
        row.project.id = Uuid::from_u128(project);
        row.project.client_id = Uuid::from_u128(client);
        row.project.name = name.into();
        row.client = client_name.map(|name| project::ProjectOverviewClient {
            id: Uuid::from_u128(client),
            name: name.into(),
            active: true,
        });
        row
    };
    project::ProjectOverview {
        canonical_permissions: false,
        projects: vec![
            row(51, 41, "Alpha", Some("Same name")),
            row(52, 42, "Beta", Some("Same name")),
            row(53, 41, "Gamma", Some("Same name")),
            row(54, 43, "Delta", Some("same NAME")),
            row(55, 44, "Orphan", None),
        ],
        ..overview()
    }
}

fn position(html: &str, needle: &str) -> usize {
    html.find(needle)
        .unwrap_or_else(|| panic!("missing {needle}: {html}"))
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
async fn client_bands_link_each_client_by_identifier() {
    let probe = Probe {
        initial_path: Some("/projects".into()),
        ..Probe::default()
    };
    *probe.overview.borrow_mut() = Some(same_name_overview());
    let mut dom = mount(&probe);
    let html = dioxus::ssr::render(&dom);
    let client = |id: u128| format!("href=\"/clients/{}\"", Uuid::from_u128(id));
    let project = |id: u128| format!("href=\"/projects/{}\"", Uuid::from_u128(id));
    for id in [41, 42, 43] {
        assert_eq!(html.matches(&client(id)).count(), 1, "{id}: {html}");
    }
    assert!(!html.contains(&client(44)), "{html}");
    assert_eq!(html.matches("Unknown client").count(), 1, "{html}");
    // The fallback stays a plain label in the band's existing color.
    assert!(
        html.contains("text-primary\">Unknown client</div>"),
        "{html}"
    );
    // Each band holds exactly its own client's projects, in name order.
    let order = [
        position(&html, "Unknown client"),
        position(&html, &project(55)),
        position(&html, &client(41)),
        position(&html, &project(51)),
        position(&html, &project(53)),
        position(&html, &client(42)),
        position(&html, &project(52)),
        position(&html, &client(43)),
        position(&html, &project(54)),
    ];
    assert!(order.is_sorted(), "{order:?}: {html}");
    // Links come from the loaded overview; nothing else is read to render them.
    assert_eq!(*probe.overview_requests.borrow(), [None]);
    assert_eq!(*probe.client_catalog_reads.borrow(), 0);
    assert!(probe.client_requests.borrow().is_empty());
    assert!(probe.client_view_requests.borrow().is_empty());

    let navigator = probe.navigator.borrow().unwrap();
    let target = format!("/clients/{}", Uuid::from_u128(42));
    dom.in_scope(probe.scope.borrow().unwrap(), || navigator.push(target));
    settle(&mut dom);
    assert!(dioxus::ssr::render(&dom).contains("Client-42"));
    assert_eq!(*probe.client_requests.borrow(), [Uuid::from_u128(42)]);
    dom.in_scope(probe.scope.borrow().unwrap(), || navigator.go_back());
    settle(&mut dom);
    assert!(dioxus::ssr::render(&dom).contains(&client(42)));
}

#[tokio::test]
async fn policy_one_client_bands_stay_plain_labels() {
    let probe = Probe {
        initial_path: Some("/projects".into()),
        ..Probe::default()
    };
    *probe.overview.borrow_mut() = Some(overview());
    let dom = mount(&probe);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("text-primary\">Visible client</div>"), "{html}");
    assert!(!html.contains("href=\"/clients/"), "{html}");
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
