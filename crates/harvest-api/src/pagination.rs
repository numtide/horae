//! Harvest's list paging (spec 021 FR-040, FR-041, research R-8). Every list
//! takes `page` and `per_page` (1–2000, default 2000). Catalogue lists also
//! take `cursor`: an opaque, signed position, the `(updated_at, id)` of the
//! row a page continues after (or, going back, ends before). They are ordered
//! by that pair, so a page walk and a cursor walk visit the same rows in the
//! same order, and the envelope keeps `page` and the totals filled either way.
//! Links are absolute on the host the request arrived on.

use std::convert::Infallible;
use std::sync::Arc;

use axum::extract::{FromRequestParts, OriginalUri};
use axum::http::{header, request::Parts};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use horae_harvest::v2::compat::HarvestPagination;
use horae_server::config::CursorKey;
use sha2::Sha256;
use uuid::Uuid;

use crate::HarvestState;
use crate::errors::ApiError;

pub(crate) const MAX_PER_PAGE: i64 = 2000;

const VERSION: u8 = 1;
const MAC_LEN: usize = 32;

/// Where a catalogue row sits in its list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Key {
    pub at: DateTime<Utc>,
    pub id: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Cursor {
    key: Key,
    /// The page ends before `key` rather than starting after it.
    before: bool,
}

/// The rows a request asks for.
pub(crate) struct Window {
    pub page: i64,
    pub per_page: i64,
    /// Rows to skip when there is no cursor.
    pub offset: i64,
    cursor: Option<Cursor>,
}

impl Window {
    /// The window of a read by id.
    pub fn one() -> Self {
        Self {
            page: 1,
            per_page: 1,
            offset: 0,
            cursor: None,
        }
    }

    /// The cursor's position as query parameters: its update time and id
    /// (both `None` without a cursor), and whether the page ends before it.
    pub fn position(&self) -> (Option<DateTime<Utc>>, Option<Uuid>, bool) {
        match self.cursor {
            Some(c) => (Some(c.key.at), Some(c.key.id), c.before),
            None => (None, None, false),
        }
    }
}

/// A list's rows with its total and the index of its first row in the list.
pub(crate) struct Listed<T> {
    pub total: i64,
    pub start: i64,
    pub rows: Vec<T>,
}

/// What a list handler needs from its request to page and link.
pub(crate) struct ListRequest {
    /// `scheme://host/path`, without the query.
    url: String,
    query: Option<String>,
    key: Arc<CursorKey>,
}

impl FromRequestParts<HarvestState> for ListRequest {
    type Rejection = Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &HarvestState,
    ) -> Result<Self, Self::Rejection> {
        let uri = parts
            .extensions
            .get::<OriginalUri>()
            .map_or(&parts.uri, |original| &original.0);
        let header = |name| {
            parts
                .headers
                .get(name)
                .and_then(|value| value.to_str().ok())
        };
        let host = header(header::HOST.as_str())
            .or_else(|| uri.authority().map(|authority| authority.as_str()))
            .unwrap_or("localhost");
        // A proxy may list several protocols; the first is the client's.
        let scheme = match header("x-forwarded-proto").and_then(|v| v.split(',').next()) {
            Some(proto) if proto.trim().eq_ignore_ascii_case("https") => "https",
            _ => "http",
        };
        Ok(Self {
            url: format!("{scheme}://{host}{}", uri.path()),
            query: uri.query().map(str::to_owned),
            key: state.0.cursor_key.clone(),
        })
    }
}

impl ListRequest {
    /// A request for a list with no query, for tests that call handlers'
    /// helpers directly.
    #[cfg(test)]
    pub fn test() -> Self {
        Self {
            url: "http://localhost/harvest/v2/list".into(),
            query: None,
            key: Arc::new(CursorKey::new([7; 32])),
        }
    }

    /// The window `page`, `per_page` and `cursor` ask for. The cursor wins
    /// over `page`; one this server did not sign is refused, never restarted.
    pub fn window(
        &self,
        page: Option<i64>,
        per_page: Option<i64>,
        cursor: Option<&str>,
    ) -> Result<Window, ApiError> {
        let per_page = per_page.unwrap_or(MAX_PER_PAGE);
        if !(1..=MAX_PER_PAGE).contains(&per_page) {
            return Err(ApiError::unprocessable(format!(
                "per_page must be between 1 and {MAX_PER_PAGE}"
            )));
        }
        if let Some(cursor) = cursor {
            let cursor = self
                .decode(cursor)
                .ok_or_else(|| ApiError::bad_request("The cursor is invalid"))?;
            return Ok(Window {
                page: 1,
                per_page,
                offset: 0,
                cursor: Some(cursor),
            });
        }
        let page = page.unwrap_or(1);
        if page < 1 {
            return Err(ApiError::unprocessable("page must be positive"));
        }
        let offset = (page - 1)
            .checked_mul(per_page)
            .ok_or_else(|| ApiError::unprocessable("Pagination offset is too large"))?;
        Ok(Window {
            page,
            per_page,
            offset,
            cursor: None,
        })
    }

    /// The envelope of a list paged by number only.
    pub fn pages<T: serde::Serialize>(
        &self,
        kind: &str,
        items: Vec<T>,
        window: &Window,
        total: i64,
    ) -> HarvestPagination<T> {
        HarvestPagination::new(kind, items, window.page, window.per_page, total, &self.url)
            .with_query(self.query.as_deref())
    }

    /// The envelope of a catalogue list, whose `next` and `previous` links
    /// carry cursors. `key` gives a row's position.
    pub fn catalogue<R, T: serde::Serialize>(
        &self,
        kind: &str,
        listed: &Listed<R>,
        key: impl Fn(&R) -> Key,
        items: Vec<T>,
        window: &Window,
    ) -> HarvestPagination<T> {
        let per_page = window.per_page;
        let page = listed.start / per_page + 1;
        let end = listed.start + i64::try_from(items.len()).unwrap_or(i64::MAX);
        let mut envelope =
            HarvestPagination::new(kind, items, page, per_page, listed.total, &self.url);
        if let (Some(first), Some(last)) = (listed.rows.first(), listed.rows.last()) {
            // A cursor may start between page boundaries, so what follows is
            // decided by the rows, not by the page number.
            envelope.next_page = (end < listed.total).then_some(page + 1);
            envelope.links.next = envelope.next_page.map(|_| {
                self.cursor_link(
                    Cursor {
                        key: key(last),
                        before: false,
                    },
                    per_page,
                )
            });
            envelope.links.previous = (listed.start > 0).then(|| {
                self.cursor_link(
                    Cursor {
                        key: key(first),
                        before: true,
                    },
                    per_page,
                )
            });
        }
        envelope.with_query(self.query.as_deref())
    }

    fn cursor_link(&self, cursor: Cursor, per_page: i64) -> String {
        format!(
            "{}?cursor={}&per_page={per_page}",
            self.url,
            self.encode(cursor)
        )
    }

    fn encode(&self, cursor: Cursor) -> String {
        let mut bytes = vec![VERSION, u8::from(cursor.before)];
        bytes.extend_from_slice(&cursor.key.at.timestamp_micros().to_be_bytes());
        bytes.extend_from_slice(cursor.key.id.as_bytes());
        let mac = self.mac(&bytes).finalize().into_bytes();
        bytes.extend_from_slice(&mac);
        URL_SAFE_NO_PAD.encode(bytes)
    }

    fn decode(&self, cursor: &str) -> Option<Cursor> {
        let bytes = URL_SAFE_NO_PAD.decode(cursor).ok()?;
        let (body, mac) = bytes.split_at_checked(bytes.len().checked_sub(MAC_LEN)?)?;
        self.mac(body).verify_slice(mac).ok()?;
        let [VERSION, before @ (0 | 1), rest @ ..] = body else {
            return None;
        };
        let (micros, id) = rest.split_at_checked(8)?;
        Some(Cursor {
            key: Key {
                at: DateTime::from_timestamp_micros(i64::from_be_bytes(micros.try_into().ok()?))?,
                id: Uuid::from_slice(id).ok()?,
            },
            before: *before == 1,
        })
    }

    fn mac(&self, bytes: &[u8]) -> Hmac<Sha256> {
        let mut mac =
            Hmac::<Sha256>::new_from_slice(self.key.bytes()).expect("HMAC accepts any key length");
        mac.update(bytes);
        mac
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> ListRequest {
        ListRequest {
            url: "https://example.test/v2/clients".into(),
            query: None,
            key: Arc::new(CursorKey::new([7; 32])),
        }
    }

    fn cursor() -> Cursor {
        Cursor {
            key: Key {
                at: DateTime::from_timestamp_micros(1_790_000_000_123_456).unwrap(),
                id: Uuid::now_v7(),
            },
            before: true,
        }
    }

    #[test]
    fn a_cursor_reads_back_its_position() {
        let request = request();
        let cursor = cursor();
        assert_eq!(request.decode(&request.encode(cursor)), Some(cursor));
    }

    #[test]
    fn a_changed_cursor_is_refused() {
        let request = request();
        let mut bytes = URL_SAFE_NO_PAD.decode(request.encode(cursor())).unwrap();
        bytes[3] ^= 1;
        assert_eq!(request.decode(&URL_SAFE_NO_PAD.encode(bytes)), None);
    }

    #[test]
    fn a_cursor_signed_with_another_key_is_refused() {
        let other = ListRequest {
            key: Arc::new(CursorKey::new([8; 32])),
            ..request()
        };
        assert_eq!(request().decode(&other.encode(cursor())), None);
    }
}
