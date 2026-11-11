use super::linking::{identity, wait_for_links};
use super::*;
use sqlx::PgPool;
use tokio::{task::JoinSet, time::timeout};

const ISSUER: &str = "https://id.example.com";
const EMAIL: &str = "ada@example.com";

fn allow(domains: &[&str], trust_unverified_email: bool) -> AutoProvision {
    AutoProvision {
        domains: domains.iter().map(|d| d.to_string()).collect(),
        trust_unverified_email,
    }
}

fn token(subject: &str, email: &str, email_verified: Option<bool>) -> Identity {
    let mut token = identity(ISSUER, subject, Some(email.into()), true);
    token.email_verified = email_verified;
    token
}

async fn organization(pool: &PgPool) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO organizations (id, name) VALUES ($1, 'Test Org')",
        id
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

/// An existing account with `email`, optionally bound to `(issuer, subject)`.
async fn account(
    pool: &PgPool,
    org_id: Uuid,
    email: &str,
    active: bool,
    binding: Option<(Option<&str>, &str)>,
) -> Uuid {
    let id = Uuid::now_v7();
    let (issuer, subject) = binding.unzip();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name, active, oidc_issuer, oidc_subject)
         VALUES ($1, $2, $3, 'Existing', $4, $5, $6)",
        id,
        org_id,
        email,
        active,
        issuer.flatten(),
        subject,
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn accounts(pool: &PgPool) -> i64 {
    sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count!" FROM users"#)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn refused(pool: &PgPool, identity: &Identity, policy: &AutoProvision) -> LoginNotice {
    resolve_user(pool, identity, policy)
        .await
        .unwrap()
        .expect_err("sign-in must be refused")
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn first_sign_in_creates_an_active_member_bound_to_the_identity(pool: PgPool) {
    let org_id = organization(&pool).await;
    let mut token = token("new-subject", "Ada@Example.com", Some(true));
    token.name = Some("Ada Lovelace".into());

    let sign_in = resolve_user(&pool, &token, &allow(&["example.com"], false))
        .await
        .unwrap()
        .unwrap();
    assert!(sign_in.created);
    let user = sign_in.user;
    assert_eq!(
        (user.org_id, user.email.as_str(), user.name.as_str()),
        (org_id, "Ada@Example.com", "Ada Lovelace")
    );
    assert_eq!(user.org_role, OrgRole::Member);
    assert_eq!(
        crate::server_fns::test_seed::built_in_profile(&pool, org_id, user.id).await,
        Some((
            horae_core::permissions::catalog::BuiltInProfile::Member,
            false
        ))
    );
    assert_eq!(
        (user.cost_rate_cents, user.billable_rate_cents),
        (None, None)
    );
    assert!(user.active);
    let binding = sqlx::query!(
        "SELECT oidc_issuer, oidc_subject FROM users WHERE id = $1",
        user.id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (
            binding.oidc_issuer.as_deref(),
            binding.oidc_subject.as_deref()
        ),
        (Some(ISSUER), Some("new-subject"))
    );
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn later_sign_ins_reuse_the_created_account(pool: PgPool) {
    organization(&pool).await;
    let token = token("new-subject", EMAIL, Some(true));
    let policy = allow(&["example.com"], false);
    let first = resolve_user(&pool, &token, &policy).await.unwrap().unwrap();
    let again = resolve_user(&pool, &token, &policy).await.unwrap().unwrap();
    assert_eq!(again.user.id, first.user.id);
    assert!(!again.created);
    assert_eq!(accounts(&pool).await, 1);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn created_account_is_named_after_the_email_without_a_name_claim(pool: PgPool) {
    organization(&pool).await;
    let sign_in = resolve_user(
        &pool,
        &token("new-subject", EMAIL, Some(true)),
        &allow(&["example.com"], false),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(sign_in.user.name, "ada");
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn nothing_is_created_when_provisioning_is_off(pool: PgPool) {
    organization(&pool).await;
    let notice = refused(
        &pool,
        &token("new-subject", EMAIL, Some(true)),
        &AutoProvision::default(),
    )
    .await;
    assert_eq!(
        notice,
        LoginNotice::NoAccount {
            email: Some(EMAIL.into())
        }
    );
    assert_eq!(accounts(&pool).await, 0);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_unlisted_domain_is_refused(pool: PgPool) {
    organization(&pool).await;
    let notice = refused(
        &pool,
        &token("new-subject", "ada@other.example", Some(true)),
        &allow(&["example.com"], true),
    )
    .await;
    assert_eq!(
        notice,
        LoginNotice::DomainNotAllowed {
            email: "ada@other.example".into()
        }
    );
    assert_eq!(accounts(&pool).await, 0);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_unverified_email_is_refused(pool: PgPool) {
    organization(&pool).await;
    // `false` is refused whatever the trust switch says; an absent claim is
    // refused without it.
    for (email_verified, trust) in [(Some(false), false), (Some(false), true), (None, false)] {
        let notice = refused(
            &pool,
            &token("new-subject", EMAIL, email_verified),
            &allow(&["example.com"], trust),
        )
        .await;
        assert_eq!(
            notice,
            LoginNotice::EmailUnconfirmed {
                email: EMAIL.into()
            },
            "{email_verified:?}, trust {trust}"
        );
    }
    assert_eq!(accounts(&pool).await, 0);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_absent_claim_provisions_when_the_operator_trusts_it(pool: PgPool) {
    organization(&pool).await;
    let sign_in = resolve_user(
        &pool,
        &token("tailnet-subject", EMAIL, None),
        &allow(&["example.com"], true),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(sign_in.created && sign_in.user.active);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn the_trust_switch_never_binds_an_existing_account_by_email(pool: PgPool) {
    let org_id = organization(&pool).await;
    let existing = account(&pool, org_id, EMAIL, true, None).await;

    let notice = refused(
        &pool,
        &token("tailnet-subject", EMAIL, None),
        &allow(&["example.com"], true),
    )
    .await;
    // An account holds the address, so the notice must not claim otherwise.
    assert_eq!(notice, LoginNotice::NoAccount { email: None });
    let subject = sqlx::query_scalar!("SELECT oidc_subject FROM users WHERE id = $1", existing)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(subject, None);
    assert_eq!(accounts(&pool).await, 1);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_deactivated_unbound_account_is_neither_recreated_nor_reactivated(pool: PgPool) {
    let org_id = organization(&pool).await;
    let existing = account(&pool, org_id, EMAIL, false, None).await;

    let notice = refused(
        &pool,
        &token("new-subject", EMAIL, Some(true)),
        &allow(&["example.com"], false),
    )
    .await;
    assert_eq!(notice, LoginNotice::Deactivated);
    let active = sqlx::query_scalar!("SELECT active FROM users WHERE id = $1", existing)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!active);
    assert_eq!(accounts(&pool).await, 1);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn an_email_held_by_another_identity_is_not_reused(pool: PgPool) {
    // Active or deactivated, bound under the same issuer or another, or held in
    // another letter case: nothing is created and the notice reveals nothing
    // about the other account.
    let cases = [
        (EMAIL, true, Some((Some(ISSUER), "other-subject"))),
        (EMAIL, false, Some((Some(ISSUER), "other-subject"))),
        (
            EMAIL,
            true,
            Some((Some("https://other.example"), "other-subject")),
        ),
        ("ADA@example.com", true, None),
    ];
    for (held, active, binding) in cases {
        let org_id = organization(&pool).await;
        let existing = account(&pool, org_id, held, active, binding).await;

        let notice = refused(
            &pool,
            &token("new-subject", EMAIL, Some(true)),
            &allow(&["example.com"], false),
        )
        .await;
        assert_eq!(
            notice,
            LoginNotice::NoAccount { email: None },
            "{held} {active} {binding:?}"
        );
        assert_eq!(accounts(&pool).await, 1);
        sqlx::query!("DELETE FROM users WHERE id = $1", existing)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query!("DELETE FROM organizations WHERE id = $1", org_id)
            .execute(&pool)
            .await
            .unwrap();
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn a_subject_held_by_another_account_is_not_reused(pool: PgPool) {
    // The pair misses (another issuer, or a legacy binding with none), and a
    // second account with the same subject would break its uniqueness.
    for issuer in [Some("https://other.example"), None] {
        let org_id = organization(&pool).await;
        let holder = account(
            &pool,
            org_id,
            "holder@example.com",
            true,
            Some((issuer, "held")),
        )
        .await;

        let notice = refused(
            &pool,
            &token("held", EMAIL, Some(true)),
            &allow(&["example.com"], false),
        )
        .await;
        assert_eq!(notice, LoginNotice::NoAccount { email: None }, "{issuer:?}");
        assert_eq!(accounts(&pool).await, 1);
        sqlx::query!("DELETE FROM users WHERE id = $1", holder)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query!("DELETE FROM organizations WHERE id = $1", org_id)
            .execute(&pool)
            .await
            .unwrap();
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn nothing_is_created_without_exactly_one_organization(pool: PgPool) {
    let token = token("new-subject", EMAIL, Some(true));
    let policy = allow(&["example.com"], false);
    for organizations in [0, 2] {
        for _ in 0..organizations {
            organization(&pool).await;
        }
        assert!(matches!(
            refused(&pool, &token, &policy).await,
            LoginNotice::NoAccount { .. }
        ));
        assert_eq!(accounts(&pool).await, 0);
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn concurrent_first_sign_ins_create_one_account(pool: PgPool) {
    let org_id = organization(&pool).await;
    let blocker = hold_organization(&pool, org_id).await;

    let mut attempts = JoinSet::new();
    for _ in 0..2 {
        let db = pool.clone();
        attempts.spawn(async move {
            resolve_user(
                &db,
                &token("racing-subject", EMAIL, Some(true)),
                &allow(&["example.com"], false),
            )
            .await
            .unwrap()
            .unwrap()
        });
    }
    wait_for_links(&pool, 2).await;
    blocker.commit().await.unwrap();

    let results = timeout(std::time::Duration::from_secs(10), attempts.join_all())
        .await
        .unwrap();
    assert_eq!(results[0].user.id, results[1].user.id);
    assert_eq!(results.iter().filter(|sign_in| sign_in.created).count(), 1);
    assert_eq!(accounts(&pool).await, 1);
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn concurrent_sign_ins_with_case_variant_emails_create_one_account(pool: PgPool) {
    let org_id = organization(&pool).await;
    let blocker = hold_organization(&pool, org_id).await;

    // Different people at the provider whose addresses differ only in case:
    // the case-sensitive unique index alone would let both rows in.
    let mut attempts = JoinSet::new();
    for (subject, email) in [
        ("subject-a", "Ada@example.com"),
        ("subject-b", "ada@example.com"),
    ] {
        let db = pool.clone();
        attempts.spawn(async move {
            resolve_user(
                &db,
                &token(subject, email, Some(true)),
                &allow(&["example.com"], false),
            )
            .await
            .unwrap()
        });
    }
    wait_for_links(&pool, 2).await;
    blocker.commit().await.unwrap();

    let results = timeout(std::time::Duration::from_secs(10), attempts.join_all())
        .await
        .unwrap();
    assert_eq!(accounts(&pool).await, 1);
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    let loser = results.into_iter().find_map(Result::err);
    assert_eq!(loser, Some(LoginNotice::NoAccount { email: None }));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn admin_creation_and_provisioning_of_case_variant_emails_create_one_account(pool: PgPool) {
    let ids = crate::server_fns::test_seed::seed(&pool, OrgRole::Admin).await;
    // Hold the email lock. The sign-in takes its share of the organization row
    // and queues on the email; the administrator's exclusive organization lock
    // then queues behind the sign-in, so the sign-in commits first and the
    // administrator must see its lowercase twin.
    let mut blocker = pool.begin().await.unwrap();
    crate::db::lock_new_user_email(&mut blocker, EMAIL)
        .await
        .unwrap();

    let db = pool.clone();
    let sign_in = tokio::spawn(async move {
        resolve_user(
            &db,
            &token("new-subject", EMAIL, Some(true)),
            &allow(&["example.com"], false),
        )
        .await
        .unwrap()
    });
    wait_for_links(&pool, 1).await;
    let db = pool.clone();
    let admin = tokio::spawn(async move {
        crate::server_fns::insert_user(
            &db,
            ids.org_id,
            ids.user_id,
            "Ada@example.com",
            "Ada",
            OrgRole::Member,
        )
        .await
    });
    wait_for_links(&pool, 2).await;
    blocker.commit().await.unwrap();

    let (admin, sign_in) = timeout(std::time::Duration::from_secs(10), async {
        (admin.await.unwrap(), sign_in.await.unwrap())
    })
    .await
    .unwrap();
    let ada = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "count!" FROM users WHERE lower(email) = 'ada@example.com'"#
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(ada, 1);
    assert!(sign_in.is_ok_and(|sign_in| sign_in.created));
    assert!(
        admin.is_err(),
        "the administrator must not create a case twin"
    );
}

/// Hold the organization row so the first insert waits on its foreign key
/// check and any other waits behind it, putting every attempt in flight before
/// either commits. Commit the returned transaction to release them.
async fn hold_organization(
    pool: &PgPool,
    org_id: Uuid,
) -> sqlx::Transaction<'static, sqlx::Postgres> {
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query!(
        "SELECT id FROM organizations WHERE id = $1 FOR UPDATE",
        org_id
    )
    .fetch_one(&mut *blocker)
    .await
    .unwrap();
    blocker
}
