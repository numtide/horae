/// DEV_LOGIN handlers — only reachable when DEV_LOGIN=1.
///
/// GET  /auth/login      → HTML page with a one-click "Sign in as Admin" button.
/// POST /auth/dev-login  → sets session to the seeded admin user, redirects to /.
/// POST /auth/logout     → flushes the session, redirects to /auth/login.
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect};
use tower_sessions::Session;

use crate::auth::page::{LoginVariant, render};
use crate::auth::session::{clear_session, set_session_user_id};

/// `GET /auth/login` in dev mode — the one-click "Sign in as Admin" page.
pub async fn login_page() -> impl IntoResponse {
    Html(render(LoginVariant::Dev, None))
}

/// `POST /auth/dev-login` — log in the first active Administrator.
pub async fn dev_login_post(
    session: Session,
) -> Result<impl IntoResponse, (StatusCode, &'static str)> {
    let state = crate::state::global_state().await;

    let row = sqlx::query!(
        r#"SELECT u.id, u.org_id, u.email, u.name
           FROM users u
           JOIN person_permission_states p ON p.org_id = u.org_id AND p.user_id = u.id
           WHERE p.is_administrator AND u.active
           ORDER BY u.id LIMIT 1"#,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "DB query failed"))?;

    let row = row.ok_or((
        StatusCode::INTERNAL_SERVER_ERROR,
        "No admin user found — run `horae seed` first.",
    ))?;

    // Rotate the session id on login to defeat session fixation (matches the
    // OIDC path).
    session.cycle_id().await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to rotate session",
        )
    })?;
    set_session_user_id(&session, row.id)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to write session"))?;

    // Dispatch user_logged_in event (FR-019).
    let mut connection = state
        .db
        .acquire()
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "DB query failed"))?;
    let user =
        crate::server_fns::user_payload(&mut connection, row.org_id, row.id, &row.email, &row.name)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "DB query failed"))?;
    state
        .plugins
        .dispatch(crate::plugin::AppEvent::UserLoggedIn {
            occurred_at: chrono::Utc::now(),
            org_id: row.org_id,
            user: crate::plugin::event::UserPayload {
                method: Some("dev".into()),
                ..user
            },
        });

    Ok(Redirect::to("/"))
}

/// `POST /auth/logout` — flush the session and redirect to login.
pub async fn logout_post(session: Session) -> impl IntoResponse {
    // Capture the user before clearing the session so the logout can be announced.
    if let Some(uid) = crate::auth::session::get_session_user_id(&session).await {
        let state = crate::state::global_state().await;
        let payload = async {
            let mut connection = state.db.acquire().await?;
            let user = sqlx::query!(
                "SELECT id, org_id, email, name FROM users WHERE id = $1",
                uid,
            )
            .fetch_one(&mut *connection)
            .await?;
            let payload = crate::server_fns::user_payload(
                &mut connection,
                user.org_id,
                user.id,
                &user.email,
                &user.name,
            )
            .await?;
            Ok::<_, sqlx::Error>((user.org_id, payload))
        };
        if let Ok((org_id, user)) = payload.await {
            state
                .plugins
                .dispatch(crate::plugin::AppEvent::UserLoggedOut {
                    occurred_at: chrono::Utc::now(),
                    org_id,
                    user,
                });
        }
    }
    clear_session(&session).await.ok();
    Redirect::to("/auth/login")
}
