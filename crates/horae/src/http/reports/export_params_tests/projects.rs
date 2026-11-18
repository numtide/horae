use super::*;

fn authorize(query: &str, org_id: Uuid, user_id: Uuid) -> Result<(), StatusCode> {
    let uri = format!("/api/projects/export/csv?{query}").parse().unwrap();
    Query::<ProjectsExportParams>::try_from_uri(&uri)
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .0
        .authorize_requester(org_id, user_id)
}

#[test]
fn project_download_accepts_legacy_links_and_matching_requester() {
    let org = Uuid::from_u128(1);
    let user = Uuid::from_u128(2);
    for scope in ["active", "archived", "budgeted"] {
        assert_eq!(authorize(&format!("scope={scope}"), org, user), Ok(()));
        assert_eq!(
            authorize(
                &format!("scope={scope}&expected_org_id={org}&expected_user_id={user}"),
                org,
                user
            ),
            Ok(())
        );
    }
}

#[test]
fn project_download_rejects_partial_malformed_and_repeated_identity() {
    let org = Uuid::from_u128(1);
    let user = Uuid::from_u128(2);
    for query in [
        format!("expected_org_id={org}"),
        format!("expected_user_id={user}"),
        format!("expected_org_id=&expected_user_id={user}"),
        format!("expected_org_id={org}&expected_user_id=invalid"),
        format!("expected_org_id={org}&expected_user_id={user}&expected_user_id={user}"),
        format!("expected_org_id={org}&expected_org_id={org}&expected_user_id={user}"),
    ] {
        assert_eq!(
            authorize(&query, org, user),
            Err(StatusCode::BAD_REQUEST),
            "{query}"
        );
    }
}

#[test]
fn project_download_binding_is_not_authority_to_select_another_requester() {
    let org = Uuid::from_u128(1);
    let user = Uuid::from_u128(2);
    let query = format!("expected_org_id={org}&expected_user_id={user}");
    for (actual_org, actual_user) in [
        (org, Uuid::from_u128(3)),
        (Uuid::from_u128(4), user),
        (Uuid::from_u128(4), Uuid::from_u128(3)),
    ] {
        assert_eq!(
            authorize(&query, actual_org, actual_user),
            Err(StatusCode::FORBIDDEN)
        );
    }
}
