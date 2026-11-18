//! `GET /api/import/harvest/jobs/{job_id}/errors`: an import job's complete
//! error details, streamed from the archive the job wrote.

use axum::http::StatusCode;
use uuid::Uuid;

use crate::ops;

#[cfg(test)]
mod stream_tests;

/// Download a single confirmed report snapshot; later checkpoints are not part
/// of this response. A missing archive fragment fails the body, never silently
/// turns into a successful but incomplete download.
pub async fn download(
    session: tower_sessions::Session,
    axum::extract::Path(job_id): axum::extract::Path<Uuid>,
) -> Result<axum::response::Response, StatusCode> {
    let user_id = crate::auth::session::get_session_user_id(&session)
        .await
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let state = crate::state::global_state().await;
    let org_id = sqlx::query_scalar!(
        "SELECT org_id FROM users WHERE id = $1 AND active = true",
        user_id,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::UNAUTHORIZED)?;
    // Authority is decided once, by `download_access`, under the locks that
    // hold it until the snapshot is read.
    download_response(&state.db, org_id, user_id, job_id).await
}

async fn download_response(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    job_id: Uuid,
) -> Result<axum::response::Response, StatusCode> {
    use axum::{http::header, response::IntoResponse};
    let mut read = download_access(pool, org_id, actor_id).await?;
    let result = ops::importers::error_details(&mut read.tx, &read.actor, job_id).await;
    let ops::importers::ErrorDetails { end, tail } = read
        .finish(result)
        .await
        .map_err(super::op_status)?
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/x-ndjson".to_owned()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"import-{job_id}-errors.jsonl\""),
            ),
            (header::CACHE_CONTROL, "no-store".to_owned()),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff".to_owned()),
        ],
        download_body(pool.clone(), org_id, actor_id, job_id, end, tail),
    )
        .into_response())
}

async fn download_access(
    pool: &sqlx::PgPool,
    org_id: Uuid,
    actor_id: Uuid,
) -> Result<ops::ReadTx, StatusCode> {
    ops::begin_read(pool, ops::Caller::session(org_id, actor_id))
        .await
        .map_err(super::op_status)
}

fn download_body(
    pool: sqlx::PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    job_id: Uuid,
    end: i64,
    tail: Vec<u8>,
) -> axum::body::Body {
    use std::collections::VecDeque;

    let stream = futures_util::stream::try_unfold(
        (0_i64, VecDeque::<Vec<u8>>::new(), Some(tail)),
        move |(mut next, mut pending, mut tail)| {
            let pool = pool.clone();
            async move {
                if pending.is_empty() && (next < end || tail.is_some()) {
                    let mut read = download_access(&pool, org_id, actor_id)
                        .await
                        .map_err(|status| std::io::Error::other(status.to_string()))?;
                    let page = if next < end {
                        ops::importers::error_chunks(&mut read.tx, &read.actor, job_id, next, end)
                            .await
                            .map(Some)
                    } else {
                        // The tail still needs current authority, like every page.
                        read.actor.require_administrator().map(|()| None)
                    };
                    // Release authority and the connection before client-paced output.
                    let page = read.finish(page).await.map_err(|error| match error {
                        ops::OpError::Forbidden(_) => {
                            std::io::Error::other(super::op_status(error).to_string())
                        }
                        _ => std::io::Error::other("report archive is incomplete"),
                    })?;
                    if let Some(page) = page {
                        next += page.len() as i64;
                        pending.extend(page);
                    } else if let Some(bytes) = tail.take().filter(|bytes| !bytes.is_empty()) {
                        pending.push_back(bytes);
                    }
                }
                let chunk = pending.pop_front();
                Ok::<_, std::io::Error>(chunk.map(|chunk| (chunk, (next, pending, tail))))
            }
        },
    );
    axum::body::Body::from_stream(stream)
}
