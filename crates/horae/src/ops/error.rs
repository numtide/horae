/// Why an operation did not complete, independent of any wire format. Each
/// adapter maps it: server functions to `ServerFnError` status codes, HTTP
/// adapters to their own responses.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum OpError {
    /// The record is missing, foreign, or hidden from this actor.
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Forbidden(String),
    /// The request conflicts with the current state; reloading may help.
    #[error("{0}")]
    Conflict(String),
    /// The input is invalid; `field` names it when one field is to blame.
    /// `field` is for adapters that pass it on (Harvest, `/api/v1`);
    /// `ServerFnError` carries only the message.
    #[error("{message}")]
    Invalid {
        field: Option<&'static str>,
        message: String,
    },
    /// Storage failed. The cause is logged where it happened, not returned.
    #[error("{0}")]
    Internal(String),
}

impl OpError {
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound(message.into())
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::Forbidden(message.into())
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict(message.into())
    }

    pub fn invalid(field: &'static str, message: impl std::fmt::Display) -> Self {
        Self::Invalid {
            field: Some(field),
            message: message.to_string(),
        }
    }
}

impl From<sqlx::Error> for OpError {
    fn from(error: sqlx::Error) -> Self {
        let code = error
            .as_database_error()
            .and_then(|error| error.code())
            .map(|code| code.into_owned());
        // Serialization failures and deadlocks are a lost race, not a fault:
        // the same request can succeed when retried.
        if matches!(code.as_deref(), Some("40001" | "40P01")) {
            return Self::conflict("The record changed while saving. Reload and try again.");
        }
        tracing::error!(%error, "Operation storage failed");
        Self::Internal("Could not complete the request. Please retry.".into())
    }
}
