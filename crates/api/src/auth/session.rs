use tower_sessions::Session;
use uuid::Uuid;

pub const SESSION_USER_KEY: &str = "user_id";
/// The signed-in person's organization, written with [`SESSION_USER_KEY`] so a
/// request needs no query to learn it. Sessions signed in before it existed
/// have none.
pub const SESSION_ORG_KEY: &str = "org_id";

/// Returns the authenticated user's UUID from the session, or `None` if not logged in.
pub async fn get_session_user_id(session: &Session) -> Option<Uuid> {
    session.get::<Uuid>(SESSION_USER_KEY).await.ok().flatten()
}

/// The signed-in person's organization as written at sign-in, or `None`.
/// It says nothing about whether the account is still active.
pub async fn get_session_org_id(session: &Session) -> Option<Uuid> {
    session.get::<Uuid>(SESSION_ORG_KEY).await.ok().flatten()
}

/// Write the authenticated person and their organization into the session.
pub async fn set_session_user(
    session: &Session,
    user_id: Uuid,
    org_id: Uuid,
) -> anyhow::Result<()> {
    session.insert(SESSION_USER_KEY, user_id).await?;
    session.insert(SESSION_ORG_KEY, org_id).await?;
    Ok(())
}

/// Remove the user from the session (logout).
pub async fn clear_session(session: &Session) -> anyhow::Result<()> {
    session.flush().await?;
    Ok(())
}
