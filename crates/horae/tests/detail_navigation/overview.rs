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
    assert!(
        html.contains("text-primary\">Visible client</div>"),
        "{html}"
    );
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

/// A legacy manager's overview: three "Alpha" projects and "Gamma" under Acme,
/// and "Alpha four" under Beta Corp.
fn managed_overview() -> project::ProjectOverview {
    let template = overview().projects.remove(0);
    let row = |project: u128, client: u128, name: &str, client_name: &str| {
        let mut row = template.clone();
        row.project.id = Uuid::from_u128(project);
        row.project.client_id = Uuid::from_u128(client);
        row.project.name = name.into();
        row.client = Some(project::ProjectOverviewClient {
            id: Uuid::from_u128(client),
            name: client_name.into(),
            active: true,
        });
        row
    };
    project::ProjectOverview {
        canonical_permissions: false,
        can_change_legacy_status: true,
        projects: vec![
            row(61, 41, "Alpha one", "Acme"),
            row(62, 41, "Alpha two", "Acme"),
            row(63, 41, "Alpha three", "Acme"),
            row(64, 41, "Gamma", "Acme"),
            row(65, 42, "Alpha four", "Beta Corp"),
        ],
        ..overview()
    }
}

/// Holds the next overview read until the returned sender answers it.
fn hold_overview(
    probe: &Probe,
) -> oneshot::Sender<Result<project::ProjectOverview, ServerFnError>> {
    let (send, receive) = oneshot::channel();
    *probe.overview_response.borrow_mut() = Some(receive);
    send
}

fn checked(html: &str, project: &str) -> bool {
    html.contains(&format!(
        "aria-label=\"Select {project}\" aria-checked=\"true\""
    ))
}

#[tokio::test]
async fn project_actions_keep_filters_selection_and_the_list_mounted() {
    let probe = Probe {
        initial_path: Some("/projects".into()),
        allow_project_activity: true,
        ..Probe::default()
    };
    *probe.overview.borrow_mut() = Some(managed_overview());
    let mut dom = mount(&probe);
    type_into(&mut dom, "Search by project or client", "alpha");
    press(&mut dom, By::Text("All clients▾"), false);
    press(&mut dom, By::Text("Acme"), false);
    for project in ["Alpha one", "Alpha two", "Alpha three"] {
        press(&mut dom, By::Label(&format!("Select {project}")), false);
    }
    let filtered = |html: &str| {
        html.contains("value=\"alpha\"")
            && html.contains("Acme<span class=\"ml-2 text-faint\">▾")
            && !html.contains("Alpha four")
            && !html.contains("Gamma")
    };
    let html = dioxus::ssr::render(&dom);
    assert!(filtered(&html), "{html}");
    assert!(
        checked(&html, "Alpha two") && checked(&html, "Alpha three"),
        "{html}"
    );

    // A row action: the reload keeps the stale list on screen, marked busy.
    let reload = hold_overview(&probe);
    press(&mut dom, By::Text("Archive"), false);
    assert_eq!(
        *probe.project_status_changes.borrow(),
        [(vec![Uuid::from_u128(61)], false)]
    );
    let html = dioxus::ssr::render(&dom);
    assert!(
        !html.contains("Loading projects") && html.contains("aria-busy=\"true\""),
        "{html}"
    );
    assert!(
        filtered(&html) && html.contains("Select Alpha one"),
        "{html}"
    );
    reload
        .send(Ok(probe.overview.borrow().clone().unwrap()))
        .unwrap();
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(
        filtered(&html) && !html.contains("Select Alpha one"),
        "{html}"
    );
    assert!(
        checked(&html, "Alpha two") && checked(&html, "Alpha three"),
        "{html}"
    );
    assert!(!html.contains("aria-busy=\"true\""), "{html}");

    // A bulk action over part of the selection.
    press(&mut dom, By::Label("Select Alpha three"), false);
    let reload = hold_overview(&probe);
    press(&mut dom, By::Text("Archive projects"), false);
    press(&mut dom, By::Text("Archive projects"), true);
    assert_eq!(
        probe.project_status_changes.borrow()[1],
        (vec![Uuid::from_u128(62)], false)
    );
    let html = dioxus::ssr::render(&dom);
    assert!(
        !html.contains("Loading projects") && filtered(&html),
        "{html}"
    );
    reload
        .send(Ok(probe.overview.borrow().clone().unwrap()))
        .unwrap();
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(html.contains("Archived 1 project"), "{html}");
    assert!(
        filtered(&html) && html.contains("Select Alpha three"),
        "{html}"
    );
    assert!(!html.contains("Select Alpha two"), "{html}");

    // A failed reload shows its error beside the filters it kept.
    press(&mut dom, By::Label("Select Alpha three"), false);
    let reload = hold_overview(&probe);
    press(&mut dom, By::Text("Archive"), false);
    reload
        .send(Err(ServerFnError::ServerError {
            code: 500,
            message: "unavailable".into(),
            details: None,
        }))
        .unwrap();
    settle(&mut dom);
    let html = dioxus::ssr::render(&dom);
    assert!(
        html.contains("Could not load projects") && filtered(&html),
        "{html}"
    );
    assert!(!html.contains("Project access is unavailable"), "{html}");
    press(&mut dom, By::Text("Retry"), false);
    let html = dioxus::ssr::render(&dom);
    assert!(
        filtered(&html) && html.contains("No projects match your filters"),
        "{html}"
    );

    // ProjectListContent starts three auxiliary reads each time it mounts.
    assert_eq!(probe.auxiliary_requesters.borrow().len(), 3);
}

/// What identifies an event target: its `id`, its `aria-label`, or its whole text.
#[derive(Clone, Copy)]
enum By<'a> {
    Id(&'a str),
    Label(&'a str),
    Text(&'a str),
}

/// The text a template node renders, including its mounted children.
fn node_text(dom: &VirtualDom, vnode: &VNode, node: &dioxus::core::TemplateNode) -> String {
    use dioxus::core::{DynamicNode, TemplateNode};
    match node {
        TemplateNode::Text { text } => text.to_string(),
        TemplateNode::Element { children, .. } => children
            .iter()
            .map(|child| node_text(dom, vnode, child))
            .collect(),
        TemplateNode::Dynamic { id } => match &vnode.dynamic_nodes[*id] {
            DynamicNode::Text(text) => text.value.clone(),
            DynamicNode::Fragment(nodes) => {
                nodes.iter().map(|node| vnode_text(dom, node)).collect()
            }
            DynamicNode::Component(component) => component
                .mounted_scope(*id, vnode, dom)
                .and_then(|scope| scope.try_root_node())
                .map(|node| vnode_text(dom, node))
                .unwrap_or_default(),
            DynamicNode::Placeholder(_) => String::new(),
        },
    }
}

fn vnode_text(dom: &VirtualDom, vnode: &VNode) -> String {
    vnode
        .template
        .roots
        .iter()
        .map(|root| node_text(dom, vnode, root))
        .collect()
}

/// Every mounted element matching `by`, in document order.
fn targets(dom: &VirtualDom, vnode: &VNode, by: By, found: &mut Vec<dioxus::core::ElementId>) {
    use dioxus::core::{AttributeValue, DynamicNode, TemplateAttribute, TemplateNode};
    'paths: for (index, path) in vnode.template.attr_paths.iter().enumerate() {
        let mut node = &vnode.template.roots[usize::from(path[0])];
        for child in &path[1..] {
            let TemplateNode::Element { children, .. } = node else {
                continue 'paths;
            };
            node = &children[usize::from(*child)];
        }
        let matched = match by {
            By::Text(name) => node_text(dom, vnode, node).trim() == name,
            By::Id(name) | By::Label(name) => {
                let key = if matches!(by, By::Id(_)) {
                    "id"
                } else {
                    "aria-label"
                };
                let static_attr = matches!(node, TemplateNode::Element { attrs, .. }
                    if attrs.iter().any(|attr| matches!(attr, TemplateAttribute::Static { name: attr, value, .. } if *attr == key && *value == name)));
                static_attr
                    || vnode.dynamic_attrs[index].iter().any(|attr| {
                        attr.name == key
                            && matches!(&attr.value, AttributeValue::Text(value) if value == name)
                    })
            }
        };
        if matched {
            found.extend(vnode.mounted_dynamic_attribute(index, dom));
        }
    }
    for (index, node) in vnode.dynamic_nodes.iter().enumerate() {
        match node {
            DynamicNode::Component(component) => {
                if let Some(node) = component
                    .mounted_scope(index, vnode, dom)
                    .and_then(|scope| scope.try_root_node())
                {
                    targets(dom, node, by, found);
                }
            }
            DynamicNode::Fragment(nodes) => {
                for node in nodes {
                    targets(dom, node, by, found);
                }
            }
            _ => {}
        }
    }
}

/// Dispatch `name` to the first element matching `by`, or the last with `last`
/// (a dialog's button follows the menu item of the same name).
fn fire(dom: &mut VirtualDom, by: By, last: bool, name: &str, data: PlatformEventData) {
    set_event_converter(Box::new(dioxus_html::SerializedHtmlEventConverter));
    let mut found = Vec::new();
    targets(dom, dom.base_scope().root_node(), by, &mut found);
    let id = if last { found.last() } else { found.first() };
    let id = *id.unwrap_or_else(|| panic!("missing event target: {}", dioxus::ssr::render(dom)));
    dom.runtime().handle_event(
        name,
        Event::new(Rc::new(data) as Rc<dyn std::any::Any>, true),
        id,
    );
    settle(dom);
}

fn press(dom: &mut VirtualDom, by: By, last: bool) {
    let data = PlatformEventData::new(Box::<SerializedMouseData>::default());
    fire(dom, by, last, "click", data);
}

pub(super) fn click(dom: &mut VirtualDom, name: &str) {
    press(dom, By::Id(name), false);
}

fn type_into(dom: &mut VirtualDom, label: &str, value: &str) {
    let data = PlatformEventData::new(Box::new(dioxus_html::SerializedFormData::new(
        value.into(),
        vec![],
    )));
    fire(dom, By::Label(label), false, "input", data);
}
