use axum::extract::{FromRef, FromRequestParts};
use axum::http::{StatusCode, request::Parts};
use horae_core::types::OrgRole;
use uuid::Uuid;

/// Extractor that authenticates a Harvest API request.
///
/// For MVP this reads the user from the tower-sessions cookie.
/// A future iteration will add `Authorization: Bearer <token>` support
/// with an `api_tokens` table.
///
/// It carries the caller's role as well as their identity: this API serves the
/// same session cookie as the SPA, so it has to apply the same rule that rates
/// and other people's rows are manager material. The handlers do that gating;
/// see [`super::forbidden`] and [`super::scoped_user_filter`].
pub struct AuthUser {
    pub user_id: Uuid,
    pub org_id: Uuid,
    pub org_role: OrgRole,
}

impl<S: Send + Sync> FromRequestParts<S> for AuthUser
where
    sqlx::PgPool: FromRef<S>,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Extract the session from request extensions (set by SessionManagerLayer).
        let session = parts
            .extensions
            .get::<tower_sessions::Session>()
            .cloned()
            .ok_or((StatusCode::UNAUTHORIZED, "No session"))?;

        let user_id = crate::auth::session::get_session_user_id(&session)
            .await
            .ok_or((StatusCode::UNAUTHORIZED, "Not authenticated"))?;

        // Look up the user's org and role. The `active` filter is what revokes a
        // deactivated user's still-live session (FR-002).
        let db = sqlx::PgPool::from_ref(state);
        let row = sqlx::query!(
            r#"SELECT org_id, org_role as "org_role: OrgRole"
               FROM users WHERE id = $1 AND active = true"#,
            user_id
        )
        .fetch_optional(&db)
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, "User not found"))?
        .ok_or((StatusCode::UNAUTHORIZED, "User not found"))?;

        Ok(AuthUser {
            user_id,
            org_id: row.org_id,
            org_role: row.org_role,
        })
    }
}
