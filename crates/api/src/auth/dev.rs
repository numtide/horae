/// DEV_LOGIN handlers — only reachable when DEV_LOGIN=1.
///
/// GET  /auth/login      → HTML page with a one-click "Sign in as Admin" button.
/// POST /auth/dev-login  → sets session to the seeded admin user, redirects to /.
/// POST /auth/logout     → flushes the session, redirects to /auth/login.
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect};
use tower_sessions::Session;

use super::page::{LoginAssets, LoginVariant, render};
use super::session::{clear_session, set_session_user};

/// `GET /auth/login` in dev mode — the one-click "Sign in as Admin" page.
pub async fn login_page(State(assets): State<LoginAssets>) -> impl IntoResponse {
    Html(render(LoginVariant::Dev, None, &assets))
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
    set_session_user(&session, row.id, row.org_id)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to write session"))?;

    // Dispatch user_logged_in event (FR-019).
    let mut connection = state
        .db
        .acquire()
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "DB query failed"))?;
    let user = crate::plugin::event::user_payload(
        &mut connection,
        row.org_id,
        row.id,
        &row.email,
        &row.name,
    )
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

    Ok(Redirect::to(&super::take_return_to(&session).await))
}

/// `POST /auth/logout` — flush the session and redirect to login.
///
/// A form field `return_to` naming an `/oauth/` page ("Not you? Sign out" on
/// the approval page) survives the flush with the pending authorization
/// requests, so the next sign-in comes back to the same request.
pub async fn logout_post(session: Session, body: axum::body::Bytes) -> impl IntoResponse {
    let return_to = url::form_urlencoded::parse(&body)
        .find(|(key, _)| key == "return_to")
        .and_then(|(_, value)| super::safe_return_to(&value).map(str::to_owned));
    let requests: Option<serde_json::Value> = match return_to {
        Some(_) => session.get(crate::oauth::pending::KEY).await.ok().flatten(),
        None => None,
    };
    end_session(&session).await;
    if let Some(return_to) = return_to {
        let kept = async {
            session.insert(super::RETURN_TO_KEY, return_to).await?;
            if let Some(requests) = requests {
                session.insert(crate::oauth::pending::KEY, requests).await?;
            }
            Ok::<_, tower_sessions::session::Error>(())
        };
        if let Err(error) = kept.await {
            tracing::warn!(%error, "Could not keep the authorization request across sign-out");
        }
    }
    Redirect::to("/auth/login")
}

/// Announces the sign-out and empties the session.
async fn end_session(session: &Session) {
    // Capture the user before clearing the session so the logout can be announced.
    if let Ok(Some(uid)) = super::session::get_session_user_id(session).await {
        let state = crate::state::global_state().await;
        let payload = async {
            let mut connection = state.db.acquire().await?;
            let user = sqlx::query!(
                "SELECT id, org_id, email, name FROM users WHERE id = $1",
                uid,
            )
            .fetch_one(&mut *connection)
            .await?;
            let payload = crate::plugin::event::user_payload(
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
    clear_session(session).await.ok();
}
