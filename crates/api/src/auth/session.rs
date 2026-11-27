use tower_sessions::{Session, session, session_store};
use uuid::Uuid;

pub const SESSION_USER_KEY: &str = "user_id";
/// The signed-in person's organization, written with [`SESSION_USER_KEY`] so a
/// request needs no query to learn it. Sessions signed in before it existed
/// have none.
pub const SESSION_ORG_KEY: &str = "org_id";

/// The session store could not be read. It says nothing about whether the
/// request is signed in, so it answers as a server failure (500 or 503),
/// never as signed out: a database that is restarting must not sign anyone
/// out.
#[derive(Debug)]
pub struct SessionUnavailable(session_store::Error);

impl std::fmt::Display for SessionUnavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "The session could not be read: {}", self.0)
    }
}

impl std::error::Error for SessionUnavailable {}

/// The authenticated user's UUID from the session, or `None` if not logged in.
pub async fn get_session_user_id(session: &Session) -> Result<Option<Uuid>, SessionUnavailable> {
    read_uuid(session, SESSION_USER_KEY).await
}

/// The signed-in person's organization as written at sign-in, or `None`.
/// It says nothing about whether the account is still active.
pub async fn get_session_org_id(session: &Session) -> Result<Option<Uuid>, SessionUnavailable> {
    read_uuid(session, SESSION_ORG_KEY).await
}

/// The first read of a request loads the session from the store. A failed
/// load caches nothing, so it is retried once, after asking the pool to check
/// its connections: the usual cause is a connection the database closed while
/// it sat idle, and a load is a read, safe to repeat. A value that does not
/// decode reads as absent, as before.
async fn read_uuid(session: &Session, key: &str) -> Result<Option<Uuid>, SessionUnavailable> {
    match session.get::<Uuid>(key).await {
        Err(session::Error::Store(error)) => {
            tracing::info!(%error, "Retrying a failed session load");
            horae_server::db::check_every_connection();
            match session.get::<Uuid>(key).await {
                Err(session::Error::Store(error)) => Err(SessionUnavailable(error)),
                read => Ok(read.ok().flatten()),
            }
        }
        read => Ok(read.ok().flatten()),
    }
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
