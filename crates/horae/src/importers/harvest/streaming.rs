//! A single-page queue backpressures blocking HTTP while SQL stays async.

use std::sync::Arc;

use anyhow::{Context, bail};
use chrono::{DateTime, Utc};
use horae_core::importers::harvest::types::{EntityType, ImportMode, SourceKind};
use sqlx::{Acquire, PgConnection, Postgres, Transaction, pool::PoolConnection};
use tokio::sync::{Mutex, mpsc};
use uuid::Uuid;

use super::api_source::http::ApiHttp;
use super::api_source::{
    ApiClient, ApiProject, ApiSource, ApiTask, ApiTimeEntry, ApiUser, HarvestData, RowLookup,
};
use super::report::ImportReport;
use super::resolve::{OrgDefaults, RunCache};
use super::{apply_rows, credentials, parents};

pub(super) enum Page {
    Catalog(HarvestData),
    Entries(Vec<ApiTimeEntry>),
    Complete,
}

#[derive(Debug, thiserror::Error)]
#[error("Harvest download ended before completion")]
struct IncompleteDownload;

pub(super) fn fetch(
    mut http: ApiHttp,
    access: &str,
    account: &str,
    since: Option<DateTime<Utc>>,
    pages: &mpsc::Sender<Page>,
) -> anyhow::Result<()> {
    let mut catalog = HarvestData::default();
    http.pages::<ApiClient>(
        access,
        account,
        "clients",
        None,
        || pages.is_closed(),
        |page| {
            catalog.clients.extend(page);
            Ok(())
        },
    )?;
    http.pages::<ApiProject>(
        access,
        account,
        "projects",
        None,
        || pages.is_closed(),
        |page| {
            catalog.projects.extend(page);
            Ok(())
        },
    )?;
    http.pages::<ApiTask>(
        access,
        account,
        "tasks",
        None,
        || pages.is_closed(),
        |page| {
            catalog.tasks.extend(page);
            Ok(())
        },
    )?;
    http.pages::<ApiUser>(
        access,
        account,
        "users",
        None,
        || pages.is_closed(),
        |page| {
            catalog.users.extend(page);
            Ok(())
        },
    )?;
    pages
        .blocking_send(Page::Catalog(catalog))
        .context("Harvest import cancelled")?;
    http.pages(
        access,
        account,
        "time_entries",
        since,
        || pages.is_closed(),
        |page| {
            pages
                .blocking_send(Page::Entries(page))
                .context("Harvest import cancelled")
        },
    )?;
    Ok(())
}

/// The worker keeps shared ownership of the close-on-drop session without
/// locking it. Cancelling SQL drops its transaction and receiver, but cannot
/// release the advisory lock until the in-flight blocking HTTP request exits.
/// This still needs only one pool connection, even when the pool maximum is one.
pub(super) async fn run(
    connection: PoolConnection<Postgres>,
    org_id: Uuid,
    currency: &str,
    mode: ImportMode,
    captured_at: DateTime<Utc>,
    fetch: impl FnOnce(&mpsc::Sender<Page>) -> anyhow::Result<()> + Send + 'static,
) -> anyhow::Result<ImportReport> {
    let session = Arc::new(Mutex::new(connection));
    let worker_session = session.clone();
    let (send, mut receive) = mpsc::channel(1);
    let worker = tokio::task::spawn_blocking(move || {
        let _keep_session = worker_session;
        fetch(&send)?;
        send.blocking_send(Page::Complete)
            .context("Harvest import cancelled")
    });
    let result = {
        let mut guard = session.lock().await;
        apply(
            &mut guard,
            org_id,
            currency,
            mode,
            captured_at,
            &mut receive,
        )
        .await
    };
    // Wake a backpressured producer on every SQL/error path before joining it.
    drop(receive);
    let fetched = worker.await.context("Harvest HTTP task failed");
    let connection = Arc::try_unwrap(session)
        .map_err(|_| anyhow::anyhow!("Harvest session is still in use"))?
        .into_inner();
    super::release_import(connection).await?;
    // Preserve the download failure rather than the consumer's missing-complete
    // error. A producer cancelled by a SQL failure must not hide that SQL error.
    match (result, fetched) {
        (Ok(report), Ok(Ok(()))) => Ok(report),
        (Err(error), Ok(Ok(()))) => Err(error),
        (Err(error), Ok(Err(fetch_error))) if error.is::<IncompleteDownload>() => Err(fetch_error),
        (Err(error), Err(fetch_error)) if error.is::<IncompleteDownload>() => Err(fetch_error),
        (Err(error), _) => Err(error),
        (_, Ok(Err(error))) | (_, Err(error)) => Err(error),
    }
}

async fn apply(
    connection: &mut PgConnection,
    org_id: Uuid,
    currency: &str,
    mode: ImportMode,
    captured_at: DateTime<Utc>,
    pages: &mut mpsc::Receiver<Page>,
) -> anyhow::Result<ImportReport> {
    let Some(Page::Catalog(catalog)) = pages.recv().await else {
        return Err(IncompleteDownload.into());
    };
    let mut tx = begin_transaction(connection).await?;
    let org = OrgDefaults {
        org_id,
        default_currency: currency,
    };
    let mut cache = RunCache::default();
    let mut report = ImportReport::new(SourceKind::HarvestApi, mode);
    parents::apply(&mut tx, &mut cache, org, &catalog, &mut report).await?;
    let lookup = RowLookup::new(&catalog);
    let mut high_water = None;
    let mut missing_timestamp = false;
    loop {
        match pages.recv().await.ok_or(IncompleteDownload)? {
            Page::Entries(entries) => {
                for entry in &entries {
                    if let Some(timestamp) = entry.updated_at {
                        high_water = Some(
                            high_water.map_or(timestamp, |old: DateTime<Utc>| old.max(timestamp)),
                        );
                    } else {
                        missing_timestamp = true;
                    }
                }
                apply_rows(
                    &mut tx,
                    &mut cache,
                    &mut report,
                    org,
                    ApiSource::new(&lookup, &entries),
                )
                .await?;
            }
            Page::Complete => break,
            Page::Catalog(_) => bail!("unexpected repeated Harvest catalog"),
        }
    }
    if mode == ImportMode::Commit
        && report.error_count() == 0
        && !missing_timestamp
        && let Some(high_water) = high_water
        && let Some(mark) = high_water
            .min(captured_at)
            .checked_sub_signed(chrono::Duration::seconds(1))
    {
        credentials::advance_watermark(&mut *tx, org_id, &[(EntityType::TimeEntry, mark)]).await?;
    }
    match mode {
        ImportMode::Commit => tx.commit().await?,
        ImportMode::DryRun => tx.rollback().await?,
    }
    Ok(report)
}

async fn begin_transaction(
    connection: &mut PgConnection,
) -> anyhow::Result<Transaction<'_, Postgres>> {
    let mut tx = connection.begin().await?;
    // Fresh imports grow tables inside one transaction, before autovacuum can
    // analyze those rows. A plan cached against the initial tiny table can scan every
    // provenance row for each lookup. Re-plan only inside this import transaction.
    sqlx::query!("SET LOCAL plan_cache_mode = force_custom_plan")
        .execute(&mut *tx)
        .await?;
    Ok(tx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(migrations = "./migrations")]
    async fn api_planning_policy_ends_with_the_transaction(pool: sqlx::PgPool) {
        let mut connection = pool.acquire().await.unwrap();
        connection.close_on_drop();
        sqlx::query!("SET plan_cache_mode = force_generic_plan")
            .execute(&mut *connection)
            .await
            .unwrap();
        for mode in [ImportMode::Commit, ImportMode::DryRun] {
            let mut tx = begin_transaction(&mut connection).await.unwrap();
            let during =
                sqlx::query_scalar!(r#"SELECT current_setting('plan_cache_mode') AS "mode!""#)
                    .fetch_one(&mut *tx)
                    .await
                    .unwrap();
            assert_eq!(during, "force_custom_plan");
            match mode {
                ImportMode::Commit => tx.commit().await.unwrap(),
                ImportMode::DryRun => tx.rollback().await.unwrap(),
            }
            let after =
                sqlx::query_scalar!(r#"SELECT current_setting('plan_cache_mode') AS "mode!""#)
                    .fetch_one(&mut *connection)
                    .await
                    .unwrap();
            assert_eq!(after, "force_generic_plan");
        }
        connection.close().await.unwrap();
    }
}
