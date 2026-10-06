use super::*;
use crate::permissions::catalog::{BuiltInProfile, Permission};

const ORG: Uuid = Uuid::from_u128(1);
const ACTOR: Uuid = Uuid::from_u128(2);
const PERSON: Uuid = Uuid::from_u128(3);
const PROJECT: Uuid = Uuid::from_u128(4);
const OTHER: Uuid = Uuid::from_u128(5);

#[test]
fn unchanged_rate_edit_requires_no_financial_write_grant() {
    assert_eq!(
        RateEdit::Unchanged.authorize(false),
        Ok(RateEdit::Unchanged)
    );
}

#[test]
fn explicit_rate_edits_require_write_even_for_zero_or_an_equal_value() {
    for edit in [RateEdit::Reset, RateEdit::Set(0), RateEdit::Set(2500)] {
        assert_eq!(edit.authorize(false), Err(RateEditDenied));
        assert_eq!(edit.authorize(true), Ok(edit));
    }
}

#[test]
fn read_only_cost_grant_cannot_authorize_an_explicit_rate_edit() {
    let read_only = PermissionSelection::new(&[Permission::CostRateReadAll]);
    let may_write = cost_rate_access(&read_only, RateAction::Write, &actor(), ORG);
    assert_eq!(RateEdit::Reset.authorize(may_write), Err(RateEditDenied));
    assert_eq!(
        RateEdit::Unchanged.authorize(may_write),
        Ok(RateEdit::Unchanged)
    );
}

fn actor() -> Actor {
    Actor {
        id: ACTOR,
        org_id: ORG,
        active: true,
    }
}

fn assignments() -> ManagementAssignments<'static> {
    ManagementAssignments {
        actor_id: ACTOR,
        org_id: ORG,
        people: &[PERSON],
        projects: &[PROJECT],
    }
}

fn resource(owner: BillableRateOwner) -> BillableRateResource {
    BillableRateResource { org_id: ORG, owner }
}

#[test]
fn billable_actions_use_only_their_own_grant_and_owner_scope() {
    use BillableRateOwner::*;
    use Permission::*;
    use RateAction::*;

    for (grants, read_managed, write_managed, read_all, write_all) in [
        (vec![], false, false, false, false),
        (vec![BillableRateReadManaged], true, false, false, false),
        (vec![BillableRateWriteManaged], true, true, false, false),
        (vec![BillableRateReadAll], true, false, true, false),
        (vec![BillableRateWriteAll], true, true, true, true),
        (
            vec![BillableRateReadAll, BillableRateWriteManaged],
            true,
            true,
            true,
            false,
        ),
    ] {
        let selection = PermissionSelection::new(&grants);
        for (action, managed, all) in [
            (Read, read_managed, read_all),
            (Write, write_managed, write_all),
        ] {
            for (owner, expected) in [
                (Person(PERSON), managed),
                (Project(PROJECT), managed),
                (Person(OTHER), all),
                (Project(OTHER), all),
                (Person(ACTOR), all),
                (GlobalTask, all),
            ] {
                assert_eq!(
                    billable_rate_access(
                        &selection,
                        action,
                        &actor(),
                        &resource(owner),
                        &assignments()
                    ),
                    expected,
                    "{grants:?} {action:?} {owner:?}"
                );
            }
        }
    }
}

#[test]
fn billable_owner_never_uses_the_other_management_dimension() {
    let selection = PermissionSelection::new(&[Permission::BillableRateWriteManaged]);
    for (people, projects, person_allowed, project_allowed) in [
        (&[PERSON][..], &[PROJECT][..], true, true),
        (&[][..], &[PROJECT][..], false, true),
        (&[PERSON][..], &[][..], true, false),
        (&[][..], &[][..], false, false),
    ] {
        let links = ManagementAssignments {
            people,
            projects,
            ..assignments()
        };
        for action in [RateAction::Read, RateAction::Write] {
            for (owner, expected) in [
                (BillableRateOwner::Person(PERSON), person_allowed),
                (BillableRateOwner::Project(PROJECT), project_allowed),
            ] {
                assert_eq!(
                    billable_rate_access(&selection, action, &actor(), &resource(owner), &links),
                    expected
                );
            }
        }
    }
    // Even identical UUID values in another namespace are not the right owner.
    let links = ManagementAssignments {
        people: &[PROJECT],
        projects: &[PERSON],
        ..assignments()
    };
    for owner in [
        BillableRateOwner::Person(PERSON),
        BillableRateOwner::Project(PROJECT),
    ] {
        assert!(!billable_rate_access(
            &selection,
            RateAction::Read,
            &actor(),
            &resource(owner),
            &links
        ));
    }
}

#[test]
fn unrelated_catalog_grants_never_authorize_ordinary_financial_fields() {
    use Permission::*;
    let unrelated: Vec<_> = Permission::ALL
        .iter()
        .copied()
        .filter(|grant| {
            !matches!(
                grant,
                BillableRateReadManaged
                    | BillableRateWriteManaged
                    | BillableRateReadAll
                    | BillableRateWriteAll
                    | CostRateReadAll
                    | CostRateWriteAll
            )
        })
        .collect();
    for grants in unrelated
        .iter()
        .map(std::slice::from_ref)
        .chain(std::iter::once(unrelated.as_slice()))
    {
        let selection = PermissionSelection::new(grants);
        for action in [RateAction::Read, RateAction::Write] {
            for owner in [
                BillableRateOwner::Person(ACTOR),
                BillableRateOwner::Person(PERSON),
                BillableRateOwner::Project(PROJECT),
                BillableRateOwner::GlobalTask,
            ] {
                assert!(
                    !billable_rate_access(
                        &selection,
                        action,
                        &actor(),
                        &resource(owner),
                        &assignments()
                    ),
                    "{grants:?}"
                );
            }
            assert!(
                !cost_rate_access(&selection, action, &actor(), ORG),
                "{grants:?}"
            );
        }
    }
}

#[test]
fn cost_and_billable_grants_are_independent_and_read_does_not_allow_write() {
    use Permission::*;
    for (grant, read, write) in [
        (CostRateReadAll, true, false),
        (CostRateWriteAll, true, true),
        (BillableRateReadManaged, false, false),
        (BillableRateWriteManaged, false, false),
        (BillableRateReadAll, false, false),
        (BillableRateWriteAll, false, false),
    ] {
        let selection = PermissionSelection::new(&[grant]);
        assert_eq!(
            cost_rate_access(&selection, RateAction::Read, &actor(), ORG),
            read
        );
        assert_eq!(
            cost_rate_access(&selection, RateAction::Write, &actor(), ORG),
            write
        );
        if read {
            for action in [RateAction::Read, RateAction::Write] {
                assert!(!billable_rate_access(
                    &selection,
                    action,
                    &actor(),
                    &resource(BillableRateOwner::Person(PERSON)),
                    &assignments()
                ));
            }
        }
    }
}

#[test]
fn all_six_defaults_have_the_approved_financial_access() {
    use BuiltInProfile::*;
    for (profile, read, write) in [
        (Member, false, false),
        (ProjectManager, false, false),
        (PeopleAdmin, false, false),
        (Accounting, true, false),
        (ExecutiveManager, true, false),
        (Administrator, true, true),
    ] {
        let selection = profile.selection();
        for (action, expected) in [(RateAction::Read, read), (RateAction::Write, write)] {
            assert_eq!(
                cost_rate_access(&selection, action, &actor(), ORG),
                expected,
                "{profile:?} {action:?}"
            );
            for owner in [
                BillableRateOwner::Person(OTHER),
                BillableRateOwner::Project(OTHER),
                BillableRateOwner::GlobalTask,
            ] {
                assert_eq!(
                    billable_rate_access(
                        &selection,
                        action,
                        &actor(),
                        &resource(owner),
                        &assignments()
                    ),
                    expected,
                    "{profile:?} {action:?} {owner:?}"
                );
            }
        }
    }
}

#[test]
fn identity_and_tenant_mismatches_deny_even_all_financial_grants() {
    let selection = BuiltInProfile::Administrator.selection();
    for action in [RateAction::Read, RateAction::Write] {
        for (current_actor, org_id, links) in [
            (
                Actor {
                    active: false,
                    ..actor()
                },
                ORG,
                assignments(),
            ),
            (actor(), OTHER, assignments()),
            (
                actor(),
                ORG,
                ManagementAssignments {
                    actor_id: OTHER,
                    ..assignments()
                },
            ),
            (
                actor(),
                ORG,
                ManagementAssignments {
                    org_id: OTHER,
                    ..assignments()
                },
            ),
        ] {
            for owner in [
                BillableRateOwner::Person(PERSON),
                BillableRateOwner::Project(PROJECT),
                BillableRateOwner::GlobalTask,
            ] {
                assert!(!billable_rate_access(
                    &selection,
                    action,
                    &current_actor,
                    &BillableRateResource { org_id, owner },
                    &links
                ));
            }
        }
        assert!(!cost_rate_access(
            &selection,
            action,
            &Actor {
                active: false,
                ..actor()
            },
            ORG
        ));
        assert!(!cost_rate_access(&selection, action, &actor(), OTHER));
    }
}

#[test]
fn removing_read_prerequisites_denies_both_actions() {
    let mut selection = PermissionSelection::new(&[
        Permission::BillableRateWriteAll,
        Permission::CostRateWriteAll,
    ]);
    selection
        .remove(Permission::BillableRateReadManaged)
        .unwrap();
    selection.remove(Permission::CostRateReadAll).unwrap();
    for action in [RateAction::Read, RateAction::Write] {
        assert!(!cost_rate_access(&selection, action, &actor(), ORG));
        for owner in [
            BillableRateOwner::Person(PERSON),
            BillableRateOwner::Project(PROJECT),
            BillableRateOwner::GlobalTask,
        ] {
            assert!(!billable_rate_access(
                &selection,
                action,
                &actor(),
                &resource(owner),
                &assignments()
            ));
        }
    }
}

#[test]
fn grant_removal_denies_without_mutating_the_remaining_selection() {
    for (grant, cost) in [
        (Permission::BillableRateWriteManaged, false),
        (Permission::CostRateWriteAll, true),
    ] {
        let mut selection = PermissionSelection::new(&[grant]);
        selection.remove(grant).unwrap();
        let before = selection.clone();
        assert!(!billable_rate_access(
            &selection,
            RateAction::Write,
            &actor(),
            &resource(BillableRateOwner::Project(PROJECT)),
            &assignments()
        ));
        assert!(!cost_rate_access(
            &selection,
            RateAction::Write,
            &actor(),
            ORG
        ));
        assert_eq!(selection, before);
        if cost {
            assert!(cost_rate_access(
                &selection,
                RateAction::Read,
                &actor(),
                ORG
            ));
        } else {
            assert!(billable_rate_access(
                &selection,
                RateAction::Read,
                &actor(),
                &resource(BillableRateOwner::Project(PROJECT)),
                &assignments()
            ));
        }
    }
}
