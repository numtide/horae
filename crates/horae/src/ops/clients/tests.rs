//! Each client operation allowed and refused by grants, per OP17/OP18 and
//! FR-035/036.

use horae_core::permissions::catalog::BuiltInProfile;
use horae_core::permissions::catalog::Permission::{self, *};
use sqlx::PgPool;

use super::*;
use crate::ops::Caller;
use crate::ops::testing::{Recorder, policy_one, seed_profile};
use crate::server_fns::test_seed::{SeedIds, seed};

/// Which authority a fixture gives the seeded person.
#[derive(Debug, Clone, Copy)]
enum Who {
    Profile(BuiltInProfile),
    /// Exactly these grants. The dead legacy role is Admin on purpose: it
    /// must not matter.
    Grants(&'static [Permission]),
}

const CATALOG: &[Permission] = &[ClientReadAll];
const WRITER: &[Permission] = &[ClientWriteAll];
const RATES: &[Permission] = &[ClientWriteAll, BillableRateWriteAll];
const RATE_READER: &[Permission] = &[ClientReadAll, BillableRateReadAll];
const INVOICES: &[Permission] = &[ClientReadAll, InvoiceReadAll];
const NOTHING: &[Permission] = &[];

/// A seeded organization whose client has a default rate, an address and a
/// tax ID, and whose project is archived (so the client may be archived).
async fn fixture(pool: &PgPool, who: Who) -> SeedIds {
    let ids = match who {
        Who::Profile(profile) => seed_profile(pool, profile).await,
        Who::Grants(grants) => {
            let ids = seed(pool, BuiltInProfile::Member).await;
            policy_one(pool, &ids, grants, false).await;
            ids
        }
    };
    sqlx::query!(
        "UPDATE clients SET default_rate_cents = 12345, address = 'Street', tax_id = 'VAT'
         WHERE id = $1",
        ids.client_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE projects SET active = false WHERE id = $1",
        ids.project_id
    )
    .execute(pool)
    .await
    .unwrap();
    ids
}

fn caller(ids: &SeedIds) -> Caller {
    Caller::session(ids.org_id, ids.user_id)
}

/// One write by the seeded person, through the server functions' path.
async fn with_tx<T>(
    pool: &PgPool,
    events: &Recorder,
    caller: Caller,
    op: impl AsyncFnOnce(&mut Tx, &Actor) -> Result<Done<T>, OpError>,
) -> Result<T, OpError> {
    let mut write = crate::ops::begin(pool, caller).await?;
    let result = op(&mut write.tx, &write.actor).await;
    write.commit(result, events).await
}

async fn with_read_tx<T>(
    pool: &PgPool,
    caller: Caller,
    op: impl AsyncFnOnce(&mut Tx, &Actor) -> Result<T, OpError>,
) -> Result<T, OpError> {
    let mut read = crate::ops::begin_read(pool, caller).await?;
    let result = op(&mut read.tx, &read.actor).await;
    read.finish(result).await
}

fn profile() -> ClientProfile {
    ClientProfile {
        name: "New".into(),
        currency: "EUR".into(),
        address: None,
        tax_id: None,
    }
}

fn keep(currency: &str, rate: Option<i64>) -> ClientProfileEdit {
    ClientProfileEdit {
        profile: ClientProfile {
            name: "Renamed".into(),
            ..profile()
        },
        rate_change: ClientRateChange::Keep,
        original: crate::models::client::ClientBillingSnapshot {
            currency: currency.into(),
            default_rate_cents: rate,
        },
    }
}

async fn create_with(pool: &PgPool, who: Who, rate: &str) -> Result<ClientDetails, OpError> {
    let ids = fixture(pool, who).await;
    with_tx(
        pool,
        &Recorder::default(),
        caller(&ids),
        async |tx, actor| create(tx, actor, &profile(), rate).await,
    )
    .await
}

async fn rate_of(pool: &PgPool, client_id: Uuid) -> Option<i64> {
    sqlx::query_scalar!(
        "SELECT default_rate_cents FROM clients WHERE id = $1",
        client_id
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn create_needs_client_write_and_global_rate_write_for_a_rate(pool: PgPool) {
    for (who, rate, allowed) in [
        (Who::Profile(BuiltInProfile::Administrator), "10", true),
        // Executive Manager reads rates everywhere but cannot set them.
        (Who::Profile(BuiltInProfile::ExecutiveManager), "", true),
        (Who::Profile(BuiltInProfile::ExecutiveManager), "10", false),
        (Who::Profile(BuiltInProfile::Member), "", false),
        (Who::Grants(WRITER), "", true),
        (Who::Grants(WRITER), "10", false),
        (Who::Grants(RATES), "10", true),
        (Who::Grants(CATALOG), "", false),
        (Who::Grants(&[BillableRateWriteAll]), "10", false),
        (
            Who::Grants(&[ClientWriteAll, BillableRateWriteManaged]),
            "10",
            false,
        ),
    ] {
        let result = create_with(&pool, who, rate).await;
        match (allowed, result) {
            (true, Ok(created)) => {
                let expected = (!rate.is_empty()).then_some(1000);
                assert_eq!(rate_of(&pool, created.client.id).await, expected);
            }
            (false, Err(OpError::Forbidden(_))) => {}
            (_, other) => panic!("{who:?} rate {rate:?}: {other:?}"),
        }
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn created_client_shows_its_rate_only_to_rate_readers(pool: PgPool) {
    let without = create_with(&pool, Who::Grants(WRITER), "").await.unwrap();
    assert_eq!(without.billing, None);
    let with = create_with(&pool, Who::Grants(RATES), "").await.unwrap();
    assert_eq!(
        with.billing,
        Some(ClientBilling {
            default_rate_cents: None
        })
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn create_announces_the_client_after_commit(pool: PgPool) {
    let ids = fixture(&pool, Who::Grants(WRITER)).await;
    let events = Recorder::default();
    let created = with_tx(&pool, &events, caller(&ids), async |tx, actor| {
        create(tx, actor, &profile(), "").await
    })
    .await
    .unwrap();
    let events = events.take();
    assert!(
        matches!(&events[..], [AppEvent::ClientCreated { client, .. }] if client.id == created.client.id)
    );
}

async fn update_with(
    pool: &PgPool,
    who: Who,
    edit: impl Fn(&SeedIds) -> ClientProfileEdit,
) -> (SeedIds, Result<ClientDetails, OpError>) {
    let ids = fixture(pool, who).await;
    let edit = edit(&ids);
    let result = with_tx(
        pool,
        &Recorder::default(),
        caller(&ids),
        async |tx, actor| update(tx, actor, ids.client_id, &edit).await,
    )
    .await;
    (ids, result)
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn update_needs_client_write(pool: PgPool) {
    for (who, snapshot, allowed) in [
        (
            Who::Profile(BuiltInProfile::Administrator),
            Some(12345),
            true,
        ),
        (Who::Profile(BuiltInProfile::Member), None, false),
        (Who::Grants(WRITER), None, true),
        (Who::Grants(CATALOG), None, false),
        (Who::Grants(NOTHING), None, false),
    ] {
        let (ids, result) = update_with(&pool, who, |_| keep("EUR", snapshot)).await;
        match (allowed, result) {
            (true, Ok(updated)) => {
                assert_eq!(updated.client.name, "Renamed");
                assert_eq!(rate_of(&pool, ids.client_id).await, Some(12345));
            }
            (false, Err(OpError::Forbidden(_))) => {}
            (_, other) => panic!("{who:?}: {other:?}"),
        }
    }
}

/// FR-035: someone who may edit clients but not see global rates keeps the
/// saved rate untouched and cannot replace or clear it.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn update_keeps_a_withheld_rate_and_needs_rate_write_to_change_it(pool: PgPool) {
    let (ids, kept) = update_with(&pool, Who::Grants(WRITER), |_| keep("EUR", None)).await;
    assert_eq!(kept.unwrap().billing, None);
    assert_eq!(rate_of(&pool, ids.client_id).await, Some(12345));

    for change in [
        ClientRateChange::Clear,
        ClientRateChange::Replace("1".into()),
    ] {
        let (ids, refused) = update_with(&pool, Who::Grants(WRITER), |_| ClientProfileEdit {
            rate_change: change.clone(),
            ..keep("EUR", None)
        })
        .await;
        assert!(matches!(refused, Err(OpError::Forbidden(_))), "{refused:?}");
        assert_eq!(rate_of(&pool, ids.client_id).await, Some(12345));

        let (ids, allowed) = update_with(&pool, Who::Grants(RATES), |_| ClientProfileEdit {
            rate_change: change.clone(),
            ..keep("EUR", Some(12345))
        })
        .await;
        allowed.unwrap();
        let expected = match change {
            ClientRateChange::Replace(_) => Some(100),
            _ => None,
        };
        assert_eq!(rate_of(&pool, ids.client_id).await, expected);
    }

    // FR-035: the refusal must not tell a set rate (even zero) from no rate.
    for rate in [Some(12345), Some(0), None] {
        let ids = fixture(&pool, Who::Grants(WRITER)).await;
        sqlx::query!(
            "UPDATE clients SET default_rate_cents = $2 WHERE id = $1",
            ids.client_id,
            rate,
        )
        .execute(&pool)
        .await
        .unwrap();
        let edit = ClientProfileEdit {
            profile: ClientProfile {
                currency: "USD".into(),
                ..profile()
            },
            ..keep("EUR", None)
        };
        let refused = with_tx(
            &pool,
            &Recorder::default(),
            caller(&ids),
            async |tx, actor| update(tx, actor, ids.client_id, &edit).await,
        )
        .await;
        assert_eq!(
            refused.unwrap_err(),
            OpError::forbidden(CURRENCY_NEEDS_RATE_WRITE),
            "{rate:?}"
        );
        assert_eq!(rate_of(&pool, ids.client_id).await, rate);
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn update_with_rate_read_still_detects_a_stale_rate(pool: PgPool) {
    let (_, result) = update_with(
        &pool,
        Who::Grants(&[ClientWriteAll, BillableRateReadAll]),
        |_| keep("EUR", None),
    )
    .await;
    assert!(matches!(result, Err(OpError::Conflict(_))), "{result:?}");
}

async fn set_active_with(
    pool: &PgPool,
    who: Who,
    active: bool,
) -> (Result<Client, OpError>, Vec<AppEvent>) {
    let ids = fixture(pool, who).await;
    if active {
        sqlx::query!(
            "UPDATE clients SET active = false WHERE id = $1",
            ids.client_id
        )
        .execute(pool)
        .await
        .unwrap();
    }
    let events = Recorder::default();
    let result = with_tx(pool, &events, caller(&ids), async |tx, actor| {
        set_active(tx, actor, ids.client_id, active).await
    })
    .await;
    (result, events.take())
}

/// The class b row of the study (015 T238): policy one used to refuse every
/// status change.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn archive_and_restore_need_client_write(pool: PgPool) {
    for active in [false, true] {
        for (who, allowed) in [
            (Who::Profile(BuiltInProfile::ProjectManager), true),
            (Who::Profile(BuiltInProfile::Member), false),
            (Who::Grants(WRITER), true),
            (Who::Grants(CATALOG), false),
            (Who::Grants(&[ProjectWriteAll]), false),
        ] {
            let (result, events) = set_active_with(&pool, who, active).await;
            match (allowed, result) {
                (true, Ok(client)) => {
                    assert_eq!(client.active, active);
                    let announced = match events[..] {
                        [AppEvent::ClientReactivated { .. }] => true,
                        [AppEvent::ClientDeactivated { .. }] => false,
                        _ => panic!("{events:?}"),
                    };
                    assert_eq!(announced, active);
                }
                (false, Err(OpError::Forbidden(_))) => assert!(events.is_empty()),
                (_, other) => panic!("{who:?} active {active}: {other:?}"),
            }
        }
    }
}

async fn read<T>(
    pool: &PgPool,
    who: Who,
    op: impl AsyncFnOnce(&mut Tx, &Actor, &SeedIds) -> Result<T, OpError>,
) -> Result<T, OpError> {
    let ids = fixture(pool, who).await;
    with_read_tx(pool, caller(&ids), async |tx, actor| {
        op(tx, actor, &ids).await
    })
    .await
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn details_need_the_catalog_and_rates_need_global_rate_read(pool: PgPool) {
    for (who, outcome) in [
        (Who::Profile(BuiltInProfile::Administrator), Some(true)),
        (Who::Profile(BuiltInProfile::ExecutiveManager), Some(true)),
        (Who::Profile(BuiltInProfile::ProjectManager), Some(false)),
        (Who::Profile(BuiltInProfile::Member), None),
        (Who::Grants(CATALOG), Some(false)),
        (Who::Grants(RATE_READER), Some(true)),
        (
            Who::Grants(&[ClientReadAll, BillableRateReadManaged]),
            Some(false),
        ),
        (Who::Grants(&[BillableRateReadAll]), None),
        (Who::Grants(NOTHING), None),
    ] {
        let result = read(&pool, who, async |tx, actor, ids| {
            details(tx, actor, ids.client_id).await
        })
        .await;
        match (outcome, result) {
            (Some(rate), Ok(details)) => {
                assert_eq!(details.client.tax_id.as_deref(), Some("VAT"));
                let expected = rate.then_some(ClientBilling {
                    default_rate_cents: Some(12345),
                });
                assert_eq!(details.billing, expected, "{who:?}");
            }
            (None, Err(OpError::NotFound(_))) => {}
            (_, other) => panic!("{who:?}: {other:?}"),
        }
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn client_invoices_need_the_catalog_and_invoice_read(pool: PgPool) {
    for (who, outcome) in [
        (Who::Profile(BuiltInProfile::Accounting), "ok"),
        (Who::Profile(BuiltInProfile::ProjectManager), "forbidden"),
        (Who::Profile(BuiltInProfile::Member), "forbidden"),
        (Who::Grants(INVOICES), "ok"),
        (Who::Grants(CATALOG), "forbidden"),
        (
            Who::Grants(&[ClientReadAll, InvoiceReadManaged]),
            "forbidden",
        ),
        (Who::Grants(&[InvoiceReadAll]), "not found"),
    ] {
        let result = read(&pool, who, async |tx, actor, ids| {
            invoices(tx, actor, ids.client_id).await
        })
        .await;
        match (outcome, result) {
            ("ok", Ok(rows)) => assert!(rows.is_empty()),
            ("forbidden", Err(OpError::Forbidden(_)))
            | ("not found", Err(OpError::NotFound(_))) => {}
            (_, other) => panic!("{who:?}: {other:?}"),
        }
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_page_offers_writes_and_invoices_by_grant(pool: PgPool) {
    for (who, manage, invoices) in [
        (Who::Profile(BuiltInProfile::ExecutiveManager), true, true),
        (Who::Profile(BuiltInProfile::ProjectManager), true, false),
        (Who::Grants(&[ClientWriteAll]), true, false),
        (Who::Grants(INVOICES), false, true),
        (Who::Grants(CATALOG), false, false),
    ] {
        let (page, view) = read(&pool, who, async |tx, actor, ids| {
            let page = overview(tx, actor, None).await?;
            Ok((page, detail_view(tx, actor, ids.client_id, None).await?))
        })
        .await
        .unwrap();
        assert!(page.can_read, "{who:?}");
        assert_eq!(
            (page.can_manage, view.can_manage),
            (manage, manage),
            "{who:?}"
        );
        assert_eq!(view.can_read_invoices, invoices, "{who:?}");
    }
}

/// Catalog readers list every client with its contact fields; anyone else
/// sees only the names of clients behind projects they work on or may read.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn list_projects_names_beyond_the_catalog(pool: PgPool) {
    for (who, assigned, expected) in [
        (Who::Profile(BuiltInProfile::Member), false, None),
        (Who::Profile(BuiltInProfile::Member), true, Some(false)),
        (Who::Grants(CATALOG), false, Some(true)),
        (Who::Grants(NOTHING), false, None),
        (Who::Grants(NOTHING), true, Some(false)),
        (Who::Grants(&[ProjectReadAll]), false, Some(false)),
        (Who::Grants(&[ProjectReadManaged]), false, None),
    ] {
        let ids = fixture(&pool, who).await;
        if assigned {
            sqlx::query!(
                "INSERT INTO assignments (id, project_id, user_id, role) VALUES ($1, $2, $3, 'freelancer')",
                Uuid::now_v7(),
                ids.project_id,
                ids.user_id,
            )
            .execute(&pool)
            .await
            .unwrap();
        }
        let clients = with_read_tx(&pool, caller(&ids), async |tx, actor| {
            list(tx, actor, true).await
        })
        .await
        .unwrap();
        match expected {
            None => assert!(clients.is_empty(), "{who:?}: {clients:?}"),
            Some(contact) => {
                let [client] = &clients[..] else {
                    panic!("{who:?}: {clients:?}")
                };
                assert_eq!(client.id, ids.client_id);
                assert_eq!(client.tax_id.is_some(), contact, "{who:?}");
                assert_eq!(client.address.is_some(), contact, "{who:?}");
            }
        }
    }
}
