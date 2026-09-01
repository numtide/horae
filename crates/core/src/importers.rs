//! Data importers — pure, I/O-free parsing and conversion for each source.
//!
//! Only Harvest exists today (`importers::harvest`); the namespace leaves room
//! for further sources without a rename. I/O (OAuth, DB, HTTP) lives in the app
//! crate, never here.

pub mod harvest;
