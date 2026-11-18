//! Shared fixture for the in-crate `#[sqlx::test]` tests: one minimal tenant
//! (org, user, client, project, task) to hang time entries off, and helpers
//! that wait on database locks. These tests live inside the crate because
//! `tests/` cannot import a bin crate's modules. Backend modules use it, so it
//! must not depend on `server_fns`.

use horae_core::permissions::catalog::BuiltInProfile;
use horae_core::types::ProjectRole;
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::{OrganizationLock, lock_organization};
use crate::models::Assignment;
use crate::ops::permissions::provisioning;

pub struct SeedIds {
    pub org_id: Uuid,
    pub user_id: Uuid,
    pub client_id: Uuid,
    pub project_id: Uuid,
    pub task_id: Uuid,
}

/// Wait for a real database lock dependency, rather than guessing task timing.
pub async fn wait_for_blocked(pool: &PgPool, blocker: i32) {
    wait_for_waiters(pool, blocker, 1).await;
}

/// Wait until `count` sessions are blocked behind `blocker`.
pub async fn wait_for_waiters(pool: &PgPool, blocker: i32, count: i64) {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let waiting = sqlx::query_scalar!(
                r#"SELECT count(*) AS "count!" FROM pg_stat_activity WHERE datname = current_database() AND $1 = ANY(pg_blocking_pids(pid))"#,
                blocker,
            ).fetch_one(pool).await.unwrap();
            if waiting >= count {
                break;
            }
            tokio::task::yield_now().await;
        }
    }).await.expect("operation must wait for the competing transaction");
}

/// Wait until no other session in this database is inside a transaction.
///
/// A dropped transaction's `ROLLBACK` is sent by the task that returns its
/// connection to the pool, and a cancelled read may close its connection
/// instead, so a read can report its result while its backend still holds
/// the organization and person locks it took.
pub async fn wait_for_other_transactions(pool: &PgPool) {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let open = sqlx::query_scalar!(
                r#"SELECT count(*) AS "count!" FROM pg_stat_activity WHERE datname = current_database() AND pid <> pg_backend_pid() AND xact_start IS NOT NULL"#,
            )
            .fetch_one(pool)
            .await
            .unwrap();
            if open == 0 {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("finished reads must end their transactions");
}

/// Whether another connection can lock the organization row `FOR UPDATE NOWAIT`
/// right now. Call it once a refused request has returned. The probe runs on
/// another thread while this runtime is blocked, so the pool's deferred
/// cleanup of a dropped transaction cannot run first: only a request that
/// rolled back before answering leaves the row free.
pub fn organization_lockable(pool: &PgPool, org_id: Uuid) -> bool {
    use sqlx::{Connection, PgConnection};
    let options = (*pool.connect_options()).clone();
    std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async move {
                let mut connection = PgConnection::connect_with(&options).await.unwrap();
                let mut probe = connection.begin().await.unwrap();
                sqlx::query!(
                    "SELECT id FROM organizations WHERE id = $1 FOR UPDATE NOWAIT",
                    org_id
                )
                .fetch_one(&mut *probe)
                .await
                .is_ok()
            })
    })
    .join()
    .unwrap()
}

/// A second pool on the same database whose transactions default to REPEATABLE
/// READ, to prove a writer selects the isolation its checks rely on rather than
/// inheriting the server default.
pub async fn repeatable_read_pool(pool: &PgPool) -> PgPool {
    // Connection `options` would pass the value's space unescaped, so set the
    // session default on each new connection instead.
    let rr = sqlx::postgres::PgPoolOptions::new()
        .after_connect(|connection, _| {
            Box::pin(async move {
                sqlx::query!(
                    "SET SESSION CHARACTERISTICS AS TRANSACTION ISOLATION LEVEL REPEATABLE READ"
                )
                .execute(connection)
                .await
                .map(|_| ())
            })
        })
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    let isolation =
        sqlx::query_scalar!(r#"SELECT current_setting('transaction_isolation') AS "isolation!""#)
            .fetch_one(&rr)
            .await
            .unwrap();
    assert_eq!(isolation, "repeatable read");
    rr
}

/// Seed the minimal tenant, giving the user `profile`'s permission state, and
/// return its ids.
pub async fn seed(pool: &PgPool, profile: BuiltInProfile) -> SeedIds {
    let ids = seed_tenant(pool).await;
    let mut connection = pool.acquire().await.unwrap();
    provisioning::provision(&mut connection, ids.org_id, ids.user_id, profile)
        .await
        .unwrap();
    ids
}

/// Adds an active person with `profile`'s stored state to `ids`' organization,
/// sharing its project and task.
pub async fn add_person(pool: &PgPool, ids: &SeedIds, profile: BuiltInProfile) -> SeedIds {
    let user_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name) VALUES ($1, $2, $3, 'Teammate')",
        user_id,
        ids.org_id,
        format!("{user_id}@test.com"),
    )
    .execute(pool)
    .await
    .unwrap();
    let mut connection = pool.acquire().await.unwrap();
    provisioning::provision(&mut connection, ids.org_id, user_id, profile)
        .await
        .unwrap();
    SeedIds { user_id, ..*ids }
}

/// Seed the minimal tenant with no permission state. The inserts name only
/// columns every schema version has, so tests that migrate from an older
/// schema can use it too.
pub async fn seed_tenant(pool: &PgPool) -> SeedIds {
    let org_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO organizations (id, name) VALUES ($1, 'Test Org')",
        org_id
    )
    .execute(pool)
    .await
    .unwrap();

    let user_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name) VALUES ($1, $2, $3, 'Test User')",
        user_id,
        org_id,
        format!("{user_id}@test.com"),
    )
    .execute(pool)
    .await
    .unwrap();

    let client_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO clients (id, org_id, name, currency) VALUES ($1, $2, 'Acme', 'EUR')",
        client_id,
        org_id,
    )
    .execute(pool)
    .await
    .unwrap();

    let project_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO projects (id, org_id, client_id, name, currency) \
         VALUES ($1, $2, $3, 'Widget', 'EUR')",
        project_id,
        org_id,
        client_id,
    )
    .execute(pool)
    .await
    .unwrap();

    let task_id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO tasks (id, org_id, name) VALUES ($1, $2, 'Dev')",
        task_id,
        org_id,
    )
    .execute(pool)
    .await
    .unwrap();

    SeedIds {
        org_id,
        user_id,
        client_id,
        project_id,
        task_id,
    }
}

pub async fn time_entry(
    pool: &PgPool,
    ids: &SeedIds,
    state: horae_core::types::EntryState,
) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO time_entries
           (id, org_id, user_id, project_id, task_id, spent_date, minutes, billable, state)
         VALUES ($1, $2, $3, $4, $5, '2026-09-07', 60, true, $6)",
        id,
        ids.org_id,
        ids.user_id,
        ids.project_id,
        ids.task_id,
        state as horae_core::types::EntryState,
    )
    .execute(pool)
    .await
    .unwrap();
    id
}

/// Give an entry `minutes` as a row stored before the daily limit existed,
/// which the trigger would refuse to write today.
pub async fn legacy_minutes(pool: &PgPool, entry_id: Uuid, minutes: i32) {
    let mut tx = pool.begin().await.unwrap();
    sqlx::query!("ALTER TABLE time_entries DISABLE TRIGGER time_entries_minutes_daily_cap")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query!(
        "UPDATE time_entries SET minutes = $2 WHERE id = $1",
        entry_id,
        minutes
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query!("ALTER TABLE time_entries ENABLE TRIGGER time_entries_minutes_daily_cap")
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
}

/// Raising a legacy entry's minutes is still refused by the trigger.
pub async fn assert_minutes_cannot_rise(pool: &PgPool, entry_id: Uuid) {
    let raised = sqlx::query!(
        "UPDATE time_entries SET minutes = minutes + 1 WHERE id = $1",
        entry_id
    )
    .execute(pool)
    .await
    .unwrap_err();
    assert!(
        raised
            .to_string()
            .contains("Daily hours cannot exceed 240."),
        "{raised}"
    );
}

/// The built-in profile a person's permission state holds, and whether it is
/// the Administrator state. `None` when the person has no state, or one that
/// came from a template or an individual selection.
pub async fn built_in_profile(
    pool: &PgPool,
    org_id: Uuid,
    user_id: Uuid,
) -> Option<(BuiltInProfile, bool)> {
    let mut connection = pool.acquire().await.unwrap();
    let state = crate::ops::permissions::load_person_permissions(&mut connection, org_id, user_id)
        .await
        .unwrap()?;
    match state.source {
        crate::ops::permissions::PermissionSource::BuiltIn(profile) => {
            assert_eq!(state.grants, profile.selection());
            Some((profile, state.is_administrator))
        }
        _ => None,
    }
}

/// Remove the permission state `seed` provisioned, for a test that writes the
/// person's state itself.
pub async fn clear_state(pool: &PgPool, user_id: Uuid) {
    sqlx::query!(
        "DELETE FROM person_permission_states WHERE user_id = $1",
        user_id
    )
    .execute(pool)
    .await
    .unwrap();
}

/// Adds a project member the way a team change does (organization gate, then
/// the project revision), for tests of what a membership change waits for.
/// Production membership changes go through the project editor.
pub async fn insert_assignment(
    db: &PgPool,
    org_id: Uuid,
    project_id: Uuid,
    user_id: Uuid,
    role: ProjectRole,
) -> Result<Assignment, sqlx::Error> {
    let mut tx = begin_assignment_change(db, org_id).await?;
    if !lock_project_revision(&mut tx, org_id, project_id).await? {
        return Err(sqlx::Error::RowNotFound);
    }
    sqlx::query_scalar!(
        "SELECT p.id FROM projects p JOIN users u ON u.org_id = p.org_id
         WHERE p.id = $1 AND u.id = $2 AND p.org_id = $3 FOR SHARE OF u",
        project_id,
        user_id,
        org_id,
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(sqlx::Error::RowNotFound)?;
    let id = Uuid::now_v7();
    let assignment = sqlx::query_as!(
        Assignment,
        r#"INSERT INTO assignments (id, project_id, user_id, role)
         VALUES ($1, $2, $3, $4)
         RETURNING id, project_id, user_id, role as "role: ProjectRole", rate_cents,
                   created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
        id,
        project_id,
        user_id,
        role as ProjectRole,
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(assignment)
}

/// Removes a project member like [`insert_assignment`] adds one.
pub async fn remove_assignment(
    db: &PgPool,
    org_id: Uuid,
    id: Uuid,
) -> Result<Option<Assignment>, sqlx::Error> {
    let mut tx = begin_assignment_change(db, org_id).await?;
    let project_id = sqlx::query_scalar!(
        "SELECT a.project_id FROM assignments a
         JOIN projects p ON p.id=a.project_id JOIN users u ON u.id=a.user_id
         WHERE a.id=$1 AND p.org_id=$2 AND u.org_id=$2",
        id,
        org_id,
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(project_id) = project_id else {
        tx.commit().await?;
        return Ok(None);
    };
    if !lock_project_revision(&mut tx, org_id, project_id).await? {
        tx.commit().await?;
        return Ok(None);
    }
    // Delete and capture the row atomically so the event carries its details
    // and a concurrent delete cannot double-notify.
    let removed = sqlx::query_as!(
        Assignment,
        r#"DELETE FROM assignments a USING projects p, users u
         WHERE a.id = $1 AND a.project_id = p.id AND a.user_id = u.id
           AND p.org_id = $2 AND u.org_id = $2 AND a.project_id = $3
         RETURNING a.id, a.project_id, a.user_id, a.role as "role: ProjectRole", a.rate_cents,
                   a.created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
        id,
        org_id,
        project_id,
    )
    .fetch_optional(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(removed)
}

async fn begin_assignment_change(
    db: &PgPool,
    org_id: Uuid,
) -> Result<sqlx::Transaction<'_, sqlx::Postgres>, sqlx::Error> {
    let mut tx = db.begin().await?;
    lock_organization(&mut tx, org_id, OrganizationLock::AccessChange).await?;
    Ok(tx)
}

async fn lock_project_revision(
    db: &mut sqlx::PgConnection,
    org_id: Uuid,
    project_id: Uuid,
) -> Result<bool, sqlx::Error> {
    // Child revision triggers write this parent. Permit entry/invoice FK checks
    // to finish before any dependent membership cascade waits for those writers.
    sqlx::query_scalar!(
        "SELECT id FROM projects WHERE id=$1 AND org_id=$2 FOR NO KEY UPDATE",
        project_id,
        org_id,
    )
    .fetch_optional(db)
    .await
    .map(|row| row.is_some())
}
