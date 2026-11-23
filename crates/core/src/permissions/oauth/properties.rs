//! Scopes only narrow (FR-063): for every person's selection and every scope
//! set, the narrowed grants are a subset of the person's, stay closed under
//! prerequisites, and hold nothing but time-entry reads and writes and catalog
//! reads.

use proptest::prelude::*;

use super::{Scope, ScopeSet};
use crate::permissions::catalog::{Permission, PermissionSelection};

fn selection() -> impl Strategy<Value = PermissionSelection> {
    prop::collection::vec(prop::sample::select(Permission::ALL), 0..20)
        .prop_map(|grants| PermissionSelection::new(&grants))
}

fn scopes() -> impl Strategy<Value = ScopeSet> {
    prop::sample::subsequence(Scope::ALL.to_vec(), 0..=3).prop_map(|scopes| ScopeSet::of(&scopes))
}

/// Everything any scope may let through (FR-063). An allow-list: a new
/// permission is outside it until someone decides otherwise.
fn within_every_scope(permission: Permission) -> bool {
    use Permission::*;
    matches!(
        permission,
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
    )
}

proptest! {
    #[test]
    fn narrowed_grants_are_a_subset_of_the_persons(grants in selection(), scopes in scopes()) {
        let narrowed = grants.intersection(&scopes.ceiling());
        prop_assert!(narrowed.iter().all(|permission| grants.contains(permission)));
    }

    #[test]
    fn narrowed_grants_stay_closed_under_prerequisites(grants in selection(), scopes in scopes()) {
        let narrowed = grants.intersection(&scopes.ceiling());
        for permission in narrowed.iter() {
            for &required in permission.prerequisites() {
                prop_assert!(narrowed.contains(required), "{permission:?} without {required:?}");
            }
        }
    }

    #[test]
    fn narrowed_grants_hold_only_time_and_catalog_reads(grants in selection(), scopes in scopes()) {
        for permission in grants.intersection(&scopes.ceiling()).iter() {
            prop_assert!(within_every_scope(permission), "{permission:?}");
        }
    }

    #[test]
    fn a_wider_scope_set_never_narrows_further(grants in selection(), a in scopes(), b in scopes()) {
        let wider = ScopeSet::of(&a.iter().chain(b.iter()).collect::<Vec<_>>());
        let narrow = grants.intersection(&a.ceiling());
        let wide = grants.intersection(&wider.ceiling());
        prop_assert!(narrow.iter().all(|permission| wide.contains(permission)));
    }
}
