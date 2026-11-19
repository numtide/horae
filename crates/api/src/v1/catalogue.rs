//! What the read-only catalogue lists share: turning a shared operation's
//! page into the answer, with the next page's signed cursor.

use axum::Json;
use chrono::Utc;
use uuid::Uuid;

use super::types::Page;
use crate::cursor::{Position, Walk};

/// The page of `walk` at `position`, from `rows` read with one row more than
/// its limit.
pub fn page<R, T: From<R>>(
    walk: &Walk<'_>,
    mut rows: Vec<R>,
    position: Position,
    id: impl Fn(&R) -> Uuid,
) -> Json<Page<T>> {
    let next_cursor = walk.next_cursor(&mut rows, position, id, Utc::now());
    Json(Page {
        data: rows.into_iter().map(T::from).collect(),
        next_cursor,
    })
}

#[cfg(test)]
mod tests;
