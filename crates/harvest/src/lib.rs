//! Harvest's API v2: the resource types Horae reads, the paging HTTP client and
//! the OAuth token calls. No database and no Horae state, so every part that
//! speaks Harvest shares one contract, tested without PostgreSQL.

pub mod http;
pub mod oauth;
pub mod v2;

/// A Harvest failure callers act on differently from a plain error. The client
/// raises it inside `anyhow::Error`; callers find it with `downcast`.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// HTTP 401 from the data API, or a refused token refresh: the grant is gone.
    #[error("Harvest no longer accepts this connection")]
    Unauthorized,
    /// HTTP 403 from the data API.
    #[error("Harvest refused access (HTTP 403)")]
    Forbidden,
    /// Harvest timed out or answered with a server error; worth retrying.
    #[error("{0}")]
    Unavailable(String),
}
