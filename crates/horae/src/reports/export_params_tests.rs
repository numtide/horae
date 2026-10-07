use super::*;
use crate::models::{permission_editor::PermissionRequester, time_report::TimeReportQuery};
use uuid::Uuid;

fn parse(filters: &str) -> Result<TimeReportQuery, StatusCode> {
    let uri = format!("/api/reports/export/csv?from=2026-09-01&to=2026-09-30{filters}")
        .parse()
        .unwrap();
    Query::<ExportParams>::try_from_uri(&uri)
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .0
        .time_query()
}

#[test]
fn download_policy_binding_rejects_unknown_and_repeated_modes() {
    for value in ["scoped", "legacy"] {
        let uri = format!(
            "/api/reports/export/csv?from=2026-09-01&to=2026-09-30&expected_policy={value}"
        )
        .parse()
        .unwrap();
        let params = Query::<ExportParams>::try_from_uri(&uri).unwrap().0;
        assert_eq!(serde_json::to_value(params.expected_policy).unwrap(), value);
    }
    for query in [
        "expected_policy=",
        "expected_policy=1",
        "expected_policy=future",
        "expected_policy=scoped&expected_policy=legacy",
    ] {
        let uri = format!("/api/reports/export/csv?from=2026-09-01&to=2026-09-30&{query}")
            .parse()
            .unwrap();
        assert!(
            Query::<ExportParams>::try_from_uri(&uri).is_err(),
            "{query}"
        );
    }
}

#[test]
fn plural_filters_preserve_every_dimension_and_deduplicate_ids() {
    let a = Uuid::from_u128(1);
    let b = Uuid::from_u128(2);
    let query = parse(&format!(
        "&client_ids={a},{b},{a}&project_ids={b}&user_ids={a}&task_ids={b},{a}&tag_ids={a}%2C{b}"
    ))
    .unwrap();
    assert_eq!(query.client_ids, vec![a, b]);
    assert_eq!(query.project_ids, vec![b]);
    assert_eq!(query.user_ids, vec![a]);
    assert_eq!(query.task_ids, vec![a, b]);
    assert_eq!(query.tag_ids, vec![a, b]);
}

#[test]
fn legacy_scalar_and_empty_plural_filters_preserve_the_existing_scope() {
    let id = Uuid::from_u128(1);
    let query = parse(&format!(
        "&client_id={id}&project_id={id}&user_id={id}&tag_id={id}&task_ids="
    ))
    .unwrap();
    assert_eq!(query.client_ids, vec![id]);
    assert_eq!(query.project_ids, vec![id]);
    assert_eq!(query.user_ids, vec![id]);
    assert_eq!(query.tag_ids, vec![id]);
    assert!(query.task_ids.is_empty());
    let empty = parse("&client_ids=&project_ids=&user_ids=&task_ids=&tag_ids=").unwrap();
    assert!(
        empty.client_ids.is_empty()
            && empty.project_ids.is_empty()
            && empty.user_ids.is_empty()
            && empty.task_ids.is_empty()
            && empty.tag_ids.is_empty()
    );
}

#[test]
fn malformed_or_ambiguous_filters_never_become_unrestricted_downloads() {
    let id = Uuid::from_u128(1);
    for field in ["client", "project", "user", "tag"] {
        for plural in [String::new(), id.to_string()] {
            assert_eq!(
                parse(&format!("&{field}_id={id}&{field}_ids={plural}")),
                Err(StatusCode::BAD_REQUEST)
            );
        }
    }
    for field in [
        "client_ids",
        "project_ids",
        "user_ids",
        "task_ids",
        "tag_ids",
    ] {
        for value in [
            "invalid".to_owned(),
            format!("{id},"),
            format!(",{id}"),
            format!("{id},invalid"),
            format!("{id},,{id}"),
            "%20".to_owned(),
        ] {
            assert_eq!(
                parse(&format!("&{field}={value}")),
                Err(StatusCode::BAD_REQUEST),
                "{field}={value}"
            );
        }
        assert_eq!(
            parse(&format!("&{field}={id}&{field}={id}")),
            Err(StatusCode::BAD_REQUEST)
        );
    }
}

#[test]
fn download_identity_is_a_complete_optional_binding_not_authority() {
    let org = Uuid::from_u128(1);
    let user = Uuid::from_u128(2);
    assert_eq!(
        parse(&format!("&expected_org_id={org}&expected_user_id={user}"))
            .unwrap()
            .expected_requester,
        Some(PermissionRequester {
            org_id: org,
            user_id: user
        })
    );
    assert_eq!(
        parse(&format!("&expected_org_id={org}")),
        Err(StatusCode::BAD_REQUEST)
    );
    assert_eq!(
        parse(&format!("&expected_user_id={user}")),
        Err(StatusCode::BAD_REQUEST)
    );
    assert_eq!(
        parse("&expected_org_id=invalid&expected_user_id=invalid"),
        Err(StatusCode::BAD_REQUEST)
    );
    assert_eq!(
        parse("&expected_org_id=&expected_user_id="),
        Err(StatusCode::BAD_REQUEST)
    );
    assert_eq!(
        parse(&format!(
            "&expected_org_id={org}&expected_user_id={user}&expected_user_id={user}"
        )),
        Err(StatusCode::BAD_REQUEST)
    );
    assert_eq!(
        parse(&format!("&org_id={org}&actor_id={user}"))
            .unwrap()
            .expected_requester,
        None
    );
}

#[test]
fn downloads_reject_cursors_and_reversed_periods() {
    assert_eq!(parse("&after="), Err(StatusCode::BAD_REQUEST));
    assert_eq!(parse("&after"), Err(StatusCode::BAD_REQUEST));
    assert_eq!(parse("&after=invalid"), Err(StatusCode::BAD_REQUEST));
    let uri = "/api/reports/export/xlsx?from=2026-09-30&to=2026-09-01"
        .parse()
        .unwrap();
    assert_eq!(
        Query::<ExportParams>::try_from_uri(&uri)
            .unwrap()
            .0
            .time_query(),
        Err(StatusCode::BAD_REQUEST)
    );
}
