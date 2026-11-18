//! How a server function answers an operation's error: its status code,
//! message and details.

use super::*;
use crate::ops::OpError;
use serde_json::json;

fn answer(error: OpError) -> (u16, String, Option<serde_json::Value>) {
    match ServerFnError::from(error) {
        ServerFnError::ServerError {
            code,
            message,
            details,
        } => (code, message, details),
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn an_error_without_detail_answers_without_details() {
    assert_eq!(
        answer(OpError::conflict("Changed")),
        (409, "Changed".into(), None)
    );
}

#[test]
fn a_field_and_a_reason_reach_details_with_the_same_status_and_message() {
    let error = OpError::bad_input("Bad")
        .with_field(json!({ "person": "row" }))
        .with_reason("stale_page");
    assert_eq!(
        answer(error),
        (
            400,
            "Bad".into(),
            Some(json!({ "field": { "person": "row" }, "reason": "stale_page" }))
        )
    );
    assert_eq!(
        answer(OpError::conflict("Reload").with_reason("stale_page")),
        (
            409,
            "Reload".into(),
            Some(json!({ "reason": "stale_page" }))
        )
    );
}

#[test]
fn a_not_found_error_answers_its_field() {
    assert_eq!(
        answer(OpError::not_found("Gone").with_field("client")),
        (404, "Gone".into(), Some(json!({ "field": "client" })))
    );
}
