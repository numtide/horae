//! Harvest's list paging (spec 021 FR-040, FR-041, research R-8). Every list
//! takes `page` and `per_page` (1–2000, default 2000). Keyed lists also take
//! `cursor`, an opaque, signed position: the key of the row a page continues
//! after (or, going back, ends before). Catalogue lists are keyed by id,
//! newest first, as Harvest orders them by creation; time entries by
//! `(spent_date, id)`, latest first. An edit never moves a record, so a cursor
//! walk skips nothing and repeats nothing, and changes are found with
//! `updated_since`. The envelope keeps `page` and the totals filled either way.
//! Links are absolute on the host the request arrived on.

use std::convert::Infallible;
use std::sync::Arc;

use axum::extract::{FromRequestParts, OriginalUri};
use axum::http::{header, request::Parts};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use hmac::{Hmac, Mac};
use horae_harvest::v2::compat::HarvestPagination;
use horae_server::config::CursorKey;
use sha2::Sha256;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::HarvestState;
use crate::errors::ApiError;

pub(crate) const MAX_PER_PAGE: i64 = 2000;

const VERSION: u8 = 2;
const MAC_LEN: usize = 32;
/// Keeps these cursors apart from `/api/v1`'s, which share the key.
const DOMAIN: &[u8] = b"harvest-v2-cursor";

/// Where a row sits in its list: its id, after its date on dated lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Key {
    pub date: Option<NaiveDate>,
    pub id: Uuid,
}

impl Key {
    pub fn id(id: Uuid) -> Self {
        Self { date: None, id }
    }
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

/// A window's cursor as query parameters: no cursor leaves `date` and `id`
/// `None`, and `before` says the page ends before the key.
pub(crate) struct Position {
    pub date: Option<NaiveDate>,
    pub id: Option<Uuid>,
    pub before: bool,
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

    pub fn position(&self) -> Position {
        match self.cursor {
            Some(c) => Position {
                date: c.key.date,
                id: Some(c.key.id),
                before: c.before,
            },
            None => Position {
                date: None,
                id: None,
                before: false,
            },
        }
    }
}

/// A list's rows with its total and the index of its first row in the list.
pub(crate) struct Listed<T> {
    pub total: i64,
    pub start: i64,
    pub rows: Vec<T>,
}

/// The update time below which every committed stamp is final. A transaction
/// still writing may yet commit rows stamped at or after its own start, so
/// lists serve only rows stamped before the oldest such start (and before
/// now). A client that takes the latest `updated_at` it saw as its next
/// `updated_since` then misses nothing that commits late; those rows show up
/// once their transaction ends. Stamps are taken at write time
/// (`0005_updated_at_at_write.sql`), so a transaction that has not written
/// yet can only stamp later than this. `pg_stat_activity` shows other
/// sessions' transactions only for the same role, which every Horae writer
/// uses.
pub(crate) async fn horizon(tx: &mut PgConnection) -> Result<DateTime<Utc>, ApiError> {
    sqlx::query_scalar!(
        r#"SELECT LEAST(statement_timestamp(),
             (SELECT min(xact_start) FROM pg_stat_activity
              WHERE datname = current_database() AND backend_xid IS NOT NULL
                AND pid <> pg_backend_pid())) AS "horizon!: DateTime<Utc>""#
    )
    .fetch_one(tx)
    .await
    .map_err(ApiError::internal)
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

    /// The envelope of a keyed list, whose `next` and `previous` links carry
    /// cursors. `key` gives a row's position.
    pub fn keyed<R, T: serde::Serialize>(
        &self,
        kind: &str,
        listed: &Listed<R>,
        key: impl Fn(&R) -> Key,
        items: Vec<T>,
        window: &Window,
    ) -> HarvestPagination<T> {
        let per_page = window.per_page;
        // A cursor may start between page boundaries; a partial page before
        // it still counts as one.
        let page = listed.start / per_page + i64::from(listed.start % per_page != 0) + 1;
        let end = listed.start + i64::try_from(items.len()).unwrap_or(i64::MAX);
        let mut envelope =
            HarvestPagination::new(kind, items, page, per_page, listed.total, &self.url);
        if let (Some(first), Some(last)) = (listed.rows.first(), listed.rows.last()) {
            // What follows is decided by the rows, not by the page number.
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
            envelope.links.previous = envelope.previous_page.map(|_| {
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

    /// `version ‖ before ‖ has date ‖ date (days from CE, i32 BE) ‖ id ‖
    /// HMAC`. Signed, not encrypted: it shows the row's internal id, which the
    /// caller may already read.
    fn encode(&self, cursor: Cursor) -> String {
        let mut bytes = vec![
            VERSION,
            u8::from(cursor.before),
            u8::from(cursor.key.date.is_some()),
        ];
        let days = cursor.key.date.map_or(0, |date| date.num_days_from_ce());
        bytes.extend_from_slice(&days.to_be_bytes());
        bytes.extend_from_slice(cursor.key.id.as_bytes());
        let mac = self.mac(&bytes).finalize().into_bytes();
        bytes.extend_from_slice(&mac);
        URL_SAFE_NO_PAD.encode(bytes)
    }

    fn decode(&self, cursor: &str) -> Option<Cursor> {
        let bytes = URL_SAFE_NO_PAD.decode(cursor).ok()?;
        let (body, mac) = bytes.split_at_checked(bytes.len().checked_sub(MAC_LEN)?)?;
        self.mac(body).verify_slice(mac).ok()?;
        let [VERSION, before @ (0 | 1), dated @ (0 | 1), rest @ ..] = body else {
            return None;
        };
        let (days, id) = rest.split_at_checked(4)?;
        let date = match dated {
            1 => Some(NaiveDate::from_num_days_from_ce_opt(i32::from_be_bytes(
                days.try_into().ok()?,
            ))?),
            _ => None,
        };
        Some(Cursor {
            key: Key {
                date,
                id: Uuid::from_slice(id).ok()?,
            },
            before: *before == 1,
        })
    }

    fn mac(&self, bytes: &[u8]) -> Hmac<Sha256> {
        let mut mac =
            Hmac::<Sha256>::new_from_slice(self.key.bytes()).expect("HMAC accepts any key length");
        mac.update(DOMAIN);
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

    fn cursor(date: Option<NaiveDate>) -> Cursor {
        Cursor {
            key: Key {
                date,
                id: Uuid::now_v7(),
            },
            before: true,
        }
    }

    #[test]
    fn a_cursor_reads_back_its_position() {
        let request = request();
        for date in [None, NaiveDate::from_ymd_opt(2026, 9, 7)] {
            let cursor = cursor(date);
            assert_eq!(request.decode(&request.encode(cursor)), Some(cursor));
        }
    }

    #[test]
    fn a_changed_cursor_is_refused() {
        let request = request();
        let mut bytes = URL_SAFE_NO_PAD
            .decode(request.encode(cursor(None)))
            .unwrap();
        bytes[8] ^= 1;
        assert_eq!(request.decode(&URL_SAFE_NO_PAD.encode(bytes)), None);
    }

    #[test]
    fn a_cursor_signed_with_another_key_is_refused() {
        let other = ListRequest {
            key: Arc::new(CursorKey::new([8; 32])),
            ..request()
        };
        assert_eq!(request().decode(&other.encode(cursor(None))), None);
    }

    #[test]
    fn page_numbers_follow_the_rows_when_a_cursor_starts_mid_page() {
        let request = request();
        let window = request.window(None, Some(5), None).unwrap();
        let listed = Listed {
            total: 20,
            start: 3,
            rows: vec![Uuid::now_v7(); 5],
        };
        let envelope = request.keyed(
            "clients",
            &listed,
            |id| Key::id(*id),
            listed.rows.clone(),
            &window,
        );
        assert_eq!(envelope.page, 2);
        assert_eq!(envelope.previous_page, Some(1));
        assert!(envelope.links.previous.is_some());
        assert_eq!(envelope.next_page, Some(3));
    }
}
