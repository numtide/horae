//! Sign-in. [`http`] holds the session and the `/auth/*` routes; [`oidc`]
//! (in `horae-server`) talks to the provider, links identities and provisions
//! accounts, and knows nothing about requests or sessions.

pub mod http;
pub use horae_server::auth::oidc;
