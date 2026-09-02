//! Data importers: source-specific fetching plus the shared apply engine.
//!
//! Only Harvest exists today (`importers::harvest`). The pure parsing and
//! conversion live in `horae_core::importers::harvest`; this module owns the I/O
//! (OAuth, credentials, HTTP, DB writes) that core cannot depend on.

pub mod harvest;
