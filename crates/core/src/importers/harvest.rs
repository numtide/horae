//! Pure-domain building blocks for the Harvest importer (Constitution II).
//!
//! This module holds the parts that must be correct regardless of where the data
//! came from and must be unit-testable without a database or a network:
//!
//! - [`convert`] — decimal hours → exact integer minutes and decimal money →
//!   integer minor units (cents), the inverse of the `minutes/60` and `cents/100`
//!   transforms the Harvest exporter applies.
//! - [`keys`] — natural-key normalization (trim + case-fold) and the composite
//!   key builders used as the matching fallback.
//! - [`types`] — the source-agnostic [`types::SourceRow`] both adapters produce,
//!   the per-record [`types::RowOutcome`], the [`types::ImportSummary`], and the
//!   [`types::ImportMode`] / [`types::SyncScope`] mode enums.
//!
//! Everything here is free of I/O dependencies; the HTTP pull, OAuth, credential
//! encryption, and database writes live in the server crate.

pub mod convert;
pub mod keys;
pub mod types;
