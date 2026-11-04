//! Whole-domain properties of the permission decisions.
//!
//! Record scope is enumerated exhaustively over a small world: two
//! organizations, four people (the first is the actor), three projects and
//! every management-assignment subset. Grant sets range over 2^50 selections,
//! so properties that take a selection sample them with proptest instead.

use std::collections::BTreeSet;

use proptest::prelude::*;
use uuid::Uuid;

use super::{
    AccessScope, Actor, ManagementAssignments, ScopedResource,
    approvals::approval_records_covered,
    catalog::{PERMISSION_CATALOG_VERSION, Permission, PermissionSelection},
    person_management::has_person_management_grant,
    rates::{
        BillableRateOwner, BillableRateResource, RateAction, billable_rate_access, cost_rate_access,
    },
};

const ORGS: [Uuid; 2] = [Uuid::from_u128(0xA0), Uuid::from_u128(0xB0)];
const PEOPLE: [Uuid; 4] = [
    Uuid::from_u128(1),
    Uuid::from_u128(2),
    Uuid::from_u128(3),
    Uuid::from_u128(4),
];
const PROJECTS: [Uuid; 3] = [
    Uuid::from_u128(101),
    Uuid::from_u128(102),
    Uuid::from_u128(103),
];
const FLAGS: [AccessScope; 4] = [
    AccessScope::OWN,
    AccessScope::MANAGED_PEOPLE,
    AccessScope::MANAGED_PROJECTS,
    AccessScope::ORGANIZATION,
];
const FLOOR: [Permission; 4] = [
    Permission::TimeReadOwn,
    Permission::TimeWriteOwn,
    Permission::ExpenseReadOwn,
    Permission::ExpenseWriteOwn,
];

fn subsets<T: Copy>(items: &[T]) -> Vec<Vec<T>> {
    (0..1u32 << items.len())
        .map(|mask| {
            items
                .iter()
                .enumerate()
                .filter(|(i, _)| mask >> i & 1 == 1)
                .map(|(_, &item)| item)
                .collect()
        })
        .collect()
}

fn scopes() -> Vec<AccessScope> {
    subsets(&FLAGS)
        .into_iter()
        .map(|flags| {
            flags
                .into_iter()
                .fold(AccessScope::NONE, AccessScope::union)
        })
        .collect()
}

fn resources() -> Vec<ScopedResource> {
    let people = std::iter::once(None).chain(PEOPLE.map(Some));
    people
        .flat_map(|person_id| {
            std::iter::once(None)
                .chain(PROJECTS.map(Some))
                .flat_map(move |project_id| {
                    ORGS.map(|org_id| ScopedResource {
                        org_id,
                        person_id,
                        project_id,
                    })
                })
        })
        .collect()
}

/// Identity and tenancy every decision requires, whatever the grants.
fn provenance_ok(actor: &Actor, assignments: &ManagementAssignments<'_>) -> bool {
    actor.active && actor.id == assignments.actor_id && actor.org_id == assignments.org_id
}

/// The record is the actor's own, or tied to one of their management assignments.
fn related(
    actor: &Actor,
    record: &ScopedResource,
    assignments: &ManagementAssignments<'_>,
) -> bool {
    record.person_id == Some(actor.id) || managed(record, assignments)
}

fn managed(record: &ScopedResource, assignments: &ManagementAssignments<'_>) -> bool {
    record
        .person_id
        .is_some_and(|id| assignments.people.contains(&id))
        || record
            .project_id
            .is_some_and(|id| assignments.projects.contains(&id))
}

/// Coverage never leaves the actor's organization or reaches a record that is
/// neither theirs nor managed by them, unless ORGANIZATION was granted. (FR-006)
/// Catches: a scope that ignores tenancy/activity, a missing dimension treated
/// as a match, or managed-project scope leaking to a person-only record.
#[test]
fn scope_coverage_stays_within_granted_dimensions() {
    let actors: Vec<Actor> = PEOPLE[..2]
        .iter()
        .flat_map(|&id| {
            ORGS.into_iter()
                .flat_map(move |org_id| [true, false].map(|active| Actor { id, org_id, active }))
        })
        .collect();
    let (people, projects) = (subsets(&PEOPLE), subsets(&PROJECTS));
    let (scopes, resources) = (scopes(), resources());
    for actor in &actors {
        for (actor_id, org_id) in PEOPLE[..2].iter().flat_map(|&a| ORGS.map(|o| (a, o))) {
            for people in &people {
                for projects in &projects {
                    let assignments = ManagementAssignments {
                        actor_id,
                        org_id,
                        people,
                        projects,
                    };
                    for scope in &scopes {
                        for record in &resources {
                            if !scope.covers(actor, record, &assignments) {
                                continue;
                            }
                            assert!(provenance_ok(actor, &assignments));
                            assert_eq!(record.org_id, actor.org_id);
                            let org = scope.includes(AccessScope::ORGANIZATION);
                            assert!(org || related(actor, record, &assignments));
                            if !org && !scope.includes(AccessScope::OWN) {
                                assert!(managed(record, &assignments));
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Widening a scope or adding an assignment never removes coverage, so
/// revoking either never adds it. (FR-006, FR-010)
/// Catches: exclusive-or style scope logic, or an assignment that shadows
/// another, either of which would let a revocation widen access.
#[test]
fn scope_coverage_is_monotone_in_scope_and_assignments() {
    let actor = Actor {
        id: PEOPLE[0],
        org_id: ORGS[0],
        active: true,
    };
    let at = |people: &[Uuid], projects: &[Uuid], scope: AccessScope, record: &ScopedResource| {
        let assignments = ManagementAssignments {
            actor_id: actor.id,
            org_id: actor.org_id,
            people,
            projects,
        };
        scope.covers(&actor, record, &assignments)
    };
    let (people, projects) = (subsets(&PEOPLE), subsets(&PROJECTS));
    let (scopes, resources) = (scopes(), resources());
    for people in &people {
        for projects in &projects {
            for &scope in &scopes {
                for record in &resources {
                    if !at(people, projects, scope, record) {
                        continue;
                    }
                    for flag in FLAGS {
                        assert!(at(people, projects, scope.union(flag), record));
                    }
                    for &extra in &PEOPLE {
                        let wider: Vec<_> = people.iter().copied().chain([extra]).collect();
                        assert!(at(&wider, projects, scope, record));
                    }
                    for &extra in &PROJECTS {
                        let wider: Vec<_> = projects.iter().copied().chain([extra]).collect();
                        assert!(at(people, &wider, scope, record));
                    }
                }
            }
        }
    }
}

/// Independent transitive closure over the declared prerequisite edges.
fn closure(grants: impl IntoIterator<Item = Permission>) -> BTreeSet<Permission> {
    let mut done = BTreeSet::new();
    let mut todo: Vec<_> = grants.into_iter().collect();
    while let Some(grant) = todo.pop() {
        if done.insert(grant) {
            todo.extend_from_slice(grant.prerequisites());
        }
    }
    done
}

fn set(selection: &PermissionSelection) -> BTreeSet<Permission> {
    selection.iter().collect()
}

fn grants() -> impl Strategy<Value = Vec<Permission>> {
    prop::collection::vec(prop::sample::select(Permission::ALL), 0..12)
}

/// Grants drawn only from `allowed`, which must be closed under prerequisites.
fn grants_from(allowed: Vec<Permission>) -> impl Strategy<Value = Vec<Permission>> {
    prop::collection::vec(prop::sample::select(allowed), 0..12)
}

fn without(excluded: fn(Permission) -> bool) -> Vec<Permission> {
    Permission::ALL
        .iter()
        .copied()
        .filter(|&p| !excluded(p))
        .collect()
}

/// Mostly the actor's own valid context, sometimes inactive or mismatched.
#[derive(Debug, Clone)]
struct World {
    actor: Actor,
    assignment_actor: Uuid,
    assignment_org: Uuid,
    people: Vec<Uuid>,
    projects: Vec<Uuid>,
}

impl World {
    fn assignments(&self) -> ManagementAssignments<'_> {
        ManagementAssignments {
            actor_id: self.assignment_actor,
            org_id: self.assignment_org,
            people: &self.people,
            projects: &self.projects,
        }
    }
}

fn world() -> impl Strategy<Value = World> {
    let valid = (
        prop::sample::subsequence(PEOPLE.to_vec(), 0..=4),
        prop::sample::subsequence(PROJECTS.to_vec(), 0..=3),
    )
        .prop_map(|(people, projects)| World {
            actor: Actor {
                id: PEOPLE[0],
                org_id: ORGS[0],
                active: true,
            },
            assignment_actor: PEOPLE[0],
            assignment_org: ORGS[0],
            people,
            projects,
        });
    let any = (
        prop::sample::select(&PEOPLE[..2]),
        prop::sample::select(&ORGS[..]),
        any::<bool>(),
        prop::sample::select(&PEOPLE[..2]),
        prop::sample::select(&ORGS[..]),
        prop::sample::subsequence(PEOPLE.to_vec(), 0..=4),
        prop::sample::subsequence(PROJECTS.to_vec(), 0..=3),
    )
        .prop_map(
            |(id, org_id, active, assignment_actor, assignment_org, people, projects)| World {
                actor: Actor { id, org_id, active },
                assignment_actor,
                assignment_org,
                people,
                projects,
            },
        );
    prop_oneof![4 => valid, 1 => any]
}

fn record() -> impl Strategy<Value = ScopedResource> {
    (
        prop_oneof![9 => Just(ORGS[0]), 1 => Just(ORGS[1])],
        prop::option::of(prop::sample::select(&PEOPLE[..])),
        prop::option::of(prop::sample::select(&PROJECTS[..])),
    )
        .prop_map(|(org_id, person_id, project_id)| ScopedResource {
            org_id,
            person_id,
            project_id,
        })
}

fn records() -> impl Strategy<Value = Vec<ScopedResource>> {
    prop::collection::vec(record(), 0..4)
}

fn owner() -> impl Strategy<Value = BillableRateOwner> {
    prop_oneof![
        prop::sample::select(&PEOPLE[..]).prop_map(BillableRateOwner::Person),
        prop::sample::select(&PROJECTS[..]).prop_map(BillableRateOwner::Project),
        Just(BillableRateOwner::GlobalTask),
    ]
}

fn action() -> impl Strategy<Value = RateAction> {
    prop_oneof![Just(RateAction::Read), Just(RateAction::Write)]
}

/// Whether the selection lets the actor read the record in one domain,
/// stated from the grant names rather than from `AccessScope`.
fn readable(
    selection: &PermissionSelection,
    [own, managed_grant, all]: [Permission; 3],
    world: &World,
    record: &ScopedResource,
) -> bool {
    selection.contains(all)
        || (selection.contains(own) && record.person_id == Some(world.actor.id))
        || (selection.contains(managed_grant) && managed(record, &world.assignments()))
}

const TIME_READ: [Permission; 3] = [
    Permission::TimeReadOwn,
    Permission::TimeReadManaged,
    Permission::TimeReadAll,
];
const EXPENSE_READ: [Permission; 3] = [
    Permission::ExpenseReadOwn,
    Permission::ExpenseReadManaged,
    Permission::ExpenseReadAll,
];

fn is_billable_rate(grant: Permission) -> bool {
    matches!(
        grant,
        Permission::BillableRateReadManaged
            | Permission::BillableRateWriteManaged
            | Permission::BillableRateReadAll
            | Permission::BillableRateWriteAll
    )
}

fn is_cost_rate(grant: Permission) -> bool {
    matches!(
        grant,
        Permission::CostRateReadAll | Permission::CostRateWriteAll
    )
}

fn is_time_approval(grant: Permission) -> bool {
    matches!(
        grant,
        Permission::TimeApproveManaged | Permission::TimeApproveAll
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2048))]

    /// A selection is exactly the requested grants, the Member floor and their
    /// transitive prerequisites, nothing more. (FR-015, US4-6)
    /// Catches: an editor normalization that silently adds an unrequested
    /// grant (privilege escalation) or drops a prerequisite.
    #[test]
    fn selection_is_the_least_closed_superset_of_the_request(requested in grants()) {
        let selection = PermissionSelection::new(&requested);
        prop_assert_eq!(set(&selection), closure(FLOOR.into_iter().chain(requested)));
    }

    /// Removing a grant removes exactly the grants that transitively require
    /// it; the floor is immutable. (FR-015, US4-6)
    /// Catches: removal leaving a dependant without its prerequisite (e.g.
    /// write-all without read-all) or silently downgrading unrelated grants.
    #[test]
    fn removal_takes_away_exactly_the_dependants(
        requested in grants(),
        removed in prop::sample::select(Permission::ALL),
    ) {
        let before = PermissionSelection::new(&requested);
        let mut after = before.clone();
        if FLOOR.contains(&removed) {
            prop_assert!(after.remove(removed).is_err());
            prop_assert_eq!(after, before);
        } else {
            prop_assert!(after.remove(removed).is_ok());
            let expected: BTreeSet<_> = before
                .iter()
                .filter(|&grant| !closure([grant]).contains(&removed))
                .collect();
            prop_assert_eq!(set(&after), expected);
        }
    }

    /// Saved grants are restored verbatim or rejected, and every catalog
    /// version other than the current one is rejected. (FR-015)
    /// Catches: read-time normalization widening stored access, or a stale or
    /// newer catalog being interpreted under the current grant meanings.
    #[test]
    fn stored_grants_are_restored_verbatim_or_rejected(
        stored in prop::collection::vec(prop::sample::select(Permission::ALL), 0..30),
        version in prop_oneof![3 => Just(PERMISSION_CATALOG_VERSION), 1 => any::<u32>()],
    ) {
        let grants: BTreeSet<_> = stored.iter().copied().collect();
        let valid = version == PERMISSION_CATALOG_VERSION
            && grants.len() == stored.len()
            && closure(FLOOR.into_iter().chain(stored.iter().copied())) == grants;
        match PermissionSelection::from_stored(version, &stored) {
            Ok(selection) => {
                prop_assert!(valid);
                prop_assert_eq!(set(&selection), grants);
            }
            Err(_) => prop_assert!(!valid),
        }
    }

    /// Every editor-built selection survives storage unchanged. (FR-015)
    /// Catches: a closure rule the reader rejects, locking out saved people.
    #[test]
    fn canonical_selections_round_trip_through_storage(requested in grants()) {
        let selection = PermissionSelection::new(&requested);
        let stored: Vec<_> = selection.iter().collect();
        prop_assert_eq!(
            PermissionSelection::from_stored(PERMISSION_CATALOG_VERSION, &stored),
            Ok(selection)
        );
    }

    /// An approval covers only records the actor may both approve and read in
    /// their domain, and never another organization's. (FR-009, FR-024)
    /// Catches: approving hidden expenses, approving own work through
    /// managed-only approval, or approval authority substituting for reads.
    #[test]
    fn approval_reaches_only_approvable_and_readable_records(
        requested in grants(),
        world in world(),
        time in records(),
        expenses in records(),
    ) {
        let selection = PermissionSelection::new(&requested);
        let assignments = world.assignments();
        if approval_records_covered(&selection, &world.actor, &assignments, &time, &expenses) {
            prop_assert!(provenance_ok(&world.actor, &assignments));
            prop_assert!(selection.iter().any(is_time_approval));
            for (record, read) in time.iter().map(|r| (r, TIME_READ))
                .chain(expenses.iter().map(|r| (r, EXPENSE_READ)))
            {
                prop_assert_eq!(record.org_id, world.actor.org_id);
                prop_assert!(
                    selection.contains(Permission::TimeApproveAll) || managed(record, &assignments)
                );
                prop_assert!(readable(&selection, read, &world, record));
            }
        }
    }

    /// An approval set is covered exactly when the caller may approve at all
    /// and every record is covered on its own. (FR-024, US2-27)
    /// Catches: `any` instead of `all`, or a guard that approves the visible
    /// portion of a selection containing an inaccessible record.
    #[test]
    fn approval_is_all_or_nothing(
        requested in grants(),
        world in world(),
        time in records(),
        expenses in records(),
    ) {
        let selection = PermissionSelection::new(&requested);
        let assignments = world.assignments();
        let covered = |t: &[ScopedResource], e: &[ScopedResource]| {
            approval_records_covered(&selection, &world.actor, &assignments, t, e)
        };
        let each = covered(&[], &[])
            && time.iter().all(|r| covered(std::slice::from_ref(r), &[]))
            && expenses.iter().all(|r| covered(&[], std::slice::from_ref(r)));
        prop_assert_eq!(covered(&time, &expenses), each);
    }

    /// More grants or more assignments never deny an approval that fewer
    /// allowed. (FR-006, FR-010)
    /// Catches: a grant that narrows another (e.g. approve-all replacing own
    /// reads), which would make revocation widen access.
    #[test]
    fn approval_is_monotone_in_grants_and_assignments(
        requested in grants(),
        extra in grants(),
        world in world(),
        extra_person in prop::sample::select(&PEOPLE[..]),
        extra_project in prop::sample::select(&PROJECTS[..]),
        time in records(),
        expenses in records(),
    ) {
        let narrow = PermissionSelection::new(&requested);
        let assignments = world.assignments();
        if approval_records_covered(&narrow, &world.actor, &assignments, &time, &expenses) {
            let broad = PermissionSelection::new(&[requested, extra].concat());
            prop_assert!(approval_records_covered(&broad, &world.actor, &assignments, &time, &expenses));
            let people = [world.people.as_slice(), &[extra_person]].concat();
            let projects = [world.projects.as_slice(), &[extra_project]].concat();
            let wider = ManagementAssignments { people: &people, projects: &projects, ..assignments };
            prop_assert!(approval_records_covered(&narrow, &world.actor, &wider, &time, &expenses));
        }
    }

    /// Without an explicit time-approval grant nothing is approvable, not even
    /// an empty selection. (FR-024, contracts/approval-visibility.md)
    /// Catches: read, write, withdrawal or report grants implying approval.
    #[test]
    fn approval_requires_an_approval_grant(
        requested in grants_from(without(is_time_approval)),
        world in world(),
        time in records(),
        expenses in records(),
    ) {
        let selection = PermissionSelection::new(&requested);
        prop_assert!(!approval_records_covered(
            &selection, &world.actor, &world.assignments(), &time, &expenses
        ));
    }

    /// Billable-rate access follows the rate's owner: a person's rate through
    /// managing that person, a project's through managing that project, global
    /// task defaults only organization-wide. (FR-021)
    /// Catches: managing a project exposing a participant's general rate, or a
    /// managed grant reaching global task defaults.
    #[test]
    fn billable_rate_access_is_confined_to_the_owner(
        requested in grants(),
        world in world(),
        action in action(),
        owner in owner(),
        org_id in prop::sample::select(&ORGS[..]),
    ) {
        let selection = PermissionSelection::new(&requested);
        let assignments = world.assignments();
        let resource = BillableRateResource { org_id, owner };
        if billable_rate_access(&selection, action, &world.actor, &resource, &assignments) {
            prop_assert!(provenance_ok(&world.actor, &assignments));
            prop_assert_eq!(org_id, world.actor.org_id);
            let (managed_grant, all) = match action {
                RateAction::Read => (Permission::BillableRateReadManaged, Permission::BillableRateReadAll),
                RateAction::Write => (Permission::BillableRateWriteManaged, Permission::BillableRateWriteAll),
            };
            let by_owner = match owner {
                BillableRateOwner::Person(id) => world.people.contains(&id),
                BillableRateOwner::Project(id) => world.projects.contains(&id),
                BillableRateOwner::GlobalTask => false,
            };
            prop_assert!(selection.contains(all) || (selection.contains(managed_grant) && by_owner));
        }
    }

    /// A person's rate ignores project assignments and a project's rate ignores
    /// person assignments entirely. (FR-021, C03)
    /// Catches: any cross-dimension leak, in either direction.
    #[test]
    fn billable_rate_owner_ignores_the_other_dimension(
        requested in grants(),
        world in world(),
        action in action(),
        owner in owner(),
        other_people in prop::sample::subsequence(PEOPLE.to_vec(), 0..=4),
        other_projects in prop::sample::subsequence(PROJECTS.to_vec(), 0..=3),
    ) {
        let selection = PermissionSelection::new(&requested);
        let assignments = world.assignments();
        let resource = BillableRateResource { org_id: ORGS[0], owner };
        let swapped = match owner {
            BillableRateOwner::Person(_) => ManagementAssignments { projects: &other_projects, ..assignments },
            BillableRateOwner::Project(_) => ManagementAssignments { people: &other_people, ..assignments },
            BillableRateOwner::GlobalTask => ManagementAssignments { people: &other_people, projects: &other_projects, ..assignments },
        };
        prop_assert_eq!(
            billable_rate_access(&selection, action, &world.actor, &resource, &assignments),
            billable_rate_access(&selection, action, &world.actor, &resource, &swapped)
        );
    }

    /// Whoever may write a rate may also read it. (FR-021, FR-022)
    /// Catches: a prerequisite edit that leaves write-only rate authority,
    /// letting someone overwrite amounts they are not allowed to see.
    #[test]
    fn rate_write_implies_rate_read(
        requested in grants(),
        world in world(),
        owner in owner(),
        org_id in prop::sample::select(&ORGS[..]),
    ) {
        let selection = PermissionSelection::new(&requested);
        let assignments = world.assignments();
        let resource = BillableRateResource { org_id, owner };
        let billable = |action| billable_rate_access(&selection, action, &world.actor, &resource, &assignments);
        prop_assert!(!billable(RateAction::Write) || billable(RateAction::Read));
        let cost = |action| cost_rate_access(&selection, action, &world.actor, org_id);
        prop_assert!(!cost(RateAction::Write) || cost(RateAction::Read));
    }

    /// No billable-rate grant means no billable-rate access, however much
    /// management or other authority is held. (FR-021, US2-24)
    /// Catches: management, project editing or invoicing implying rate access.
    #[test]
    fn billable_rates_need_a_billable_rate_grant(
        requested in grants_from(without(is_billable_rate)),
        world in world(),
        action in action(),
        owner in owner(),
    ) {
        let selection = PermissionSelection::new(&requested);
        let resource = BillableRateResource { org_id: world.actor.org_id, owner };
        prop_assert!(!billable_rate_access(&selection, action, &world.actor, &resource, &world.assignments()));
    }

    /// No cost grant means no cost access, whatever else is held. (FR-022)
    /// Catches: billable-rate, management or profile shortcuts to cost rates.
    #[test]
    fn cost_rates_need_a_cost_rate_grant(
        requested in grants_from(without(is_cost_rate)),
        world in world(),
        action in action(),
    ) {
        let selection = PermissionSelection::new(&requested);
        prop_assert!(!cost_rate_access(&selection, action, &world.actor, world.actor.org_id));
    }

    /// Without a grant compatible with person management, managed-person
    /// assignments change no decision. (FR-028)
    /// Catches: the compatibility list missing a grant that core scopes by
    /// managed people, so a "dormant" assignment silently grants access.
    #[test]
    fn person_assignments_are_inert_without_a_compatible_grant(
        requested in grants_from(without(|grant| {
            has_person_management_grant(&PermissionSelection::new(&[grant]))
        })),
        world in world(),
        time in records(),
        expenses in records(),
        action in action(),
        owner in owner(),
    ) {
        let selection = PermissionSelection::new(&requested);
        prop_assert!(!has_person_management_grant(&selection));
        let assignments = world.assignments();
        let unmanaged = ManagementAssignments { people: &[], ..assignments };
        prop_assert_eq!(
            approval_records_covered(&selection, &world.actor, &assignments, &time, &expenses),
            approval_records_covered(&selection, &world.actor, &unmanaged, &time, &expenses)
        );
        let resource = BillableRateResource { org_id: world.actor.org_id, owner };
        prop_assert_eq!(
            billable_rate_access(&selection, action, &world.actor, &resource, &assignments),
            billable_rate_access(&selection, action, &world.actor, &resource, &unmanaged)
        );
    }
}
