use super::*;
use axum::body::Body;
use serde_json::json;

fn problem(error: OpError) -> (u16, String, Option<String>, Option<serde_json::Value>) {
    let problem = Problem::from(error);
    (
        problem.status,
        problem.code,
        problem.detail,
        problem.field.map(|field| *field),
    )
}

#[test]
fn each_operation_error_has_one_status_and_code() {
    assert_eq!(
        problem(OpError::not_found("Project not found")),
        (
            404,
            "not_found".into(),
            Some("Project not found".into()),
            None
        )
    );
    assert_eq!(
        problem(OpError::forbidden("No")),
        (403, "forbidden".into(), Some("No".into()), None)
    );
    assert_eq!(
        problem(OpError::conflict("Locked")),
        (409, "conflict".into(), Some("Locked".into()), None)
    );
    assert_eq!(
        problem(OpError::bad_input("Bad")),
        (422, "invalid".into(), Some("Bad".into()), None)
    );
}

#[test]
fn an_invalid_field_and_a_reason_are_passed_on() {
    assert_eq!(
        problem(OpError::invalid("minutes", "Too long")),
        (
            422,
            "invalid".into(),
            Some("Too long".into()),
            Some(json!("minutes"))
        )
    );
    assert_eq!(
        problem(
            OpError::conflict("Submitted")
                .with_reason("entry_locked")
                .with_field(json!({ "entry": "e1" }))
        ),
        (
            409,
            "entry_locked".into(),
            Some("Submitted".into()),
            Some(json!({ "entry": "e1" }))
        )
    );
    assert_eq!(
        problem(OpError::invalid("notes", "Bad").with_reason("too_long")),
        (
            422,
            "too_long".into(),
            Some("Bad".into()),
            Some(json!("notes"))
        )
    );
}

#[test]
fn a_reason_does_not_rename_a_missing_or_forbidden_record() {
    assert_eq!(
        problem(OpError::not_found("Gone").with_reason("gone")).1,
        "not_found"
    );
    assert_eq!(
        problem(OpError::forbidden("No").with_reason("scope")).1,
        "forbidden"
    );
}

#[test]
fn an_internal_error_withholds_its_detail() {
    let internal = Problem::from(OpError::Internal("database down at 10.0.0.1".into()));
    assert_eq!((internal.status, internal.code.as_str()), (500, "internal"));
    assert_eq!(internal.detail, None);
    let text = serde_json::to_string(&internal).unwrap();
    assert!(!text.contains("10.0.0.1"), "{text}");
}

#[tokio::test]
async fn a_problem_answers_as_problem_json() {
    let response = Problem::from(OpError::conflict("Locked")).into_response();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(response.headers()[header::CONTENT_TYPE], CONTENT_TYPE);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
        json!({
            "type": "about:blank",
            "title": "Conflict",
            "status": 409,
            "detail": "Locked",
            "code": "conflict",
            "field": null,
        })
    );
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    minutes: i32,
}

async fn decode(content_type: Option<&str>, body: &str) -> Result<i32, Problem> {
    let mut request = Request::builder().method("POST").uri("/");
    if let Some(content_type) = content_type {
        request = request.header(header::CONTENT_TYPE, content_type);
    }
    let request = request.body(Body::from(body.to_owned())).unwrap();
    ApiJson::<Input>::from_request(request, &())
        .await
        .map(|ApiJson(input)| input.minutes)
}

#[tokio::test]
async fn a_json_body_decodes() {
    for content_type in ["application/json", "application/json; charset=utf-8"] {
        assert_eq!(
            decode(Some(content_type), r#"{"minutes": 90}"#)
                .await
                .unwrap(),
            90
        );
    }
}

#[tokio::test]
async fn an_unknown_body_field_is_unprocessable() {
    let problem = decode(Some("application/json"), r#"{"minutes": 90, "hours": 1.5}"#)
        .await
        .unwrap_err();
    assert_eq!(
        (problem.status, problem.code.as_str()),
        (422, "unknown_field")
    );
    assert_eq!(problem.field.as_deref(), Some(&json!("hours")));
}

#[tokio::test]
async fn a_body_that_does_not_decode_is_a_bad_request() {
    for (content_type, body) in [
        (Some("application/json"), "{"),
        (Some("application/json"), r#"{"minutes": "ninety"}"#),
        (Some("application/json"), "{}"),
        (Some("text/plain"), r#"{"minutes": 90}"#),
        (None, r#"{"minutes": 90}"#),
    ] {
        let problem = decode(content_type, body).await.unwrap_err();
        assert_eq!(
            (problem.status, problem.code.as_str()),
            (400, "bad_request"),
            "{content_type:?} {body}"
        );
    }
}

#[derive(Debug, Deserialize)]
struct Filters {
    person_id: Option<uuid::Uuid>,
}

#[tokio::test]
async fn a_query_parameter_of_the_wrong_type_is_a_bad_request() {
    let (mut parts, ()) = Request::builder()
        .uri("/?person_id=nobody")
        .body(())
        .unwrap()
        .into_parts();
    let problem = ApiQuery::<Filters>::from_request_parts(&mut parts, &())
        .await
        .err()
        .unwrap();
    assert_eq!(
        (problem.status, problem.code.as_str()),
        (400, "bad_request")
    );

    let (mut parts, ()) = Request::builder()
        .uri("/?unknown=1")
        .body(())
        .unwrap()
        .into_parts();
    let ApiQuery(filters) = ApiQuery::<Filters>::from_request_parts(&mut parts, &())
        .await
        .ok()
        .unwrap();
    assert!(
        filters.person_id.is_none(),
        "unknown parameters are ignored"
    );
}
