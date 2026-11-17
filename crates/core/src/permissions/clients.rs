//! Client grant rules shared by the Clients page and every link to it.
//!
//! `has` answers whether the person's current, trusted grants include a
//! permission. Callers still check that the person is active.

use super::catalog::Permission;

/// Whether someone may read the client catalog: the Clients page, a client's
/// details, and the links that lead there.
#[must_use]
pub fn reads_catalog(has: impl Fn(Permission) -> bool) -> bool {
    has(Permission::ClientReadAll)
}
