//! The run report: the per-entity summary plus the per-record error list, and the
//! `processed = created + updated + skipped + errored` reconciliation (FR-021).

use horae_core::harvest_import::types::{
    EntityType, ImportMode, ImportSummary, RowError, RowOutcome, SourceKind,
};
use serde::{Deserialize, Serialize};

/// The result of an import run, returned by every surface (server fn + CLI).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportReport {
    pub source: SourceKind,
    pub mode: ImportMode,
    pub summary: ImportSummary,
    pub row_errors: Vec<RowError>,
}

impl ImportReport {
    pub fn new(source: SourceKind, mode: ImportMode) -> Self {
        Self {
            source,
            mode,
            summary: ImportSummary::default(),
            row_errors: Vec::new(),
        }
    }

    /// Fold one entity outcome into the summary, collecting the error detail when
    /// the outcome is `Errored`.
    pub fn record(&mut self, entity: EntityType, outcome: &RowOutcome) {
        self.summary.counts_mut(entity).record(outcome);
        if let RowOutcome::Errored {
            source_location,
            reason,
        } = outcome
        {
            self.row_errors.push(RowError {
                source_location: source_location.clone(),
                entity,
                reason: reason.clone(),
            });
        }
    }

    /// True when every entity type reconciles (`processed` equals the sum of its
    /// four buckets — always true by construction, asserted in tests).
    pub fn reconciles(&self) -> bool {
        EntityType::ALL.iter().all(|&e| {
            let c = self.summary.counts(e);
            c.processed() == c.created + c.updated + c.skipped + c.errored
        })
    }

    /// Total records that errored across all entity types.
    pub fn error_count(&self) -> usize {
        self.row_errors.len()
    }
}
