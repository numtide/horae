use super::*;
use horae_core::permissions::catalog::{PERMISSION_CATALOG_VERSION, Permission};
use uuid::Uuid;

fn snapshot() -> OwnPermissions {
    OwnPermissions {
        catalog_version: PERMISSION_CATALOG_VERSION,
        grants: vec![Permission::TimeReadOwn, Permission::InvoiceReadManaged],
        is_administrator: false,
        access_revision: 928314,
        person_revision: 817243,
        managed_person_ids: vec![Uuid::now_v7()],
        managed_project_ids: vec![Uuid::now_v7(), Uuid::now_v7()],
    }
}

fn render(
    pending: bool,
    response: Option<Result<Option<OwnPermissions>, ServerFnError>>,
) -> String {
    dioxus::ssr::render_element(permission_content(pending, &response))
}

#[test]
fn own_permissions_view_loading_hides_previous_snapshot() {
    let html = render(true, Some(Ok(Some(snapshot()))));
    assert!(html.contains("Loading your permissions"), "{html}");
    assert!(!html.contains("View your own time"), "{html}");
    assert!(!html.contains("Managed projects"), "{html}");
}

#[test]
fn own_permissions_view_legacy_is_not_empty_or_a_profile() {
    let html = render(false, Some(Ok(None)));
    assert!(
        html.contains("Detailed permissions are not enabled"),
        "{html}"
    );
    assert!(!html.contains("No configured permissions"), "{html}");
    assert!(!html.contains("Member"), "{html}");
}

#[test]
fn own_permissions_view_preserves_exact_grants_without_profile_inference() {
    let html = render(false, Some(Ok(Some(snapshot()))));
    assert!(html.contains("View your own time"), "{html}");
    assert!(
        html.contains("View invoices for managed projects"),
        "{html}"
    );
    assert!(!html.contains("Edit your own time"), "{html}");
    assert!(!html.contains("Project Manager"), "{html}");
    assert!(!html.contains("Administrator access"), "{html}");
}

#[test]
fn own_permissions_view_does_not_disclose_internal_fields_or_offer_edits() {
    let own = snapshot();
    let ids = own
        .managed_person_ids
        .iter()
        .chain(&own.managed_project_ids)
        .copied()
        .collect::<Vec<_>>();
    let html = render(false, Some(Ok(Some(own))));
    for id in ids {
        assert!(!html.contains(&id.to_string()), "{html}");
    }
    for private in [
        "928314",
        "817243",
        "<input",
        "<select",
        "Update permissions",
        "href=",
        "style=",
    ] {
        assert!(!html.contains(private), "unexpected {private}: {html}");
    }
    assert!(html.contains("Managed people"), "{html}");
    assert!(html.contains("Managed projects"), "{html}");
    assert!(html.contains("do not grant access on their own"), "{html}");
}

#[test]
fn own_permissions_view_admin_identity_is_explicit_not_derived_from_grants() {
    let mut own = snapshot();
    own.grants = Permission::ALL.to_vec();
    assert!(!render(false, Some(Ok(Some(own.clone())))).contains("Administrator access"));
    own.is_administrator = true;
    own.grants.clear();
    assert!(render(false, Some(Ok(Some(own)))).contains("Administrator access"));
}

#[test]
fn own_permissions_view_empty_grants_are_not_normalized() {
    let mut own = snapshot();
    own.grants.clear();
    let html = render(false, Some(Ok(Some(own))));
    assert!(html.contains("No configured permissions"), "{html}");
    assert!(!html.contains("View your own time"), "{html}");
}

#[test]
fn own_permissions_view_distinguishes_authentication_forbidden_and_failure() {
    for (code, expected) in [
        (401, "Sign in again"),
        (403, "Ask an Administrator"),
        (500, "Could not load your permissions"),
    ] {
        let html = render(
            false,
            Some(Err(ServerFnError::ServerError {
                code,
                message: "private diagnostic".into(),
                details: None,
            })),
        );
        assert!(html.contains(expected), "{html}");
        assert!(html.contains("role=\"alert\""), "{html}");
        assert!(!html.contains("private diagnostic"), "{html}");
        assert!(!html.contains("View your own time"), "{html}");
    }
}

#[test]
fn own_permissions_view_unsupported_catalog_hides_uninterpretable_grants() {
    let mut own = snapshot();
    own.catalog_version += 1;
    let html = render(false, Some(Ok(Some(own))));
    assert!(html.contains("Could not load your permissions"), "{html}");
    assert!(!html.contains("View your own time"), "{html}");
}

#[test]
fn own_permissions_view_grants_do_not_promise_unavailable_features() {
    let mut own = snapshot();
    own.grants = vec![
        Permission::SavedReportReadInactive,
        Permission::SavedReportWriteInactive,
    ];
    let html = render(false, Some(Ok(Some(own))));
    assert!(
        html.contains("View saved reports owned by inactive people"),
        "{html}"
    );
    assert!(
        html.contains("Edit saved reports owned by inactive people"),
        "{html}"
    );
    assert!(
        html.contains("do not enable unavailable features"),
        "{html}"
    );
}
