//! API page and parent-batch checkpoints, committed under the job's lease fence.

use anyhow::{Context, bail, ensure};
use chrono::{DateTime, Utc};
use horae_core::importers::harvest::types::{EntityType, ImportMode, SourceKind, SyncScope};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sqlx::{PgConnection, Postgres, pool::PoolConnection};
use tokio::sync::mpsc;
use uuid::Uuid;

use super::super::api_source::http::{ApiHttp, PageCursor};
use super::super::api_source::{
    ApiClient, ApiProject, ApiSource, ApiTask, ApiTimeEntry, ApiUser, HarvestData, RowLookup,
};
use super::super::report::ImportReport;
use super::super::resolve::{OrgDefaults, RunCache};
use super::super::{apply_rows, credentials, parents};
use super::{IncompleteDownload, Page, Settings, begin_transaction};
use crate::jobs::JobLease;

const PARENT_BATCH: usize = 500;

#[derive(Serialize, Deserialize)]
pub(in crate::importers::harvest) struct Request {
    pub account_id: String,
    pub currency: String,
    pub sync: SyncScope,
    #[serde(deserialize_with = "Option::deserialize")]
    pub since: Option<DateTime<Utc>>,
    pub captured_at: DateTime<Utc>,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Collection {
    Clients,
    Projects,
    Tasks,
    Users,
    TimeEntries,
}

impl Collection {
    fn name(self) -> &'static str {
        match self {
            Self::Clients => "clients",
            Self::Projects => "projects",
            Self::Tasks => "tasks",
            Self::Users => "users",
            Self::TimeEntries => "time_entries",
        }
    }

    fn next(self) -> Option<Self> {
        match self {
            Self::Clients => Some(Self::Projects),
            Self::Projects => Some(Self::Tasks),
            Self::Tasks => Some(Self::Users),
            Self::Users => Some(Self::TimeEntries),
            Self::TimeEntries => None,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub(in crate::importers::harvest) struct Download {
    collection: Collection,
    page: PageCursor,
}

#[derive(Serialize, Deserialize)]
pub(super) struct Checkpoint {
    version: u8,
    request: Request,
    download: Download,
    catalog: HarvestData,
    parent_offset: usize,
    report: ImportReport,
    cache: RunCache,
    #[serde(deserialize_with = "Option::deserialize")]
    high_water: Option<DateTime<Utc>>,
    missing_timestamp: bool,
}

impl Checkpoint {
    fn parent_count(&self) -> usize {
        self.catalog.clients.len() + self.catalog.projects.len() + self.catalog.tasks.len()
    }

    async fn save(
        &self,
        connection: &mut PgConnection,
        lease: &JobLease,
        phase: &str,
    ) -> anyhow::Result<()> {
        let (_, count) = super::super::job_report(&self.report)?;
        lease
            .save_checkpoint(connection, &serde_json::to_value(self)?, phase, count)
            .await
    }
}

enum Rows {
    Clients(Vec<ApiClient>),
    Projects(Vec<ApiProject>),
    Tasks(Vec<ApiTask>),
    Users(Vec<ApiUser>),
    Entries(Vec<ApiTimeEntry>),
}

pub(in crate::importers::harvest) struct Batch {
    rows: Rows,
    next: Download,
}

pub(in crate::importers::harvest) async fn run(
    mut connection: PoolConnection<Postgres>,
    org_id: Uuid,
    request: Request,
    access: String,
    http: ApiHttp,
    lease: &JobLease,
) -> anyhow::Result<ImportReport> {
    let loaded = async {
        lease.check_organization(org_id)?;
        if let Some(saved) = lease.load_checkpoint(&mut connection).await? {
            let saved: Checkpoint =
                serde_json::from_value(saved).context("invalid API import checkpoint")?;
            ensure!(
                saved.version == 1,
                "unsupported API import checkpoint version"
            );
            ensure!(
                saved.request.account_id == request.account_id,
                "API checkpoint account mismatch"
            );
            ensure!(
                saved.request.sync == request.sync
                    && saved.report.mode == ImportMode::Commit
                    && saved.report.source == SourceKind::HarvestApi,
                "API checkpoint request mismatch"
            );
            ensure!(
                saved.parent_offset <= saved.parent_count(),
                "invalid API parent checkpoint offset"
            );
            ensure!(
                saved.parent_offset == 0 || saved.download.collection == Collection::TimeEntries,
                "API parent checkpoint precedes catalog completion"
            );
            Ok(saved)
        } else {
            Ok(Checkpoint {
                version: 1,
                request,
                download: Download {
                    collection: Collection::Clients,
                    page: http.cursor("clients", None)?,
                },
                catalog: HarvestData::default(),
                parent_offset: 0,
                report: ImportReport::new(SourceKind::HarvestApi, ImportMode::Commit),
                cache: RunCache::default(),
                high_water: None,
                missing_timestamp: false,
            })
        }
    }
    .await;
    let checkpoint = match loaded {
        Ok(checkpoint) => checkpoint,
        Err(error) => {
            super::super::release_import(connection).await?;
            return Err(error);
        }
    };
    let download = checkpoint.download.clone();
    let account = checkpoint.request.account_id.clone();
    let since = checkpoint.request.since;
    super::run_inner(
        connection,
        org_id,
        Settings::Durable(Box::new(checkpoint)),
        move |pages| fetch(http, &access, &account, since, download, pages),
        Some(lease),
    )
    .await
}

fn fetch(
    mut http: ApiHttp,
    access: &str,
    account: &str,
    since: Option<DateTime<Utc>>,
    mut download: Download,
    pages: &mpsc::Sender<Page>,
) -> anyhow::Result<()> {
    loop {
        match download.collection {
            Collection::Clients => fetch_pages(
                &mut http,
                (access, account),
                &mut download,
                pages,
                Rows::Clients,
            )?,
            Collection::Projects => fetch_pages(
                &mut http,
                (access, account),
                &mut download,
                pages,
                Rows::Projects,
            )?,
            Collection::Tasks => fetch_pages(
                &mut http,
                (access, account),
                &mut download,
                pages,
                Rows::Tasks,
            )?,
            Collection::Users => fetch_pages(
                &mut http,
                (access, account),
                &mut download,
                pages,
                Rows::Users,
            )?,
            Collection::TimeEntries => fetch_pages(
                &mut http,
                (access, account),
                &mut download,
                pages,
                Rows::Entries,
            )?,
        }
        let Some(collection) = download.collection.next() else {
            return Ok(());
        };
        download = Download {
            collection,
            page: http.cursor(
                collection.name(),
                if collection == Collection::TimeEntries {
                    since
                } else {
                    None
                },
            )?,
        };
        if collection == Collection::TimeEntries {
            pages
                .blocking_send(Page::CatalogReady(Box::new(download.clone())))
                .context("Harvest import cancelled")?;
        }
    }
}

fn fetch_pages<T: DeserializeOwned>(
    http: &mut ApiHttp,
    (access, account): (&str, &str),
    download: &mut Download,
    pages: &mpsc::Sender<Page>,
    rows: fn(Vec<T>) -> Rows,
) -> anyhow::Result<()> {
    let collection = download.collection;
    http.pages_from(
        access,
        account,
        collection.name(),
        &mut download.page,
        || pages.is_closed(),
        |page, cursor| {
            pages
                .blocking_send(Page::Batch(Box::new(Batch {
                    rows: rows(page),
                    next: Download {
                        collection,
                        page: cursor.clone(),
                    },
                })))
                .context("Harvest import cancelled")
        },
    )
}

pub(super) async fn apply(
    connection: &mut PgConnection,
    org_id: Uuid,
    mut checkpoint: Checkpoint,
    pages: &mut mpsc::Receiver<Page>,
    lease: &JobLease,
) -> anyhow::Result<ImportReport> {
    while checkpoint.download.collection != Collection::TimeEntries {
        match pages.recv().await.ok_or(IncompleteDownload)? {
            Page::Batch(batch) => {
                match batch.rows {
                    Rows::Clients(rows) => checkpoint.catalog.clients.extend(rows),
                    Rows::Projects(rows) => checkpoint.catalog.projects.extend(rows),
                    Rows::Tasks(rows) => checkpoint.catalog.tasks.extend(rows),
                    Rows::Users(rows) => checkpoint.catalog.users.extend(rows),
                    Rows::Entries(_) => bail!("API entries preceded catalog completion"),
                }
                checkpoint.download = batch.next;
                let mut tx = begin_transaction(connection).await?;
                checkpoint
                    .save(
                        &mut tx,
                        lease,
                        &format!("download_{}", checkpoint.download.collection.name()),
                    )
                    .await?;
                tx.commit().await?;
            }
            Page::CatalogReady(next) => checkpoint.download = *next,
            _ => return Err(IncompleteDownload.into()),
        }
    }
    let org = OrgDefaults {
        org_id,
        default_currency: &checkpoint.request.currency,
    };
    while checkpoint.parent_offset < checkpoint.parent_count() {
        let end = checkpoint
            .parent_offset
            .saturating_add(PARENT_BATCH)
            .min(checkpoint.parent_count());
        let mut tx = begin_transaction(connection).await?;
        parents::apply_batch(
            &mut tx,
            &mut checkpoint.cache,
            org,
            &checkpoint.catalog,
            &mut checkpoint.report,
            checkpoint.parent_offset..end,
        )
        .await?;
        checkpoint.parent_offset = end;
        checkpoint.save(&mut tx, lease, "catalog").await?;
        tx.commit().await?;
    }
    let lookup = RowLookup::new(&checkpoint.catalog);
    loop {
        match pages.recv().await.ok_or(IncompleteDownload)? {
            Page::Batch(batch) => {
                let Rows::Entries(entries) = batch.rows else {
                    bail!("unexpected repeated API catalog")
                };
                for entry in &entries {
                    if let Some(timestamp) = entry.updated_at {
                        checkpoint.high_water = Some(
                            checkpoint
                                .high_water
                                .map_or(timestamp, |old| old.max(timestamp)),
                        );
                    } else {
                        checkpoint.missing_timestamp = true;
                    }
                }
                let mut tx = begin_transaction(connection).await?;
                apply_rows(
                    &mut tx,
                    &mut checkpoint.cache,
                    &mut checkpoint.report,
                    org,
                    ApiSource::new(&lookup, &entries),
                )
                .await?;
                checkpoint.download = batch.next;
                checkpoint.save(&mut tx, lease, "time_entries").await?;
                tx.commit().await?;
            }
            Page::Complete => break,
            _ => bail!("unexpected API checkpoint message"),
        }
    }
    let mut tx = begin_transaction(connection).await?;
    if checkpoint.report.error_count() == 0
        && !checkpoint.missing_timestamp
        && let Some(high_water) = checkpoint.high_water
        && let Some(mark) = high_water
            .min(checkpoint.request.captured_at)
            .checked_sub_signed(chrono::Duration::seconds(1))
    {
        credentials::advance_watermark(&mut *tx, org_id, &[(EntityType::TimeEntry, mark)]).await?;
    }
    super::super::finish_import(tx, &checkpoint.report, Some(lease)).await?;
    Ok(checkpoint.report)
}
