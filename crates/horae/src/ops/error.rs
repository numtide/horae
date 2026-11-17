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
    /// Another error with what a client acts on besides its status and
    /// message. Built with [`OpError::with_field`] and [`OpError::with_reason`];
    /// match on [`OpError::kind`] to see the error underneath.
    #[error("{error}")]
    Detailed {
        error: Box<OpError>,
        detail: ErrorDetail,
    },
}

/// The machine-readable part of an error, in the spirit of problem+json's
/// per-field errors: the input to blame and a stable reason. Adapters pass it
/// on only when it is present, so errors without one answer as before.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ErrorDetail {
    /// The input to blame, as the client names it: a plain name or a
    /// structured identity such as a form row.
    pub field: Option<serde_json::Value>,
    /// A stable code the client branches on, independent of the message.
    pub reason: Option<&'static str>,
}

impl OpError {
    /// Names the input to blame. Only invalid input, a missing record or a
    /// conflict can blame one; any other error is returned unchanged.
    pub fn with_field(self, field: impl serde::Serialize) -> Self {
        if !matches!(
            self.kind(),
            Self::Invalid { .. } | Self::NotFound(_) | Self::Conflict(_)
        ) {
            return self;
        }
        let field = serde_json::to_value(field).ok();
        self.detailed(|detail| detail.field = field)
    }

    /// Adds a stable reason the client branches on.
    pub fn with_reason(self, reason: &'static str) -> Self {
        self.detailed(|detail| detail.reason = Some(reason))
    }

    fn detailed(self, change: impl FnOnce(&mut ErrorDetail)) -> Self {
        let (error, mut detail) = match self {
            Self::Detailed { error, detail } => (error, detail),
            error => (Box::new(error), ErrorDetail::default()),
        };
        change(&mut detail);
        Self::Detailed { error, detail }
    }

    /// The error without its detail, to match on what went wrong.
    pub fn kind(&self) -> &Self {
        match self {
            Self::Detailed { error, .. } => error.kind(),
            error => error,
        }
    }

    pub fn detail(&self) -> Option<&ErrorDetail> {
        match self {
            Self::Detailed { detail, .. } => Some(detail),
            _ => None,
        }
    }

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

    /// Invalid input that no single field is to blame for.
    pub fn bad_input(message: impl std::fmt::Display) -> Self {
        Self::Invalid {
            field: None,
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_detail_keeps_the_kind_of_error() {
        let error = OpError::bad_input("Bad")
            .with_field(json!({ "person": "row" }))
            .with_reason("stale_page");
        assert_eq!(error.kind(), &OpError::bad_input("Bad"));
    }

    #[test]
    fn only_input_errors_blame_a_field() {
        for error in [OpError::forbidden("No"), OpError::Internal("Down".into())] {
            assert_eq!(error.clone().with_field("name"), error);
        }
    }
}
