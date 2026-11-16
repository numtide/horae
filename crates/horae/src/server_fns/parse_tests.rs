//! A malformed argument is the caller's mistake, so each conversion answers
//! 400 with a message naming the field.

use super::*;

fn bad_request<T: std::fmt::Debug>(result: Result<T, ServerFnError>) -> (u16, String) {
    match result {
        Err(ServerFnError::ServerError { code, message, .. }) => (code, message),
        other => panic!("expected a server error, got {other:?}"),
    }
}

#[test]
fn a_malformed_uuid_is_a_bad_request() {
    assert_eq!(
        bad_request(parse_uuid("not-a-uuid", "task_id")),
        (BAD_REQUEST, "Invalid task_id".to_owned())
    );
}

#[test]
fn a_malformed_optional_uuid_is_a_bad_request() {
    assert_eq!(
        bad_request(parse_opt_uuid(Some("nope".into()), "client_id")),
        (BAD_REQUEST, "Invalid client_id".to_owned())
    );
}

#[test]
fn a_malformed_date_is_a_bad_request() {
    assert_eq!(
        bad_request(parse_date("2026-13-01", "period_from")),
        (
            BAD_REQUEST,
            "Invalid period_from (use YYYY-MM-DD)".to_owned()
        )
    );
}

#[test]
fn a_malformed_enum_is_a_bad_request() {
    assert_eq!(
        bad_request(parse_enum::<InvoiceStatus>("lost", "status")),
        (BAD_REQUEST, "Invalid status".to_owned())
    );
}
