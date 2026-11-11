//! Branding and the workspace time zone, allowed and refused by stored grants.
//! Fixtures keep the organization on its default policy and give the person a
//! dead legacy role of Admin: neither may matter.

use std::time::Duration;

use horae_core::permissions::catalog::BuiltInProfile::{self, *};
use horae_core::permissions::catalog::Permission::{self, *};
use horae_core::types::OrgRole;
use sqlx::PgPool;
use uuid::Uuid;

use super::*;
use crate::ops::Caller;
use crate::ops::testing::{Recorder, provision, set_grants};
use crate::server_fns::test_seed::{SeedIds, seed, wait_for_blocked};

#[derive(Debug, Clone, Copy)]
enum Who {
    Profile(BuiltInProfile),
    Grants(&'static [Permission]),
}

async fn fixture(pool: &PgPool, who: Who) -> SeedIds {
    let ids = seed(pool, OrgRole::Admin).await;
    match who {
        Who::Profile(profile) => {
            provision(
                pool,
                &ids,
                profile.direct_permissions(),
                profile == Administrator,
            )
            .await;
        }
        Who::Grants(grants) => provision(pool, &ids, grants, false).await,
    }
    ids
}

/// Someone who may edit the branding and the time zone.
async fn administrator(pool: &PgPool) -> SeedIds {
    fixture(pool, Who::Profile(Administrator)).await
}

fn default_branding() -> OrgBranding {
    OrgBranding {
        provider_name: None,
        provider_address: None,
        provider_tax_id: None,
        provider_email: None,
        provider_phone: None,
        bank_name: None,
        bank_iban: None,
        bank_bic: None,
        bank_routing: None,
        bank_account: None,
        invoice_notes: None,
        invoice_payment_terms: Some("Net 30".into()),
    }
}

async fn row_version(pool: &PgPool, org_id: Uuid) -> Option<String> {
    sqlx::query_scalar!("SELECT xmin::text FROM organizations WHERE id = $1", org_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn stored(pool: &PgPool, org_id: Uuid) -> OrgBranding {
    crate::reports::fetch_branding_from(pool, org_id)
        .await
        .unwrap()
}

async fn workspace_zone(pool: &PgPool, org_id: Uuid) -> String {
    sqlx::query_scalar!("SELECT timezone FROM organizations WHERE id = $1", org_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// A save through the server function's path, edited from `expected`.
/// Returns the branding and whether it was announced as changed.
async fn save_from(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    expected: &OrgBranding,
    branding: &OrgBranding,
) -> Result<(OrgBranding, bool), OpError> {
    let events = Recorder::default();
    let mut write = crate::ops::begin(pool, Caller::session(org_id, actor_id)).await?;
    let result = update_branding(&mut write.tx, &write.actor, expected, branding).await;
    let saved = write.commit(result, &events).await?;
    let events = events.take();
    if let [event] = events.as_slice() {
        assert!(
            matches!(event, AppEvent::OrgBrandingUpdated { org, .. }
                if org.org_id == org_id && org.provider_name == saved.provider_name),
            "{event:?}"
        );
    }
    assert!(events.len() <= 1, "{events:?}");
    Ok((saved, !events.is_empty()))
}

/// A save edited from the current row, as a page that just loaded sends it.
async fn save(
    pool: &PgPool,
    org_id: Uuid,
    actor_id: Uuid,
    branding: &OrgBranding,
) -> Result<(OrgBranding, bool), OpError> {
    let expected = stored(pool, org_id).await;
    save_from(pool, org_id, actor_id, &expected, branding).await
}

async fn read(pool: &PgPool, org_id: Uuid, actor_id: Uuid) -> Result<OrgBranding, OpError> {
    let mut read = crate::ops::begin_read(pool, Caller::session(org_id, actor_id)).await?;
    let result = branding(&mut read.tx, &read.actor).await;
    read.finish(result).await
}

async fn set_zone(pool: &PgPool, org_id: Uuid, actor_id: Uuid, zone: &str) -> Result<(), OpError> {
    let mut write = crate::ops::begin(pool, Caller::session(org_id, actor_id)).await?;
    let result = set_timezone(&mut write.tx, &write.actor, zone).await;
    write.commit(result, &Recorder::default()).await
}

#[track_caller]
fn assert_forbidden<T: std::fmt::Debug>(result: Result<T, OpError>, context: impl std::fmt::Debug) {
    assert!(
        matches!(result, Err(OpError::Forbidden(_))),
        "{context:?}: {result:?}"
    );
}

// ── Who may read and save the branding ───────────────────────────────────────

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn branding_needs_invoice_write_for_everyone(pool: PgPool) {
    for (who, allowed) in [
        (Who::Profile(Administrator), true),
        (Who::Profile(ExecutiveManager), true),
        (Who::Profile(Accounting), true),
        (Who::Profile(ProjectManager), false),
        (Who::Profile(PeopleAdmin), false),
        (Who::Profile(Member), false),
        (Who::Grants(&[InvoiceWriteAll]), true),
        (Who::Grants(&[InvoiceReadAll]), false),
        (Who::Grants(&[InvoiceWriteManaged]), false),
        (Who::Grants(&[CompanyWrite]), false),
    ] {
        let ids = fixture(&pool, who).await;
        let before = row_version(&pool, ids.org_id).await;
        let requested = OrgBranding {
            provider_name: Some(format!("Saved by {who:?}")),
            ..default_branding()
        };
        let loaded = read(&pool, ids.org_id, ids.user_id).await;
        let saved = save(&pool, ids.org_id, ids.user_id, &requested).await;
        if allowed {
            assert_eq!(loaded.unwrap(), default_branding(), "{who:?}");
            assert_eq!(saved.unwrap(), (requested, true), "{who:?}");
        } else {
            assert_forbidden(loaded, who);
            assert_forbidden(saved, who);
            assert_eq!(row_version(&pool, ids.org_id).await, before, "{who:?}");
        }
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn branding_refuses_inactive_foreign_and_unknown_people_even_for_noops(pool: PgPool) {
    let ids = administrator(&pool).await;
    let foreign = administrator(&pool).await;
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
        .execute(&pool)
        .await
        .unwrap();
    let before = row_version(&pool, ids.org_id).await;
    for actor in [ids.user_id, foreign.user_id, Uuid::now_v7()] {
        assert_forbidden(read(&pool, ids.org_id, actor).await, actor);
        for name in [None, Some("Unauthorized change")] {
            let requested = OrgBranding {
                provider_name: name.map(str::to_owned),
                ..default_branding()
            };
            assert_forbidden(
                save(&pool, ids.org_id, actor, &requested).await,
                (actor, name),
            );
            assert_eq!(row_version(&pool, ids.org_id).await, before);
        }
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_missing_organization_is_not_an_unchanged_save(pool: PgPool) {
    let result = save_from(
        &pool,
        Uuid::now_v7(),
        Uuid::now_v7(),
        &default_branding(),
        &default_branding(),
    )
    .await;
    assert_forbidden(result, "missing organization");
}

// ── What a save writes ───────────────────────────────────────────────────────

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_save_edited_from_an_older_snapshot_is_refused_and_keeps_the_newer_values(pool: PgPool) {
    let ids = administrator(&pool).await;
    let opened = stored(&pool, ids.org_id).await;
    let newer = OrgBranding {
        bank_iban: Some("ES91 2100 0418 4502 0005 1332".into()),
        ..opened.clone()
    };
    save(&pool, ids.org_id, ids.user_id, &newer).await.unwrap();
    let before = row_version(&pool, ids.org_id).await;
    let phone_fix = OrgBranding {
        provider_phone: Some("+34 600 000 000".into()),
        ..opened.clone()
    };
    let result = save_from(&pool, ids.org_id, ids.user_id, &opened, &phone_fix).await;
    assert!(matches!(result, Err(OpError::Conflict(_))), "{result:?}");
    assert_eq!(
        (
            stored(&pool, ids.org_id).await,
            row_version(&pool, ids.org_id).await
        ),
        (newer, before)
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_stored_value_too_long_to_print_does_not_block_saving_other_fields(pool: PgPool) {
    let ids = administrator(&pool).await;
    let overlong = "x".repeat(crate::models::organization::MAX_BRANDING_VALUE_BYTES + 1);
    // Saved through the API before values were checked.
    sqlx::query!(
        "UPDATE organizations SET invoice_notes = $2 WHERE id = $1",
        ids.org_id,
        overlong,
    )
    .execute(&pool)
    .await
    .unwrap();
    let requested = OrgBranding {
        provider_name: Some("Acme".into()),
        ..stored(&pool, ids.org_id).await
    };
    let saved = save(&pool, ids.org_id, ids.user_id, &requested).await;
    assert_eq!(saved.unwrap(), (requested, true));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn unchanged_branding_preserves_the_row_and_announces_nothing(pool: PgPool) {
    let ids = administrator(&pool).await;
    let branding = default_branding();
    let before = row_version(&pool, ids.org_id).await;
    let saved = save(&pool, ids.org_id, ids.user_id, &branding).await;
    assert_eq!(
        (saved.unwrap(), row_version(&pool, ids.org_id).await),
        ((branding, false), before)
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn each_branding_field_changes_once_including_null_and_empty(pool: PgPool) {
    let ids = administrator(&pool).await;
    let mut fields = serde_json::to_value(default_branding()).unwrap();
    for field in [
        "provider_name",
        "provider_address",
        "provider_tax_id",
        "provider_email",
        "provider_phone",
        "bank_name",
        "bank_iban",
        "bank_bic",
        "bank_routing",
        "bank_account",
        "invoice_notes",
        "invoice_payment_terms",
    ] {
        for value in [Some("Changed"), Some(""), None] {
            fields[field] = serde_json::to_value(value).unwrap();
            let requested: OrgBranding = serde_json::from_value(fields.clone()).unwrap();
            let saved = save(&pool, ids.org_id, ids.user_id, &requested).await;
            assert_eq!(
                saved.unwrap(),
                (requested.clone(), true),
                "{field}: {value:?}"
            );
            let before = row_version(&pool, ids.org_id).await;
            let repeated = save(&pool, ids.org_id, ids.user_id, &requested).await;
            assert_eq!(
                (repeated.unwrap(), row_version(&pool, ids.org_id).await),
                ((requested, false), before),
                "repeated {field}: {value:?}"
            );
        }
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn branding_refuses_a_value_the_invoice_pdf_cannot_print(pool: PgPool) {
    let ids = administrator(&pool).await;
    let longest = "x".repeat(crate::models::organization::MAX_BRANDING_VALUE_BYTES);
    let accepted = OrgBranding {
        invoice_notes: Some(longest.clone()),
        ..default_branding()
    };
    save(&pool, ids.org_id, ids.user_id, &accepted)
        .await
        .unwrap();
    let before = row_version(&pool, ids.org_id).await;
    for refused in [longest + "x", "Nul\0byte".into()] {
        let requested = OrgBranding {
            bank_account: Some(refused),
            ..accepted.clone()
        };
        let result = save(&pool, ids.org_id, ids.user_id, &requested).await;
        assert!(matches!(result, Err(OpError::Invalid { .. })), "{result:?}");
        assert_eq!(row_version(&pool, ids.org_id).await, before);
    }
}

// ── Concurrency ──────────────────────────────────────────────────────────────

/// A save opened before a competing edit, which commits while the save waits
/// for the row. Returns the save's result and the row's name afterwards.
async fn competing_edit(
    pool: &PgPool,
    requested_name: Option<&str>,
) -> (Result<(OrgBranding, bool), OpError>, Option<String>) {
    let ids = administrator(pool).await;
    let opened = stored(pool, ids.org_id).await;
    let mut first = pool.begin().await.unwrap();
    let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *first)
        .await
        .unwrap()
        .unwrap();
    sqlx::query!(
        "UPDATE organizations SET provider_name = 'Changed' WHERE id = $1",
        ids.org_id
    )
    .execute(&mut *first)
    .await
    .unwrap();
    let mut requested = default_branding();
    requested.provider_name = requested_name.map(str::to_owned);
    let run_pool = pool.clone();
    let mut run = tokio::task::JoinSet::new();
    run.spawn(
        async move { save_from(&run_pool, ids.org_id, ids.user_id, &opened, &requested).await },
    );
    wait_for_blocked(pool, blocker).await;
    first.commit().await.unwrap();
    let result = tokio::time::timeout(Duration::from_secs(5), run.join_next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    (result, stored(pool, ids.org_id).await.provider_name)
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_save_waiting_on_a_competing_edit_is_refused_whatever_it_requested(pool: PgPool) {
    for requested in [Some("Changed"), None] {
        let (result, name) = competing_edit(&pool, requested).await;
        assert!(
            matches!(result, Err(OpError::Conflict(_))),
            "{requested:?}: {result:?}"
        );
        assert_eq!(name.as_deref(), Some("Changed"), "{requested:?}");
    }
}

/// An access change that holds the organization row while the save waits:
/// the permission editor's revocation, or a deactivation.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn branding_rechecks_authority_after_the_organization_wait(pool: PgPool) {
    // The write's isolation must not come from the session default.
    let run_pool = crate::server_fns::test_seed::repeatable_read_pool(&pool).await;
    for deactivate in [false, true] {
        for change in [false, true] {
            let ids = administrator(&pool).await;
            let mut revoke = pool.begin().await.unwrap();
            let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
                .fetch_one(&mut *revoke)
                .await
                .unwrap()
                .unwrap();
            sqlx::query!(
                "SELECT id FROM organizations WHERE id = $1 FOR UPDATE",
                ids.org_id
            )
            .fetch_one(&mut *revoke)
            .await
            .unwrap();
            let requested = OrgBranding {
                provider_name: change.then(|| "Unauthorized change".to_owned()),
                ..default_branding()
            };
            let db = run_pool.clone();
            let mut run = tokio::task::JoinSet::new();
            run.spawn(async move { save(&db, ids.org_id, ids.user_id, &requested).await });
            wait_for_blocked(&pool, blocker).await;
            if deactivate {
                sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
                    .execute(&mut *revoke)
                    .await
                    .unwrap();
            } else {
                set_grants(&mut revoke, &ids, &[]).await.unwrap();
            }
            revoke.commit().await.unwrap();
            let result = tokio::time::timeout(Duration::from_secs(5), run.join_next())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            assert_forbidden(result, (deactivate, change));
            // The revocation itself moves the row, so compare its values.
            assert_eq!(stored(&pool, ids.org_id).await, default_branding());
        }
    }
    run_pool.close().await;
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn branding_rechecks_authority_after_a_deactivation_wait(pool: PgPool) {
    let ids = administrator(&pool).await;
    let before = row_version(&pool, ids.org_id).await;
    let mut revoke = pool.begin().await.unwrap();
    let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *revoke)
        .await
        .unwrap()
        .unwrap();
    sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
        .execute(&mut *revoke)
        .await
        .unwrap();
    let db = pool.clone();
    let mut run = tokio::task::JoinSet::new();
    run.spawn(async move { save(&db, ids.org_id, ids.user_id, &default_branding()).await });
    wait_for_blocked(&pool, blocker).await;
    revoke.commit().await.unwrap();
    let result = tokio::time::timeout(Duration::from_secs(5), run.join_next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_forbidden(result, "deactivated");
    assert_eq!(row_version(&pool, ids.org_id).await, before);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn branding_holds_authority_until_its_write_commits(pool: PgPool) {
    let ids = administrator(&pool).await;
    let requested = OrgBranding {
        provider_name: Some("Authorized before revocation".into()),
        ..default_branding()
    };
    let mut barrier = pool.begin().await.unwrap();
    let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *barrier)
        .await
        .unwrap()
        .unwrap();
    // SHARE permits the actor's FOR SHARE locks but blocks the UPDATE.
    sqlx::query!("LOCK TABLE organizations IN SHARE MODE")
        .execute(&mut *barrier)
        .await
        .unwrap();
    let db = pool.clone();
    let submitted = requested.clone();
    let mut write = tokio::task::JoinSet::new();
    write.spawn(async move { save(&db, ids.org_id, ids.user_id, &submitted).await });
    wait_for_blocked(&pool, blocker).await;
    let writer = sqlx::query_scalar!(
        "SELECT pid AS \"pid!\" FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid))",
        blocker,
    ).fetch_one(&pool).await.unwrap();
    let db = pool.clone();
    let mut revoke = tokio::task::JoinSet::new();
    revoke.spawn(async move {
        sqlx::query!("UPDATE users SET active = false WHERE id = $1", ids.user_id)
            .execute(&db)
            .await
    });
    tokio::select! {
        result = revoke.join_next() => panic!("revocation completed before branding committed: {result:?}"),
        () = wait_for_blocked(&pool, writer) => {},
    }
    barrier.commit().await.unwrap();
    let saved = tokio::time::timeout(Duration::from_secs(5), write.join_next())
        .await
        .unwrap()
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(saved, (requested, true));
    tokio::time::timeout(Duration::from_secs(5), revoke.join_next())
        .await
        .unwrap()
        .unwrap()
        .unwrap()
        .unwrap();
    assert_forbidden(
        save(&pool, ids.org_id, ids.user_id, &default_branding()).await,
        "after revocation",
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn a_failed_branding_write_rolls_back_and_releases_its_locks(pool: PgPool) {
    let ids = administrator(&pool).await;
    let before = row_version(&pool, ids.org_id).await;
    sqlx::query!(
        "ALTER TABLE organizations ADD CONSTRAINT reject_branding_write CHECK (false) NOT VALID"
    )
    .execute(&pool)
    .await
    .unwrap();
    let requested = OrgBranding {
        provider_name: Some("Must roll back".into()),
        ..default_branding()
    };
    let result = save(&pool, ids.org_id, ids.user_id, &requested).await;
    assert!(matches!(result, Err(OpError::Internal(_))), "{result:?}");
    assert_eq!(row_version(&pool, ids.org_id).await, before);
    // Transaction drop queues rollback; wait for completion, not a scheduler tick.
    tokio::time::timeout(Duration::from_secs(5), async {
        let mut probe = pool.begin().await.unwrap();
        sqlx::query!(
            "SELECT id FROM organizations WHERE id = $1 FOR UPDATE",
            ids.org_id
        )
        .fetch_one(&mut *probe)
        .await
        .unwrap();
        sqlx::query!("SELECT id FROM users WHERE id = $1 FOR UPDATE", ids.user_id)
            .fetch_one(&mut *probe)
            .await
            .unwrap();
        probe.rollback().await.unwrap();
    })
    .await
    .expect("failed branding update must release organization and actor locks");
    sqlx::query!("ALTER TABLE organizations DROP CONSTRAINT reject_branding_write")
        .execute(&pool)
        .await
        .unwrap();
    let saved = save(&pool, ids.org_id, ids.user_id, &requested).await;
    assert_eq!(saved.unwrap(), (requested, true));
}

// ── Workspace time zone ──────────────────────────────────────────────────────

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_workspace_zone_needs_company_write(pool: PgPool) {
    for (who, allowed) in [
        (Who::Profile(Administrator), true),
        (Who::Grants(&[CompanyWrite]), true),
        (Who::Profile(ExecutiveManager), false),
        (Who::Profile(Accounting), false),
        (Who::Profile(Member), false),
        (Who::Grants(&[CompanyRead]), false),
        (Who::Grants(&[InvoiceWriteAll]), false),
    ] {
        let ids = fixture(&pool, who).await;
        let result = set_zone(&pool, ids.org_id, ids.user_id, "America/Los_Angeles").await;
        if allowed {
            result.unwrap();
            assert_eq!(
                workspace_zone(&pool, ids.org_id).await,
                "America/Los_Angeles"
            );
        } else {
            assert_forbidden(result, who);
            assert_eq!(workspace_zone(&pool, ids.org_id).await, "UTC", "{who:?}");
        }
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_administrator_of_another_organization_cannot_change_the_zone(pool: PgPool) {
    let ours = administrator(&pool).await;
    let theirs = administrator(&pool).await;
    assert_forbidden(
        set_zone(&pool, ours.org_id, theirs.user_id, "Asia/Tokyo").await,
        "foreign",
    );
    assert_eq!(workspace_zone(&pool, ours.org_id).await, "UTC");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_workspace_zone_is_validated(pool: PgPool) {
    let ids = administrator(&pool).await;
    for bad in ["Nowhere/City", "europe/madrid", ""] {
        let result = set_zone(&pool, ids.org_id, ids.user_id, bad).await;
        assert!(
            matches!(
                result,
                Err(OpError::Invalid {
                    field: Some("timezone"),
                    ..
                })
            ),
            "{bad:?}: {result:?}"
        );
    }
    assert_eq!(workspace_zone(&pool, ids.org_id).await, "UTC");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn the_zone_rechecks_authority_after_a_revocation_wait(pool: PgPool) {
    let ids = administrator(&pool).await;
    let mut revoke = pool.begin().await.unwrap();
    let blocker = sqlx::query_scalar!("SELECT pg_backend_pid()")
        .fetch_one(&mut *revoke)
        .await
        .unwrap()
        .unwrap();
    set_grants(&mut revoke, &ids, &[]).await.unwrap();
    let db = pool.clone();
    let mut run = tokio::task::JoinSet::new();
    run.spawn(async move { set_zone(&db, ids.org_id, ids.user_id, "Asia/Tokyo").await });
    wait_for_blocked(&pool, blocker).await;
    revoke.commit().await.unwrap();
    let result = tokio::time::timeout(Duration::from_secs(5), run.join_next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_forbidden(result, "revoked");
    assert_eq!(workspace_zone(&pool, ids.org_id).await, "UTC");
}
