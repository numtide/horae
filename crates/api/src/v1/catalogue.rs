//! What the read-only catalogue lists share: turning a shared operation's
//! page into the answer, with the next page's signed cursor.

use axum::Json;
use chrono::Utc;
use uuid::Uuid;

use super::types::Page;
use crate::cursor::{Position, Walk};
use crate::problem::Problem;

/// The page of `walk` at `position`, from `rows` read with one row more than
/// its limit.
pub fn page<R, T>(
    walk: &Walk<'_>,
    mut rows: Vec<R>,
    position: Position,
    id: impl Fn(&R) -> Uuid,
) -> Result<Json<Page<T>>, Problem>
where
    T: TryFrom<R>,
    Problem: From<T::Error>,
{
    let next_cursor = walk.next_cursor(&mut rows, position, id, Utc::now());
    Ok(Json(Page {
        data: rows
            .into_iter()
            .map(T::try_from)
            .collect::<Result<_, _>>()?,
        next_cursor,
    }))
}

#[cfg(test)]
mod tests;
