use super::*;
use crate::models::permission_editor::PermissionRequester;
use crate::models::time_report::{TimeReportGrouping, TimeReportPolicy};
use crate::reports::limits::time::authorize_rows;
use cursor::groups::{Fragment, GroupRow};

pub(in crate::reports) async fn download(
    pool: PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    params: ExportParams,
    dimension: TimeReportGrouping,
) -> Result<Response, StatusCode> {
    let query = params.time_query()?;
    let requester = PermissionRequester {
        org_id,
        user_id: actor_id,
    };
    if params.expected_policy == Some(TimeReportPolicy::Legacy)
        || query
            .expected_requester
            .is_some_and(|expected| expected != requester)
    {
        return Err(StatusCode::FORBIDDEN);
    }
    response(
        &EXPORTS,
        DOWNLOAD_TIMEOUT,
        move |sender, filename| async move {
            let mut connection = pool.acquire().await.map_err(database_error)?;
            connection.close_on_drop();
            let mut tx = connection.begin().await.map_err(database_error)?;
            let (authority, policy) = Authority::time(&mut tx, org_id, actor_id).await?;
            if policy != 1 {
                return Err(StatusCode::FORBIDDEN);
            }
            cursor::groups::declare(&mut tx, org_id, actor_id, &query, dimension).await?;
            let _ = filename.send("time-report.csv".to_owned());
            deliver(&mut tx, &sender, &authority, dimension).await?;
            tx.commit().await.map_err(database_error)
        },
    )
    .await
}

struct Source {
    rows: std::vec::IntoIter<GroupRow>,
}

impl Source {
    async fn next(
        &mut self,
        connection: &mut PgConnection,
    ) -> Result<Option<Fragment>, StatusCode> {
        if self.rows.len() == 0 {
            self.rows = cursor::groups::fetch(connection, CHUNK_ROWS as i32)
                .await?
                .into_iter();
        }
        self.rows
            .next()
            .map(|row| {
                row.into_fragment()?
                    .ok_or(StatusCode::INTERNAL_SERVER_ERROR)
            })
            .transpose()
    }
}

pub(super) async fn deliver(
    connection: &mut PgConnection,
    sender: &mpsc::Sender<Vec<u8>>,
    authority: &Authority,
    dimension: TimeReportGrouping,
) -> Result<(), StatusCode> {
    let mut first_rows = cursor::groups::fetch(connection, 1).await?;
    let mut next = first_rows
        .pop()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
        .into_fragment()?;
    let mut source = Source {
        rows: Vec::new().into_iter(),
    };
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer
        .write_record(crate::reports::groups::headers(dimension))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if next.is_none() {
        let permit = sender
            .reserve()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        authority.check(connection, &[], &[]).await?;
        permit.send(
            writer
                .into_inner()
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        );
        return Ok(());
    }
    while let Some(first) = next {
        // Output capacity is reserved without gates; all fragments then share
        // one authorization lifetime, even if fetching them needs many batches.
        let permit = sender
            .reserve()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let grants = authority.hold_time(connection).await?;
        let group_id = first.group_id;
        let mut fragment = first;
        let mut contexts = Vec::with_capacity(CHUNK_ROWS);
        loop {
            if fragment.group_id != group_id {
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
            contexts.push(fragment.context);
            if contexts.len() == CHUNK_ROWS || fragment.last_context {
                authorize_rows(
                    connection,
                    PermissionRequester {
                        org_id: authority.org_id,
                        user_id: authority.actor_id,
                    },
                    &grants,
                    &contexts,
                )
                .await?;
                contexts.clear();
            }
            if fragment.last_context {
                let nonbillable = fragment
                    .rounded_minutes
                    .checked_sub(fragment.billable_minutes)
                    .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
                writer
                    .write_record([
                        fragment.name,
                        crate::reports::format_hours2(fragment.rounded_minutes),
                        crate::reports::format_hours2(fragment.billable_minutes),
                        crate::reports::format_hours2(nonbillable),
                    ])
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                break;
            }
            fragment = source
                .next(connection)
                .await?
                .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
        }
        let bytes = writer
            .into_inner()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        delivery::release_authority(connection).await?;
        permit.send(bytes);
        writer = csv::Writer::from_writer(Vec::new());
        next = source.next(connection).await?;
    }
    Ok(())
}
