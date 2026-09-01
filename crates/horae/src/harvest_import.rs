//! Server-side Harvest importer: the source-agnostic engine plus the two source
//! adapters, OAuth connect flow, credential storage, and provenance-backed
//! resolve/apply (plan.md **Project Structure**).
//!
//! The engine ([`run_import`]) drives a stream of [`SourceRow`]s — produced by
//! either the API adapter or the CSV adapter — through resolve → apply → report.
//! It never learns which adapter produced a row. Each row is applied in its own
//! savepoint (see [`apply`]); a `DryRun` runs the whole stream inside a
//! transaction that is rolled back, so nothing persists — not data, not
//! provenance, not the watermark (FR-014, research.md §7).

pub mod apply;
pub mod credentials;
pub mod provenance;
pub mod report;
pub mod resolve;

use horae_core::harvest_import::types::{ImportMode, SourceKind, SourceRow};
use sqlx::PgPool;
use uuid::Uuid;

use report::ImportReport;
use resolve::{OrgDefaults, RunCache};

/// A source of normalized rows the engine consumes lazily (research.md §9). Both
/// adapters implement it: the CSV adapter walks its parsed records, the API
/// adapter walks Harvest pages. Returning `None` ends the run.
pub trait RowSource {
    fn next_row(&mut self) -> impl Future<Output = anyhow::Result<Option<SourceRow>>> + Send;
}

/// Drive a source through the engine and return the run report. In `Commit` mode
/// the outer transaction is committed; in `DryRun` it is rolled back so nothing
/// persists (FR-014). Advancing the incremental watermark on a committing API run
/// is the caller's responsibility, done only after this returns success.
pub async fn run_import<S: RowSource>(
    pool: &PgPool,
    org_id: Uuid,
    default_currency: &str,
    source: SourceKind,
    mode: ImportMode,
    mut src: S,
) -> anyhow::Result<ImportReport> {
    let mut report = ImportReport::new(source, mode);
    let mut cache = RunCache::default();
    let org = OrgDefaults {
        org_id,
        default_currency,
    };

    let mut tx = pool.begin().await?;
    while let Some(row) = src.next_row().await? {
        let result = apply::apply_row(&mut tx, &mut cache, org, &row).await;
        for (entity, outcome) in &result.outcomes {
            report.record(*entity, outcome);
        }
    }

    match mode {
        ImportMode::Commit => tx.commit().await?,
        ImportMode::DryRun => tx.rollback().await?,
    }

    debug_assert!(report.reconciles());
    Ok(report)
}

/// An in-memory row source over a `Vec` — used by the CSV adapter (after parsing)
/// and by integration tests that hand-build rows.
pub struct VecSource {
    rows: std::vec::IntoIter<SourceRow>,
}

impl VecSource {
    pub fn new(rows: Vec<SourceRow>) -> Self {
        Self {
            rows: rows.into_iter(),
        }
    }
}

impl RowSource for VecSource {
    async fn next_row(&mut self) -> anyhow::Result<Option<SourceRow>> {
        Ok(self.rows.next())
    }
}
