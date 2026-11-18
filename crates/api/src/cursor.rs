//! List cursors (research R-5). Every list is ordered by `id`, a UUID v7, and
//! a page continues after the last id of the previous one. The cursor names
//! that id, the list and a digest of its filters, and when it was issued, all
//! signed with HMAC-SHA256, so a client can neither forge a position nor reuse
//! a cursor on another list, with other filters or after 24 hours.
//!
//! Layout before base64url: `version ‖ list length ‖ list ‖ SHA-256(filters)
//! ‖ last id ‖ issued at (Unix seconds, big endian) ‖ HMAC`.

use axum::http::StatusCode;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{DateTime, TimeDelta, Utc};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::config::CursorKey;
use crate::problem::Problem;

const VERSION: u8 = 1;
const MAC_LEN: usize = 32;
/// How long a cursor stays valid.
pub const MAX_AGE: TimeDelta = TimeDelta::hours(24);
/// How far ahead of this server's clock a cursor may claim to be issued.
const CLOCK_SKEW: TimeDelta = TimeDelta::minutes(1);

pub const DEFAULT_LIMIT: u32 = 100;
pub const MAX_LIMIT: u32 = 1000;

type HmacSha256 = Hmac<Sha256>;

/// One list as a client walks it: its name and its normalized filters. Two
/// requests with the same filters must give the same `filters` string.
pub struct Walk<'a> {
    pub key: &'a CursorKey,
    pub list: &'a str,
    pub filters: &'a str,
}

/// Where a page starts and how long it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub limit: u32,
    /// The last id of the previous page; `None` for the first page.
    pub after: Option<Uuid>,
}

impl Walk<'_> {
    pub fn encode(&self, last_id: Uuid, issued_at: DateTime<Utc>) -> String {
        let mut bytes = self.unsigned(last_id, issued_at);
        let mac = self.mac(&bytes).finalize().into_bytes();
        bytes.extend_from_slice(&mac);
        URL_SAFE_NO_PAD.encode(bytes)
    }

    /// The id a cursor continues after, or `None` when it is not one this
    /// walk issued in the last 24 hours.
    pub fn decode(&self, cursor: &str, now: DateTime<Utc>) -> Option<Uuid> {
        let bytes = URL_SAFE_NO_PAD.decode(cursor).ok()?;
        let (body, mac) = bytes.split_at_checked(bytes.len().checked_sub(MAC_LEN)?)?;
        self.mac(body).verify_slice(mac).ok()?;
        // Signed by this key, so only an older version or another list can differ.
        let (last_id, issued_at) = self.parse(body)?;
        let age = now - issued_at;
        (age <= MAX_AGE && age >= -CLOCK_SKEW).then_some(last_id)
    }

    /// The page a request asks for: `limit` from 1 to 1000 (100 when
    /// absent), after the cursor's position.
    pub fn position(
        &self,
        limit: Option<u32>,
        cursor: Option<&str>,
        now: DateTime<Utc>,
    ) -> Result<Position, Problem> {
        let limit = limit.unwrap_or(DEFAULT_LIMIT);
        if !(1..=MAX_LIMIT).contains(&limit) {
            return Err(Problem::invalid(
                "limit",
                format!("Choose a limit from 1 to {MAX_LIMIT}"),
            ));
        }
        let after = match cursor {
            None => None,
            Some(cursor) => Some(self.decode(cursor, now).ok_or_else(invalid_cursor)?),
        };
        Ok(Position { limit, after })
    }

    /// Cuts `rows`, read with one row more than `limit`, to the page, and
    /// gives the cursor of the next page when that extra row exists.
    pub fn next_cursor<T>(
        &self,
        rows: &mut Vec<T>,
        position: Position,
        id: impl Fn(&T) -> Uuid,
        now: DateTime<Utc>,
    ) -> Option<String> {
        let limit = usize::try_from(position.limit).unwrap_or(usize::MAX);
        if rows.len() <= limit {
            return None;
        }
        rows.truncate(limit);
        rows.last().map(|row| self.encode(id(row), now))
    }

    fn unsigned(&self, last_id: Uuid, issued_at: DateTime<Utc>) -> Vec<u8> {
        let list = self.list.as_bytes();
        let length = u8::try_from(list.len()).expect("a list name fits in 255 bytes");
        let mut bytes = vec![VERSION, length];
        bytes.extend_from_slice(list);
        bytes.extend_from_slice(&Sha256::digest(self.filters.as_bytes()));
        bytes.extend_from_slice(last_id.as_bytes());
        bytes.extend_from_slice(&issued_at.timestamp().to_be_bytes());
        bytes
    }

    fn parse(&self, body: &[u8]) -> Option<(Uuid, DateTime<Utc>)> {
        let (&version, rest) = body.split_first()?;
        let (&length, rest) = rest.split_first()?;
        let (list, rest) = rest.split_at_checked(usize::from(length))?;
        let (digest, rest) = rest.split_at_checked(32)?;
        let (last_id, rest) = rest.split_at_checked(16)?;
        let issued_at: [u8; 8] = rest.try_into().ok()?;
        let same_walk = version == VERSION
            && list == self.list.as_bytes()
            && digest == Sha256::digest(self.filters.as_bytes()).as_slice();
        if !same_walk {
            return None;
        }
        Some((
            Uuid::from_slice(last_id).ok()?,
            DateTime::from_timestamp(i64::from_be_bytes(issued_at), 0)?,
        ))
    }

    fn mac(&self, bytes: &[u8]) -> HmacSha256 {
        let mut mac =
            HmacSha256::new_from_slice(self.key.bytes()).expect("HMAC accepts any key length");
        mac.update(bytes);
        mac
    }
}

/// A cursor that is malformed, forged, from another list or filters, or
/// expired: the client starts the walk again.
pub fn invalid_cursor() -> Problem {
    Problem::new(
        StatusCode::BAD_REQUEST,
        "invalid_cursor",
        Some("The cursor is invalid or has expired; start the list again".into()),
    )
    .with_field("cursor")
}

#[cfg(test)]
mod tests;
