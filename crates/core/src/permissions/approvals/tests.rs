use super::approval_records_covered;
use crate::permissions::{
    Actor, ManagementAssignments, ScopedResource,
    catalog::{Permission, PermissionSelection},
};
use uuid::Uuid;

const ORG: Uuid = Uuid::from_u128(1);
const ACTOR: Uuid = Uuid::from_u128(2);
const PERSON: Uuid = Uuid::from_u128(3);
const PROJECT: Uuid = Uuid::from_u128(4);
const OTHER: Uuid = Uuid::from_u128(5);

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

fn resource(person: Uuid, project: Uuid) -> ScopedResource {
    ScopedResource {
        org_id: ORG,
        person_id: Some(person),
        project_id: Some(project),
    }
}

fn covered(grants: &[Permission], time: &[ScopedResource], expenses: &[ScopedResource]) -> bool {
    approval_records_covered(
        &PermissionSelection::new(grants),
        &actor(),
        &assignments(),
        time,
        expenses,
    )
}

#[test]
fn one_unreadable_expense_denies_the_whole_supplied_set() {
    let time = [resource(PERSON, PROJECT)];
    let own = resource(ACTOR, PROJECT);
    let hidden = resource(PERSON, PROJECT);
    for expenses in [[own, hidden], [hidden, own]] {
        assert!(!covered(&[Permission::TimeApproveAll], &time, &expenses));
    }
    assert!(covered(
        &[Permission::TimeApproveAll, Permission::ExpenseReadManaged],
        &time,
        &[hidden, own]
    ));
}

#[test]
fn all_records_need_approval_scope_even_when_both_domains_are_readable() {
    let grants = [
        Permission::TimeApproveManaged,
        Permission::TimeReadAll,
        Permission::ExpenseReadAll,
    ];
    let managed = resource(PERSON, PROJECT);
    let outside = resource(OTHER, OTHER);
    assert!(covered(&grants, &[managed], &[managed]));
    assert!(!covered(&grants, &[managed, outside], &[managed]));
    assert!(!covered(&grants, &[managed], &[managed, outside]));
}

#[test]
fn expense_read_scope_and_approval_scope_form_an_intersection() {
    let records = [
        resource(ACTOR, OTHER),
        resource(ACTOR, PROJECT),
        resource(PERSON, OTHER),
        resource(OTHER, PROJECT),
        resource(PERSON, PROJECT),
        resource(OTHER, OTHER),
    ];
    // Independent expected coverage: own; own+managed; organization.
    let visibility = [
        (
            Permission::ExpenseReadOwn,
            [true, true, false, false, false, false],
        ),
        (
            Permission::ExpenseReadManaged,
            [true, true, true, true, true, false],
        ),
        (Permission::ExpenseReadAll, [true; 6]),
    ];
    let authority = [
        (
            Permission::TimeApproveManaged,
            [false, true, true, true, true, false],
        ),
        (Permission::TimeApproveAll, [true; 6]),
    ];
    for (approve, approved) in authority {
        for (read, readable) in visibility {
            for (index, record) in records.iter().enumerate() {
                assert_eq!(
                    covered(&[approve, read], &[], &[*record]),
                    approved[index] && readable[index],
                    "approval={approve:?}, read={read:?}, record={index}"
                );
            }
        }
    }
}

#[test]
fn catalog_grants_cannot_substitute_for_approval() {
    let row = [resource(PERSON, PROJECT)];
    for grant in Permission::ALL {
        let expected = matches!(
            grant,
            Permission::TimeApproveManaged | Permission::TimeApproveAll
        );
        assert_eq!(
            covered(
                &[*grant, Permission::TimeReadAll, Permission::ExpenseReadAll],
                &row,
                &row
            ),
            expected,
            "{grant:?}"
        );
    }
}

#[test]
fn only_expense_reads_or_their_write_prerequisites_make_others_expenses_visible() {
    let row = [resource(PERSON, PROJECT)];
    for grant in Permission::ALL {
        let expected = matches!(
            grant,
            Permission::ExpenseReadManaged
                | Permission::ExpenseReadAll
                | Permission::ExpenseWriteManaged
                | Permission::ExpenseWriteAll
        );
        assert_eq!(
            covered(&[Permission::TimeApproveAll, *grant], &row, &row),
            expected,
            "{grant:?}"
        );
    }
}

#[test]
fn no_expense_read_is_needed_when_there_are_no_expenses() {
    assert!(covered(
        &[Permission::TimeApproveManaged],
        &[resource(PERSON, OTHER)],
        &[]
    ));
}

#[test]
fn empty_record_sets_do_not_remove_identity_or_explicit_grant_checks() {
    assert!(!covered(&[], &[], &[]));
    assert!(covered(&[Permission::TimeApproveManaged], &[], &[]));
    let grants =
        PermissionSelection::new(&[Permission::TimeApproveAll, Permission::ExpenseReadAll]);
    let inactive = Actor {
        active: false,
        ..actor()
    };
    let foreign_org = ManagementAssignments {
        org_id: OTHER,
        ..assignments()
    };
    let foreign_actor = ManagementAssignments {
        actor_id: OTHER,
        ..assignments()
    };
    assert!(!approval_records_covered(
        &grants,
        &inactive,
        &assignments(),
        &[],
        &[]
    ));
    for scope in [foreign_org, foreign_actor] {
        assert!(!approval_records_covered(
            &grants,
            &actor(),
            &scope,
            &[],
            &[]
        ));
    }
}

#[test]
fn inactive_and_foreign_provenance_deny_even_with_all_grants() {
    let grants = PermissionSelection::new(Permission::ALL);
    let row = [resource(PERSON, PROJECT)];
    let foreign = [ScopedResource {
        org_id: OTHER,
        ..row[0]
    }];
    let inactive = Actor {
        active: false,
        ..actor()
    };
    assert!(!approval_records_covered(
        &grants,
        &inactive,
        &assignments(),
        &row,
        &row
    ));
    assert!(!approval_records_covered(
        &grants,
        &actor(),
        &assignments(),
        &foreign,
        &row
    ));
    assert!(!approval_records_covered(
        &grants,
        &actor(),
        &assignments(),
        &row,
        &foreign
    ));
    for scope in [
        ManagementAssignments {
            org_id: OTHER,
            ..assignments()
        },
        ManagementAssignments {
            actor_id: OTHER,
            ..assignments()
        },
    ] {
        assert!(!approval_records_covered(
            &grants,
            &actor(),
            &scope,
            &row,
            &row
        ));
    }
}

#[test]
fn removing_the_last_matching_relationship_revokes_both_domains() {
    let grants = PermissionSelection::new(&[
        Permission::TimeApproveManaged,
        Permission::ExpenseReadManaged,
    ]);
    let row = [resource(PERSON, PROJECT)];
    let people_only = ManagementAssignments {
        projects: &[],
        ..assignments()
    };
    let projects_only = ManagementAssignments {
        people: &[],
        ..assignments()
    };
    for scope in [assignments(), people_only, projects_only] {
        assert!(approval_records_covered(
            &grants,
            &actor(),
            &scope,
            &row,
            &row
        ));
    }
    let none = ManagementAssignments {
        people: &[],
        projects: &[],
        ..assignments()
    };
    assert!(!approval_records_covered(
        &grants,
        &actor(),
        &none,
        &row,
        &row
    ));
}

#[test]
fn reevaluation_does_not_reuse_prior_read_or_approval_success() {
    let row = [resource(PERSON, PROJECT)];
    assert!(covered(
        &[Permission::TimeApproveAll, Permission::ExpenseReadAll],
        &row,
        &row
    ));
    assert!(!covered(&[Permission::TimeApproveAll], &row, &row));
    assert!(!covered(
        &[Permission::TimeReadAll, Permission::ExpenseReadAll],
        &row,
        &row
    ));
}

#[test]
fn missing_dimensions_and_input_duplicates_do_not_infer_scope() {
    let grants = [
        Permission::TimeApproveManaged,
        Permission::ExpenseReadManaged,
    ];
    let no_person = ScopedResource {
        person_id: None,
        ..resource(PERSON, PROJECT)
    };
    let no_project = ScopedResource {
        project_id: None,
        ..resource(PERSON, PROJECT)
    };
    assert!(covered(
        &grants,
        &[no_person, no_project, no_person],
        &[no_project, no_project]
    ));
    let neither = ScopedResource {
        person_id: None,
        project_id: None,
        org_id: ORG,
    };
    assert!(!covered(&grants, &[neither], &[]));
    assert!(!covered(&grants, &[], &[neither]));
}
