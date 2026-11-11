use super::*;
use crate::models::user::{PersonRate, PersonRates};
use crate::server_fns::test_seed::seed;
use crate::server_fns::users::rates::{read_person_rates, write_person_rates};
use horae_core::permissions::catalog::{
    BuiltInProfile, PERMISSION_CATALOG_VERSION, Permission, PermissionSelection,
};
use horae_core::permissions::rates::RateEdit;
use sqlx::PgPool;
use uuid::Uuid;

fn code(error: ServerFnError) -> u16 {
    match error {
        ServerFnError::ServerError { code, .. } => code,
        other => panic!("unexpected error {other:?}"),
    }
}

async fn person(pool: &PgPool, org: Uuid, role: OrgRole) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name, org_role, billable_rate_cents, cost_rate_cents)
         VALUES ($1, $2, $3, 'Person', $4, 12000, 5000)",
        id,
        org,
        format!("{id}@test.com"),
        role as OrgRole,
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn currency(pool: &PgPool, org: Uuid) -> String {
    sqlx::query_scalar!(
        "SELECT default_currency FROM organizations WHERE id=$1",
        org
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

/// Detailed permissions on, the actor holding exactly `grants` (closed over
/// their dependencies) and managing `managed`.
async fn policy_one(
    pool: &PgPool,
    org: Uuid,
    actor: Uuid,
    grants: &[Permission],
    managed: &[Uuid],
) {
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version = 1 WHERE id = $1",
        org
    )
    .execute(pool)
    .await
    .unwrap();
    let values: Vec<String> =
        serde_json::from_value(serde_json::to_value(PermissionSelection::new(grants)).unwrap())
            .unwrap();
    sqlx::query!(
        "DELETE FROM person_permission_states WHERE user_id=$1",
        actor
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO person_permission_states
           (id, org_id, user_id, catalog_version, grants, is_administrator, source)
         VALUES ($1, $2, $3, $4, $5, false, 'individual')",
        Uuid::now_v7(),
        org,
        actor,
        i32::try_from(PERMISSION_CATALOG_VERSION).unwrap(),
        &values,
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "DELETE FROM person_management_assignments WHERE manager_id=$1",
        actor
    )
    .execute(pool)
    .await
    .unwrap();
    for target in managed {
        sqlx::query!(
            "INSERT INTO person_management_assignments (id,org_id,manager_id,managed_user_id)
             VALUES ($1,$2,$3,$4)",
            Uuid::now_v7(),
            org,
            actor,
            target
        )
        .execute(pool)
        .await
        .unwrap();
    }
}

/// (billable read, billable write, cost read, cost write), or `None` when the
/// read is refused.
type Access = Option<(bool, bool, bool, bool)>;

fn access(rates: &PersonRates) -> (bool, bool, bool, bool) {
    let field = |rate: Option<PersonRate>| (rate.is_some(), rate.is_some_and(|r| r.editable));
    let (billable_read, billable_write) = field(rates.billable);
    let (cost_read, cost_write) = field(rates.cost);
    (billable_read, billable_write, cost_read, cost_write)
}

async fn observe(pool: &PgPool, org: Uuid, actor: Uuid, target: Uuid) -> Access {
    match read_person_rates(pool, org, actor, target).await {
        Ok(rates) => Some(access(&rates)),
        Err(error) => {
            assert_eq!(code(error), FORBIDDEN);
            None
        }
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn policy_zero_follows_the_legacy_role_rule(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let target = person(&pool, ids.org_id, OrgRole::Member).await;
    let manager = person(&pool, ids.org_id, OrgRole::Manager).await;
    let member = person(&pool, ids.org_id, OrgRole::Member).await;

    assert_eq!(
        observe(&pool, ids.org_id, ids.user_id, target).await,
        Some((true, true, true, true))
    );
    assert_eq!(
        observe(&pool, ids.org_id, manager, target).await,
        Some((true, true, false, false))
    );
    assert_eq!(observe(&pool, ids.org_id, member, target).await, None);
    // Being the person does not reveal one's own rates.
    assert_eq!(observe(&pool, ids.org_id, member, member).await, None);
    assert_eq!(
        observe(&pool, ids.org_id, manager, manager).await,
        Some((true, true, false, false))
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn policy_zero_manager_cannot_write_or_see_a_cost_rate(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let target = person(&pool, ids.org_id, OrgRole::Member).await;
    let manager = person(&pool, ids.org_id, OrgRole::Manager).await;
    let eur = currency(&pool, ids.org_id).await;

    let refused = write_person_rates(
        &pool,
        ids.org_id,
        manager,
        target,
        &eur,
        RateEdit::Unchanged,
        RateEdit::Set(1),
    )
    .await
    .unwrap_err();
    assert_eq!(code(refused), FORBIDDEN);

    let saved = write_person_rates(
        &pool,
        ids.org_id,
        manager,
        target,
        &eur,
        RateEdit::Set(9900),
        RateEdit::Unchanged,
    )
    .await
    .unwrap();
    assert_eq!(saved.billable.unwrap().cents, Some(9900));
    assert_eq!(saved.cost, None);
    let stored = sqlx::query!(
        "SELECT billable_rate_cents, cost_rate_cents FROM users WHERE id=$1",
        target
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (stored.billable_rate_cents, stored.cost_rate_cents),
        (Some(9900), Some(5000))
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn policy_one_uses_each_rate_grant_and_its_managed_scope(pool: PgPool) {
    use Permission::*;
    let ids = seed(&pool, OrgRole::Admin).await;
    let managed = person(&pool, ids.org_id, OrgRole::Member).await;
    let other = person(&pool, ids.org_id, OrgRole::Member).await;
    // The legacy role must not matter once detailed permissions are on.
    let actor = person(&pool, ids.org_id, OrgRole::Admin).await;

    let cases: [(&[Permission], Access, Access); 8] = [
        (&[PeopleReadAll], None, None),
        (
            &[PeopleReadAll, BillableRateReadManaged],
            Some((true, false, false, false)),
            None,
        ),
        (
            &[PeopleReadAll, BillableRateWriteManaged],
            Some((true, true, false, false)),
            None,
        ),
        (
            &[PeopleReadAll, BillableRateReadAll],
            Some((true, false, false, false)),
            Some((true, false, false, false)),
        ),
        (
            &[PeopleReadAll, BillableRateWriteAll],
            Some((true, true, false, false)),
            Some((true, true, false, false)),
        ),
        (
            &[PeopleReadAll, CostRateReadAll],
            Some((false, false, true, false)),
            Some((false, false, true, false)),
        ),
        (
            &[PeopleReadAll, CostRateWriteAll],
            Some((false, false, true, true)),
            Some((false, false, true, true)),
        ),
        (
            &[
                PeopleReadAll,
                BillableRateReadAll,
                BillableRateWriteManaged,
                CostRateReadAll,
            ],
            Some((true, true, true, false)),
            Some((true, false, true, false)),
        ),
    ];
    for (grants, on_managed, on_other) in cases {
        policy_one(&pool, ids.org_id, actor, grants, &[managed]).await;
        assert_eq!(
            observe(&pool, ids.org_id, actor, managed).await,
            on_managed,
            "{grants:?} managed"
        );
        assert_eq!(
            observe(&pool, ids.org_id, actor, other).await,
            on_other,
            "{grants:?} other"
        );
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn policy_one_rate_grants_need_people_access_and_never_cover_oneself(pool: PgPool) {
    use Permission::*;
    let ids = seed(&pool, OrgRole::Admin).await;
    let managed = person(&pool, ids.org_id, OrgRole::Member).await;
    let actor = person(&pool, ids.org_id, OrgRole::Manager).await;

    // Accounting reads both rates org-wide but holds no People grant, so it
    // sees no one's rates: rates need people access.
    let accounting = BuiltInProfile::Accounting.direct_permissions();
    assert!(accounting.contains(&BillableRateReadAll) && accounting.contains(&CostRateReadAll));
    assert!(!accounting.contains(&PeopleReadAll) && !accounting.contains(&PeopleReadManaged));
    policy_one(&pool, ids.org_id, actor, accounting, &[managed]).await;
    assert_eq!(observe(&pool, ids.org_id, actor, managed).await, None);

    policy_one(
        &pool,
        ids.org_id,
        actor,
        &[PeopleReadManaged, BillableRateWriteManaged],
        &[managed],
    )
    .await;
    assert_eq!(
        observe(&pool, ids.org_id, actor, managed).await,
        Some((true, true, false, false))
    );
    assert_eq!(observe(&pool, ids.org_id, actor, actor).await, None);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn policy_one_refuses_writes_beyond_the_grant(pool: PgPool) {
    use Permission::*;
    let ids = seed(&pool, OrgRole::Admin).await;
    let managed = person(&pool, ids.org_id, OrgRole::Member).await;
    let other = person(&pool, ids.org_id, OrgRole::Member).await;
    let actor = person(&pool, ids.org_id, OrgRole::Member).await;
    let eur = currency(&pool, ids.org_id).await;
    policy_one(
        &pool,
        ids.org_id,
        actor,
        &[
            PeopleReadAll,
            BillableRateReadAll,
            BillableRateWriteManaged,
            CostRateReadAll,
        ],
        &[managed],
    )
    .await;

    for (target, billable, cost) in [
        (other, RateEdit::Set(100), RateEdit::Unchanged),
        (other, RateEdit::Reset, RateEdit::Unchanged),
        (managed, RateEdit::Unchanged, RateEdit::Set(100)),
        (managed, RateEdit::Unchanged, RateEdit::Reset),
    ] {
        let refused = write_person_rates(&pool, ids.org_id, actor, target, &eur, billable, cost)
            .await
            .unwrap_err();
        assert_eq!(code(refused), FORBIDDEN, "{billable:?} {cost:?}");
    }
    let saved = write_person_rates(
        &pool,
        ids.org_id,
        actor,
        managed,
        &eur,
        RateEdit::Reset,
        RateEdit::Unchanged,
    )
    .await
    .unwrap();
    assert_eq!(
        saved.billable,
        Some(PersonRate {
            cents: None,
            editable: true
        })
    );
    assert_eq!(
        saved.cost,
        Some(PersonRate {
            cents: Some(5000),
            editable: false
        })
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn rates_round_trip_exact_minor_units(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let target = person(&pool, ids.org_id, OrgRole::Member).await;
    let eur = currency(&pool, ids.org_id).await;
    let write = |billable, cost| {
        write_person_rates(&pool, ids.org_id, ids.user_id, target, &eur, billable, cost)
    };

    let saved = write(RateEdit::Set(9550), RateEdit::Set(4001))
        .await
        .unwrap();
    assert_eq!(saved.currency, eur);
    assert_eq!(saved.billable.unwrap().cents, Some(9550));
    assert_eq!(saved.cost.unwrap().cents, Some(4001));

    let kept = write(RateEdit::Unchanged, RateEdit::Set(0)).await.unwrap();
    assert_eq!(kept.billable.unwrap().cents, Some(9550));
    assert_eq!(kept.cost.unwrap().cents, Some(0));

    write(RateEdit::Reset, RateEdit::Reset).await.unwrap();
    let read = read_person_rates(&pool, ids.org_id, ids.user_id, target)
        .await
        .unwrap();
    assert_eq!(read.billable.unwrap().cents, None);
    assert_eq!(read.cost.unwrap().cents, None);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn writes_refuse_negative_amounts_and_a_stale_currency(pool: PgPool) {
    let ids = seed(&pool, OrgRole::Admin).await;
    let target = person(&pool, ids.org_id, OrgRole::Member).await;
    let eur = currency(&pool, ids.org_id).await;

    let negative = write_person_rates(
        &pool,
        ids.org_id,
        ids.user_id,
        target,
        &eur,
        RateEdit::Set(-1),
        RateEdit::Unchanged,
    )
    .await
    .unwrap_err();
    assert_eq!(code(negative), BAD_REQUEST);

    let stale = write_person_rates(
        &pool,
        ids.org_id,
        ids.user_id,
        target,
        "XXX",
        RateEdit::Set(100),
        RateEdit::Unchanged,
    )
    .await
    .unwrap_err();
    assert_eq!(code(stale), CONFLICT);
    let billable = sqlx::query_scalar!("SELECT billable_rate_cents FROM users WHERE id=$1", target)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(billable, Some(12000));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn another_organizations_person_is_not_found(pool: PgPool) {
    let ours = seed(&pool, OrgRole::Admin).await;
    let theirs = seed(&pool, OrgRole::Admin).await;
    let target = person(&pool, theirs.org_id, OrgRole::Member).await;
    let eur = currency(&pool, ours.org_id).await;

    let read = read_person_rates(&pool, ours.org_id, ours.user_id, target)
        .await
        .unwrap_err();
    assert_eq!(code(read), NOT_FOUND);
    let write = write_person_rates(
        &pool,
        ours.org_id,
        ours.user_id,
        target,
        &eur,
        RateEdit::Set(1),
        RateEdit::Set(1),
    )
    .await
    .unwrap_err();
    assert_eq!(code(write), NOT_FOUND);
    let stored = sqlx::query!(
        "SELECT billable_rate_cents, cost_rate_cents FROM users WHERE id=$1",
        target
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (stored.billable_rate_cents, stored.cost_rate_cents),
        (Some(12000), Some(5000))
    );
}
