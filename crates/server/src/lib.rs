//! Horae's backend: the operations, the database, the background jobs, the
//! importers and plugins. It knows nothing about Dioxus or Axum; the `horae`
//! crate puts its server functions and HTTP routes in front of it.

pub mod auth;
pub mod config;
pub mod db;
pub mod importers;
pub mod init;
pub mod jobs;
pub mod notifications;
pub mod ops;
pub mod plugin;
pub mod render;
pub mod seed;
pub mod state;
#[cfg(any(test, feature = "test-support"))]
pub mod testing;
pub mod timezone;

// The UI↔server types live in their own crate; keep the `crate::models` paths.
use horae_types as models;
