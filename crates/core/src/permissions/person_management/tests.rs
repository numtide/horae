use uuid::Uuid;

use super::{SelfManagement, has_person_management_grant, validate_no_self_management};
use crate::permissions::catalog::{Permission, PermissionSelection};

// Independent expected mapping from the person-management contract, not a
// production allow-list: project/invoice/cost/report grants cannot supply it.
const COMPATIBLE: &[Permission] = &[
    Permission::TimeReadManaged,
    Permission::TimeWriteManaged,
    Permission::TimeApproveManaged,
    Permission::TimeReadAll,
    Permission::TimeWriteAll,
    Permission::TimeApproveAll,
    Permission::ExpenseReadManaged,
    Permission::ExpenseWriteManaged,
    Permission::ExpenseReadAll,
    Permission::ExpenseWriteAll,
    Permission::PeopleReadManaged,
    Permission::PeopleWriteManaged,
    Permission::PeopleReadAll,
    Permission::PeopleWriteAll,
    Permission::BillableRateReadManaged,
    Permission::BillableRateWriteManaged,
    Permission::BillableRateReadAll,
    Permission::BillableRateWriteAll,
    Permission::ApprovalWithdrawManaged,
];

#[test]
fn own_tracking_floor_does_not_qualify() {
    assert!(!has_person_management_grant(&PermissionSelection::new(&[])));
}

#[test]
fn each_catalog_grant_has_the_specified_compatibility_without_changing_grants() {
    for &permission in Permission::ALL {
        let selection = PermissionSelection::new(&[permission]);
        let before = selection.clone();
        assert_eq!(
            has_person_management_grant(&selection),
            COMPATIBLE.contains(&permission),
            "{permission:?}"
        );
        assert_eq!(selection, before);
    }
}

#[test]
fn any_compatible_grant_suffices_in_every_pair() {
    for &first in Permission::ALL {
        for &second in Permission::ALL {
            let selection = PermissionSelection::new(&[first, second]);
            assert_eq!(
                has_person_management_grant(&selection),
                COMPATIBLE.contains(&first) || COMPATIBLE.contains(&second),
                "{first:?}, {second:?}"
            );
        }
    }
}

#[test]
fn combined_unrelated_grants_do_not_create_person_management_eligibility() {
    let unrelated: Vec<_> = Permission::ALL
        .iter()
        .copied()
        .filter(|permission| !COMPATIBLE.contains(permission))
        .collect();
    assert!(!has_person_management_grant(&PermissionSelection::new(
        &unrelated
    )));
}

#[test]
fn eligibility_tracks_last_compatible_grant_loss_and_restoration() {
    let mut selection = PermissionSelection::new(&[
        Permission::TimeWriteAll,
        Permission::ExpenseReadManaged,
        Permission::CostRateReadAll,
    ]);
    assert!(has_person_management_grant(&selection));
    selection.remove(Permission::TimeReadManaged).unwrap();
    assert!(has_person_management_grant(&selection));
    selection.remove(Permission::ExpenseReadManaged).unwrap();
    assert!(!has_person_management_grant(&selection));
    selection.add(Permission::ApprovalWithdrawManaged);
    assert!(has_person_management_grant(&selection));
}

#[test]
fn self_link_rejects_the_proposal_in_every_position_without_filtering_it() {
    let manager = Uuid::from_u128(1);
    let others = [Uuid::from_u128(2), Uuid::from_u128(3)];
    for proposed in [
        vec![manager],
        vec![manager, others[0], others[1]],
        vec![others[0], manager, others[1]],
        vec![others[0], others[1], manager],
        vec![manager, manager],
    ] {
        let before = proposed.clone();
        assert_eq!(
            validate_no_self_management(manager, &proposed),
            Err(SelfManagement)
        );
        assert_eq!(proposed, before);
    }
}

#[test]
fn other_people_and_empty_removal_need_neither_grants_nor_a_separate_actor() {
    let manager = Uuid::from_u128(1);
    for proposed in [vec![], vec![Uuid::from_u128(2), Uuid::from_u128(3)]] {
        let before = proposed.clone();
        assert_eq!(validate_no_self_management(manager, &proposed), Ok(()));
        assert_eq!(proposed, before);
    }
}
