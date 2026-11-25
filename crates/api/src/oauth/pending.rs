//! Authorization requests waiting in the person's session for Allow or Deny
//! (research R-3, data-model *Session data*): at most five, each for ten
//! minutes (FR-027), under a random id the approval page's address carries.

use chrono::{Duration, Utc};
use tower_sessions::Session;

use crate::ops::oauth::authorize::PendingRequest;

pub const KEY: &str = "oauth_requests";
const MAX: usize = 5;
const TTL: Duration = Duration::minutes(10);

type Requests = Vec<(String, PendingRequest)>;

/// Keeps `request` and answers its id. The oldest request goes when there
/// are already five.
pub async fn store(
    session: &Session,
    request: PendingRequest,
) -> Result<String, tower_sessions::session::Error> {
    let mut requests = live(session).await?;
    if requests.len() >= MAX {
        requests.remove(0);
    }
    let id = crate::ops::oauth::request_id();
    requests.push((id.clone(), request));
    session.insert(KEY, requests).await?;
    Ok(id)
}

/// The request `id` of this session, unless it expired.
pub async fn get(
    session: &Session,
    id: &str,
) -> Result<Option<PendingRequest>, tower_sessions::session::Error> {
    Ok(live(session)
        .await?
        .into_iter()
        .find(|(key, _)| key == id)
        .map(|(_, request)| request))
}

/// [`get`], removing the request so a second Allow or Deny finds nothing.
pub async fn take(
    session: &Session,
    id: &str,
) -> Result<Option<PendingRequest>, tower_sessions::session::Error> {
    let mut requests = live(session).await?;
    let Some(index) = requests.iter().position(|(key, _)| key == id) else {
        return Ok(None);
    };
    let (_, request) = requests.remove(index);
    session.insert(KEY, requests).await?;
    Ok(Some(request))
}

async fn live(session: &Session) -> Result<Requests, tower_sessions::session::Error> {
    let requests: Requests = session.get(KEY).await?.unwrap_or_default();
    let cutoff = Utc::now() - TTL;
    Ok(requests
        .into_iter()
        .filter(|(_, request)| request.created_at > cutoff)
        .collect())
}
