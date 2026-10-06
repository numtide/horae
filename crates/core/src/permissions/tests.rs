use super::{AccessScope, Actor, ManagementAssignments, ScopedResource};
use uuid::Uuid;

const ORG: Uuid = Uuid::from_u128(1);
const ACTOR: Uuid = Uuid::from_u128(2);
const PERSON: Uuid = Uuid::from_u128(3);
const PROJECT: Uuid = Uuid::from_u128(4);
const OTHER: Uuid = Uuid::from_u128(5);
const PEOPLE: [Uuid; 1] = [PERSON];
const PROJECTS: [Uuid; 1] = [PROJECT];

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
        people: &PEOPLE,
        projects: &PROJECTS,
    }
}

fn resource(person_id: Option<Uuid>, project_id: Option<Uuid>) -> ScopedResource {
    ScopedResource {
        org_id: ORG,
        person_id,
        project_id,
    }
}

fn scopes() -> Vec<(AccessScope, [bool; 4])> {
    (0..16)
        .map(|bits| {
            let enabled = std::array::from_fn(|bit| bits & (1 << bit) != 0);
            let scope = [
                AccessScope::OWN,
                AccessScope::MANAGED_PEOPLE,
                AccessScope::MANAGED_PROJECTS,
                AccessScope::ORGANIZATION,
            ]
            .into_iter()
            .zip(enabled)
            .filter(|(_, enabled)| *enabled)
            .fold(AccessScope::NONE, |scope, (grant, _)| scope.union(grant));
            (scope, enabled)
        })
        .collect()
}

#[test]
fn every_scope_combination_matches_only_explicitly_granted_dimensions() {
    // Expected dimensions are independent of the implementation's flag encoding.
    let cases = [
        (Some(ACTOR), Some(OTHER), [true, false, false]),
        (Some(PERSON), Some(OTHER), [false, true, false]),
        (Some(OTHER), Some(PROJECT), [false, false, true]),
        (Some(PERSON), Some(PROJECT), [false, true, true]),
        (Some(ACTOR), Some(PROJECT), [true, false, true]),
        (Some(OTHER), Some(OTHER), [false, false, false]),
        (None, None, [false, false, false]),
        (None, Some(PROJECT), [false, false, true]),
        (Some(PERSON), None, [false, true, false]),
        (Some(ACTOR), None, [true, false, false]),
        (None, Some(OTHER), [false, false, false]),
        (Some(OTHER), None, [false, false, false]),
    ];
    for (scope, [own, people, projects, organization]) in scopes() {
        for (person_id, project_id, [is_own, is_person, is_project]) in cases {
            let expected = organization
                || (own && is_own)
                || (people && is_person)
                || (projects && is_project);
            assert_eq!(
                scope.covers(&actor(), &resource(person_id, project_id), &assignments()),
                expected,
                "scope={scope:?}, person={person_id:?}, project={project_id:?}",
            );
        }
    }
}

#[test]
fn inactive_or_foreign_context_is_denied_for_every_scope_combination() {
    let mut inactive = actor();
    inactive.active = false;
    let mut foreign_resource = resource(Some(ACTOR), Some(PROJECT));
    foreign_resource.org_id = OTHER;
    let mut foreign_assignments = assignments();
    foreign_assignments.org_id = OTHER;
    let mut another_actors_assignments = assignments();
    another_actors_assignments.actor_id = OTHER;
    for (scope, _) in scopes() {
        for target in [
            resource(Some(ACTOR), Some(PROJECT)),
            resource(Some(PERSON), Some(PROJECT)),
            resource(None, None),
        ] {
            assert!(!scope.covers(&inactive, &target, &assignments()));
            assert!(!scope.covers(&actor(), &target, &foreign_assignments));
            assert!(!scope.covers(&actor(), &target, &another_actors_assignments));
        }
        assert!(!scope.covers(&actor(), &foreign_resource, &assignments()));
    }
}

#[test]
fn assignments_do_not_grant_scope_and_missing_assignments_do_not_match() {
    let target = resource(Some(PERSON), Some(PROJECT));
    assert!(!AccessScope::NONE.covers(&actor(), &target, &assignments()));
    let empty = ManagementAssignments {
        people: &[],
        projects: &[],
        ..assignments()
    };
    let managed = AccessScope::MANAGED_PEOPLE.union(AccessScope::MANAGED_PROJECTS);
    assert!(!managed.covers(&actor(), &target, &empty));
    assert!(AccessScope::OWN.covers(&actor(), &resource(Some(ACTOR), None), &empty));
    assert!(AccessScope::ORGANIZATION.covers(&actor(), &resource(None, None), &empty));
}

#[test]
fn removing_an_assignment_removes_only_that_matching_dimension() {
    let target = resource(Some(PERSON), Some(PROJECT));
    let people_only = ManagementAssignments {
        projects: &[],
        ..assignments()
    };
    let projects_only = ManagementAssignments {
        people: &[],
        ..assignments()
    };
    let empty = ManagementAssignments {
        people: &[],
        projects: &[],
        ..assignments()
    };
    let combined = AccessScope::MANAGED_PEOPLE.union(AccessScope::MANAGED_PROJECTS);
    assert!(combined.covers(&actor(), &target, &assignments()));
    assert!(combined.covers(&actor(), &target, &people_only));
    assert!(combined.covers(&actor(), &target, &projects_only));
    assert!(!combined.covers(&actor(), &target, &empty));
    assert!(!AccessScope::MANAGED_PROJECTS.covers(&actor(), &target, &people_only));
    assert!(!AccessScope::MANAGED_PEOPLE.covers(&actor(), &target, &projects_only));
}

#[test]
fn assignment_matching_searches_all_members_and_is_insensitive_to_duplicates() {
    let people = [OTHER, PERSON, PERSON];
    let projects = [OTHER, PROJECT, PROJECT];
    let populated = ManagementAssignments {
        people: &people,
        projects: &projects,
        ..assignments()
    };
    let target = resource(Some(PERSON), Some(PROJECT));
    assert!(AccessScope::MANAGED_PEOPLE.covers(&actor(), &target, &populated));
    assert!(AccessScope::MANAGED_PROJECTS.covers(&actor(), &target, &populated));
}

#[test]
fn scope_union_obeys_set_laws() {
    for (a, _) in scopes() {
        assert_eq!(a.union(AccessScope::NONE), a);
        assert_eq!(AccessScope::NONE.union(a), a);
        assert_eq!(a.union(a), a);
        for (b, _) in scopes() {
            assert_eq!(a.union(b), b.union(a));
            for (c, _) in scopes() {
                assert_eq!(a.union(b).union(c), a.union(b.union(c)));
            }
        }
    }
}
