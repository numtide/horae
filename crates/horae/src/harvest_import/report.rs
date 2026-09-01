//! The run report type. The data and its reconciliation live in `horae-core`
//! (pure, so they cross the `#[server]` boundary onto the web target); this
//! module re-exports it under the server-side importer for local use (FR-021).

pub use horae_core::harvest_import::types::ImportReport;
