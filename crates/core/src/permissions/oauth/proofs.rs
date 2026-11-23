//! Kani proofs over every scope bit pattern and every permission (FR-060,
//! FR-063). They reason over each scope's permission list, not over
//! `PermissionSelection`: its `BTreeSet` keeps CBMC busy past ten minutes even
//! for one symbolic grant. Together they make the ceiling the plain union of
//! the lists; that a request's grants are the person's intersected with it is
//! sampled by `properties.rs` over whole selections.

use super::{Scope, ScopeSet};
use crate::permissions::catalog::Permission;

fn any_permission() -> Permission {
    let index: usize = kani::any();
    kani::assume(index < Permission::ALL.len());
    Permission::ALL[index]
}

fn any_scope() -> Scope {
    let index: usize = kani::any();
    kani::assume(index < Scope::ALL.len());
    Scope::ALL[index]
}

/// Normalizing any scope byte, undefined bits included, is idempotent, only
/// adds, adds at most `time:read`, and leaves `time:write` with `time:read`.
/// (FR-060)
/// Catches: a normalize that drops a requested scope or needs a second pass.
#[kani::proof]
fn normalize_is_idempotent_and_only_adds_time_read() {
    let scopes = ScopeSet(kani::any());
    let normal = scopes.normalize();
    assert!(normal.normalize() == normal);
    assert!(scopes.is_subset(normal));
    assert!(normal.is_subset(scopes.with(Scope::TimeRead)));
    assert!(!normal.contains(Scope::TimeWrite) || normal.contains(Scope::TimeRead));
}

/// `time:write` lets through every permission `time:read` does, so the
/// `time:read` that normalizing adds never widens a ceiling. (FR-060)
/// Catches: a write list missing a read, which `normalize` would then grant.
#[kani::proof]
#[kani::unwind(7)]
fn time_write_lets_through_everything_time_read_does() {
    let permission = any_permission();
    if Scope::TimeRead.permissions().contains(&permission) {
        assert!(Scope::TimeWrite.permissions().contains(&permission));
    }
}

/// Every scope's list holds each prerequisite of its permissions, so closing
/// the ceiling under prerequisites adds nothing beyond the lists. (FR-063)
/// Catches: a list entry whose prerequisite the ceiling would silently add.
#[kani::proof]
#[kani::unwind(7)]
fn scope_lists_are_closed_under_prerequisites() {
    let list = any_scope().permissions();
    let index: usize = kani::any();
    kani::assume(index < list.len());
    for required in list[index].prerequisites() {
        assert!(list.contains(required));
    }
}

/// No scope lets through anything but time-entry reads and writes and catalog
/// reads, whatever the person holds. (FR-063)
/// Catches: a scope list that hands an OAuth app approvals, rates, invoices or
/// any other write.
#[kani::proof]
fn scopes_let_through_only_time_and_catalog_reads() {
    use Permission::*;
    let list = any_scope().permissions();
    let index: usize = kani::any();
    kani::assume(index < list.len());
    assert!(matches!(
        list[index],
        TimeReadOwn
            | TimeWriteOwn
            | TimeReadManaged
            | TimeWriteManaged
            | TimeReadAll
            | TimeWriteAll
            | ProjectReadManaged
            | ProjectReadAll
            | ClientReadAll
            | TaskReadAll
            | PeopleReadManaged
            | PeopleReadAll
    ));
}
