use dioxus::prelude::*;
use uuid::Uuid;

use crate::models::project_creation::PROJECT_EDITOR_SESSION_CHANGED;

const UNAUTHORIZED: u16 = 401;
const FORBIDDEN: u16 = 403;
const CONFLICT: u16 = 409;

#[derive(Clone, Copy)]
pub(super) struct Boundary {
    pub generation: Uuid,
    pub invalidated: Signal<bool>,
    pub on_unavailable: EventHandler<Uuid>,
}

pub(super) fn changed() -> ServerFnError {
    ServerFnError::ServerError {
        code: CONFLICT,
        message: "Your session changed. Reload the project before continuing.".into(),
        details: Some(serde_json::json!({ "reason": PROJECT_EDITOR_SESSION_CHANGED })),
    }
}

pub(super) fn invalidates(error: &ServerFnError) -> bool {
    match error {
        ServerFnError::ServerError {
            code: UNAUTHORIZED | FORBIDDEN,
            ..
        } => true,
        ServerFnError::ServerError {
            code: CONFLICT,
            details: Some(details),
            ..
        } => {
            details.get("reason").and_then(serde_json::Value::as_str)
                == Some(PROJECT_EDITOR_SESSION_CHANGED)
        }
        _ => false,
    }
}

pub(super) fn report(error: ServerFnError) -> ServerFnError {
    if invalidates(&error)
        && let Some(mut boundary) = try_consume_context::<Boundary>()
        && !*boundary.invalidated.peek()
    {
        boundary.invalidated.set(true);
        boundary.on_unavailable.call(boundary.generation);
    }
    error
}

pub(super) fn finish<T>(result: Result<T, ServerFnError>) -> Result<T, ServerFnError> {
    if try_consume_context::<Boundary>().is_some_and(|boundary| *boundary.invalidated.peek()) {
        return Err(changed());
    }
    result.map_err(report)
}
