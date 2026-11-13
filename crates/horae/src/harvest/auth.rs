use axum::extract::{FromRef, FromRequestParts};
use axum::http::request::Parts;
use uuid::Uuid;

use super::errors::ApiError;

/// Extractor that authenticates a Harvest API request.
///
/// For MVP this reads the user from the tower-sessions cookie.
/// A future iteration will add `Authorization: Bearer <token>` support
/// with an `api_tokens` table.
///
/// It carries only the caller's identity. Each reader loads the caller's
/// stored permission state inside its own transaction (see
/// [`super::begin_read`]), so the grants it decides with cannot go stale
/// between this extractor and the rows it returns.
pub struct AuthUser {
    pub user_id: Uuid,
    pub org_id: Uuid,
}

impl<S: Send + Sync> FromRequestParts<S> for AuthUser
where
    sqlx::PgPool: FromRef<S>,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Extract the session from request extensions (set by SessionManagerLayer).
        let session = parts
            .extensions
            .get::<tower_sessions::Session>()
            .cloned()
            .ok_or_else(|| ApiError::unauthorized("No session"))?;

        let user_id = crate::auth::session::get_session_user_id(&session)
            .await
            .ok_or_else(|| ApiError::unauthorized("Not authenticated"))?;

        // Look up the user's org. The `active` filter is what revokes a
        // deactivated user's still-live session (FR-002).
        let db = sqlx::PgPool::from_ref(state);
        let org_id = sqlx::query_scalar!(
            "SELECT org_id FROM users WHERE id = $1 AND active = true",
            user_id
        )
        .fetch_optional(&db)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::unauthorized("User not found"))?;

        Ok(AuthUser { user_id, org_id })
    }
}
