use super::PERMISSION_CATALOG_VERSION;
use super::{BuiltInProfile, Permission, PermissionSelection, StoredPermissionError};

#[test]
fn restores_each_builtin_selection_without_changing_grants() {
    for profile in BuiltInProfile::ALL {
        let selection = profile.selection();
        let stored: Vec<_> = selection.iter().collect();
        assert_eq!(
            PermissionSelection::from_stored(PERMISSION_CATALOG_VERSION, &stored),
            Ok(selection),
            "{profile:?}"
        );
    }
}

#[test]
fn restores_custom_selection_without_reapplying_profile_defaults() {
    let mut selection = BuiltInProfile::ExecutiveManager.selection();
    selection.remove(Permission::TimeReadAll).unwrap();
    selection.add(Permission::BillingRead);
    let stored: Vec<_> = selection.iter().collect();
    assert_eq!(
        PermissionSelection::from_stored(PERMISSION_CATALOG_VERSION, &stored),
        Ok(selection)
    );
}

#[test]
fn stored_order_does_not_change_the_selected_set() {
    let selection = BuiltInProfile::Administrator.selection();
    let mut stored: Vec<_> = selection.iter().collect();
    stored.reverse();
    assert_eq!(
        PermissionSelection::from_stored(PERMISSION_CATALOG_VERSION, &stored),
        Ok(selection)
    );
}

#[test]
fn rejects_unsupported_versions_even_for_valid_grants() {
    let stored: Vec<_> = BuiltInProfile::Member.selection().iter().collect();
    for version in [0, PERMISSION_CATALOG_VERSION + 1, u32::MAX] {
        assert_eq!(
            PermissionSelection::from_stored(version, &stored),
            Err(StoredPermissionError::UnsupportedCatalogVersion)
        );
    }
}

#[test]
fn rejects_empty_saved_selection_instead_of_adding_member_floor() {
    assert_eq!(
        PermissionSelection::from_stored(PERMISSION_CATALOG_VERSION, &[]),
        Err(StoredPermissionError::NonCanonicalSelection)
    );
}

#[test]
fn rejects_each_missing_floor_grant() {
    for missing in BuiltInProfile::Member.selection().iter() {
        let stored: Vec<_> = Permission::ALL
            .iter()
            .copied()
            .filter(|permission| *permission != missing)
            .collect();
        assert_eq!(
            PermissionSelection::from_stored(PERMISSION_CATALOG_VERSION, &stored),
            Err(StoredPermissionError::NonCanonicalSelection),
            "missing {missing:?}"
        );
    }
}

#[test]
fn rejects_every_missing_prerequisite_without_repairing_input() {
    for permission in Permission::ALL {
        for required in permission.prerequisites() {
            let selection = PermissionSelection::new(&[*permission]);
            let stored: Vec<_> = selection.iter().filter(|grant| grant != required).collect();
            let before = stored.clone();
            assert_eq!(
                PermissionSelection::from_stored(PERMISSION_CATALOG_VERSION, &stored),
                Err(StoredPermissionError::NonCanonicalSelection),
                "{permission:?} without {required:?}"
            );
            assert_eq!(stored, before);
        }
    }
}

#[test]
fn rejects_duplicate_grants_instead_of_silently_deduplicating() {
    for duplicate in Permission::ALL {
        let mut stored = Permission::ALL.to_vec();
        stored.push(*duplicate);
        assert_eq!(
            PermissionSelection::from_stored(PERMISSION_CATALOG_VERSION, &stored),
            Err(StoredPermissionError::DuplicatePermission),
            "{duplicate:?}"
        );
    }
}

#[test]
fn restores_each_valid_grant_pair_and_its_revoked_selection() {
    for &first in Permission::ALL {
        for &second in Permission::ALL {
            let mut selection = PermissionSelection::new(&[first, second]);
            let stored: Vec<_> = selection.iter().collect();
            assert_eq!(
                PermissionSelection::from_stored(PERMISSION_CATALOG_VERSION, &stored),
                Ok(selection.clone())
            );
            if selection.remove(first).is_ok() {
                let stored: Vec<_> = selection.iter().collect();
                assert_eq!(
                    PermissionSelection::from_stored(PERMISSION_CATALOG_VERSION, &stored),
                    Ok(selection)
                );
            }
        }
    }
}
