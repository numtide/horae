//! Kani proofs over unbounded identifiers, every scope bit pattern, every rate
//! amount and every catalog version; run with `cargo kani -p horae-core`.
//!
//! Kani is not in nixpkgs and its setup downloads a pinned nightly toolchain,
//! so these proofs are not a nix check. Install it outside the dev shell's
//! toolchain with `cargo install --locked kani-verifier && cargo kani setup`
//! (setup needs `rustup`). The proptest properties in `properties.rs` cover the
//! same decisions inside the normal test run.

use uuid::Uuid;

use super::{
    AccessScope, Actor, ManagementAssignments, ScopedResource,
    catalog::{PERMISSION_CATALOG_VERSION, Permission, PermissionSelection},
    rates::RateEdit,
};

fn id() -> Uuid {
    Uuid::from_u128(kani::any())
}

fn maybe_id() -> Option<Uuid> {
    kani::any::<bool>().then(id)
}

/// Coverage stays within the actor's organization and the dimensions the
/// scope grants, for any ids, up to two assignments per dimension and every
/// scope byte, including bits no constant defines. (FR-006)
/// Catches: tenancy/activity bypasses, a missing dimension treated as a match,
/// or undefined scope bits granting coverage. Also proves `covers` cannot panic.
#[kani::proof]
// `Uuid` equality is a 16-byte memcmp, so every comparison loop needs 17.
#[kani::unwind(17)]
fn covers_stays_within_granted_dimensions() {
    let scope = AccessScope(kani::any());
    let actor = Actor {
        id: id(),
        org_id: id(),
        active: kani::any(),
    };
    let record = ScopedResource {
        org_id: id(),
        person_id: maybe_id(),
        project_id: maybe_id(),
    };
    let (people, projects) = ([id(), id()], [id(), id()]);
    let (people_len, projects_len): (usize, usize) = (kani::any(), kani::any());
    kani::assume(people_len <= 2 && projects_len <= 2);
    let assignments = ManagementAssignments {
        actor_id: id(),
        org_id: id(),
        people: &people[..people_len],
        projects: &projects[..projects_len],
    };
    if scope.covers(&actor, &record, &assignments) {
        assert!(actor.active && actor.id == assignments.actor_id);
        assert!(record.org_id == actor.org_id && assignments.org_id == actor.org_id);
        let person = |held: AccessScope, set: &[Uuid]| {
            scope.includes(held) && record.person_id.is_some_and(|p| set.contains(&p))
        };
        assert!(
            scope.includes(AccessScope::ORGANIZATION)
                || person(AccessScope::OWN, &[actor.id])
                || person(AccessScope::MANAGED_PEOPLE, assignments.people)
                || (scope.includes(AccessScope::MANAGED_PROJECTS)
                    && record
                        .project_id
                        .is_some_and(|p| assignments.projects.contains(&p)))
        );
    }
}

/// Widening any scope byte never removes coverage. (FR-006, FR-010)
/// Catches: bit logic where adding a flag masks another, so revocation widens.
#[kani::proof]
#[kani::unwind(17)]
fn covers_is_monotone_in_scope() {
    let (scope, extra) = (AccessScope(kani::any()), AccessScope(kani::any()));
    let actor = Actor {
        id: id(),
        org_id: id(),
        active: kani::any(),
    };
    let record = ScopedResource {
        org_id: id(),
        person_id: maybe_id(),
        project_id: maybe_id(),
    };
    let (people, projects) = ([id()], [id()]);
    let assignments = ManagementAssignments {
        actor_id: id(),
        org_id: id(),
        people: &people,
        projects: &projects,
    };
    if scope.covers(&actor, &record, &assignments) {
        assert!(scope.union(extra).covers(&actor, &record, &assignments));
    }
}

/// An explicit amount or reset never passes without write authority, for
/// every i64 including zero, and authorization never alters the edit.
/// (FR-021, FR-034)
/// Catches: "zero" or "equal to stored" shortcuts that let a read-only caller
/// write money, or an authorized edit rewritten on the way through.
#[kani::proof]
fn rate_edits_need_write_authority() {
    let edit = match kani::any::<u8>() % 3 {
        0 => RateEdit::Unchanged,
        1 => RateEdit::Reset,
        _ => RateEdit::Set(kani::any()),
    };
    let may_write: bool = kani::any();
    match edit.authorize(may_write) {
        Ok(authorized) => {
            assert!(authorized == edit);
            assert!(may_write || edit == RateEdit::Unchanged);
        }
        Err(_) => assert!(!may_write && edit != RateEdit::Unchanged),
    }
}

/// No catalog version but the current one restores a selection, whatever the
/// stored grants. (FR-015)
/// Catches: a stale or newer catalog read under today's grant meanings.
#[kani::proof]
// The set building after the version check is unreachable here; without a
// bound CBMC still unrolls it forever. Unwinding assertions stay on.
#[kani::unwind(3)]
fn other_catalog_versions_restore_nothing() {
    let version: u32 = kani::any();
    kani::assume(version != PERMISSION_CATALOG_VERSION);
    let pick = || {
        let index: usize = kani::any();
        kani::assume(index < Permission::ALL.len());
        Permission::ALL[index]
    };
    let stored = [pick(), pick()];
    assert!(PermissionSelection::from_stored(version, &stored).is_err());
}
