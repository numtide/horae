//! Approvals by built-in profile, on stored permission state alone.

use horae_core::permissions::catalog::BuiltInProfile;
use horae_core::types::EntryState;
use sqlx::PgPool;
use uuid::Uuid;

use super::*;
use crate::ops::testing::{Recorder, seed_profile};
use crate::ops::{self, Caller};
use crate::server_fns::test_seed::{SeedIds, time_entry};

/// Adds an active person with `profile`'s stored state to `ids`' organization,
/// sharing its project and task.
pub(crate) async fn add_person(pool: &PgPool, ids: &SeedIds, profile: BuiltInProfile) -> SeedIds {
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
    crate::server_fns::provisioning::provision(&mut connection, ids.org_id, user_id, profile)
        .await
        .unwrap();
    SeedIds { user_id, ..*ids }
}

/// Makes `manager` the manager of `person`.
pub(crate) async fn manage(pool: &PgPool, manager: &SeedIds, person: &SeedIds) {
    sqlx::query!(
        "INSERT INTO person_management_assignments (id, org_id, manager_id, managed_user_id)
         VALUES ($1, $2, $3, $4)",
        Uuid::now_v7(),
        manager.org_id,
        manager.user_id,
        person.user_id,
    )
    .execute(pool)
    .await
    .unwrap();
}

const WEEK: &str = "2026-09-07";

fn week() -> NaiveDate {
    WEEK.parse().unwrap()
}

fn caller(ids: &SeedIds) -> Caller {
    Caller::session(ids.org_id, ids.user_id)
}

/// Submits `ids`' week with one open entry in it.
async fn submitted(pool: &PgPool, ids: &SeedIds) -> Approval {
    time_entry(pool, ids, EntryState::Open).await;
    let mut write = ops::begin(pool, caller(ids)).await.unwrap();
    let result = submit_week(&mut write.tx, &write.actor, week()).await;
    write.commit(result, &Recorder::default()).await.unwrap()
}

async fn list_as(pool: &PgPool, ids: &SeedIds) -> Result<Vec<ApprovalSummary>, OpError> {
    let mut read = ops::begin_read(pool, caller(ids)).await?;
    let result = list(&mut read.tx, &read.actor, None, week()).await;
    read.finish(result).await
}

async fn approve_as(pool: &PgPool, ids: &SeedIds, approvals: &[Uuid]) -> Result<usize, OpError> {
    let mut write = ops::begin(pool, caller(ids)).await?;
    let result = approve(&mut write.tx, &write.actor, approvals).await;
    Ok(write.commit(result, &Recorder::default()).await?.len())
}

async fn reopen_as(pool: &PgPool, ids: &SeedIds, approval: Uuid) -> Result<(), OpError> {
    let mut write = ops::begin(pool, caller(ids)).await?;
    let result = reopen(&mut write.tx, &write.actor, approval).await;
    write.commit(result, &Recorder::default()).await.map(drop)
}

async fn state(pool: &PgPool, approval: Uuid) -> Option<EntryState> {
    sqlx::query_scalar!(
        r#"SELECT state as "state: EntryState" FROM approvals WHERE id = $1"#,
        approval
    )
    .fetch_optional(pool)
    .await
    .unwrap()
}

#[derive(Debug, Clone, Copy)]
enum Who {
    Profile(BuiltInProfile),
    /// A Project Manager who manages the submitter, or nobody.
    ProjectManager {
        manages: bool,
    },
}

/// Who may list, approve and reopen a teammate's week, and what everyone else
/// is told.
const MATRIX: [(Who, Option<&str>); 5] = [
    (Who::Profile(BuiltInProfile::Administrator), None),
    (Who::Profile(BuiltInProfile::ExecutiveManager), None),
    (Who::ProjectManager { manages: true }, None),
    (Who::ProjectManager { manages: false }, Some(NOT_COVERED)),
    (
        Who::Profile(BuiltInProfile::Member),
        Some(APPROVAL_REQUIRED),
    ),
];

/// The approver and a Member teammate whose week is submitted.
async fn fixture(pool: &PgPool, who: Who) -> (SeedIds, SeedIds, Approval) {
    let profile = match who {
        Who::Profile(profile) => profile,
        Who::ProjectManager { .. } => BuiltInProfile::ProjectManager,
    };
    let approver = seed_profile(pool, profile).await;
    let teammate = add_person(pool, &approver, BuiltInProfile::Member).await;
    if let Who::ProjectManager { manages: true } = who {
        manage(pool, &approver, &teammate).await;
    }
    let approval = submitted(pool, &teammate).await;
    (approver, teammate, approval)
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn listing_a_teammates_week_follows_the_profile(pool: PgPool) {
    for (who, refusal) in MATRIX {
        let (approver, _, approval) = fixture(&pool, who).await;
        let result = list_as(&pool, &approver).await;
        match refusal {
            None => {
                let rows = result.unwrap();
                assert_eq!(
                    rows.iter().map(|row| row.approval.id).collect::<Vec<_>>(),
                    [approval.id],
                    "{who:?}"
                );
            }
            Some(message) => {
                assert_eq!(result.unwrap_err(), OpError::forbidden(message), "{who:?}")
            }
        }
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn approving_a_teammates_week_follows_the_profile(pool: PgPool) {
    for (who, refusal) in MATRIX {
        let (approver, _, approval) = fixture(&pool, who).await;
        let result = approve_as(&pool, &approver, &[approval.id]).await;
        match refusal {
            None => {
                assert_eq!(result, Ok(1), "{who:?}");
                assert_eq!(state(&pool, approval.id).await, Some(EntryState::Approved));
            }
            Some(message) => {
                assert_eq!(result, Err(OpError::forbidden(message)), "{who:?}");
                assert_eq!(state(&pool, approval.id).await, Some(EntryState::Submitted));
            }
        }
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn rejecting_and_reopening_a_teammates_week_follows_the_profile(pool: PgPool) {
    for (who, refusal) in MATRIX {
        let (approver, teammate, approval) = fixture(&pool, who).await;
        let result = reopen_as(&pool, &approver, approval.id).await;
        match refusal {
            None => {
                assert_eq!(result, Ok(()), "{who:?}");
                assert_eq!(state(&pool, approval.id).await, None);
                // An approved week reopens too.
                let again = submitted(&pool, &teammate).await;
                approve_as(&pool, &approver, &[again.id]).await.unwrap();
                assert_eq!(
                    reopen_as(&pool, &approver, again.id).await,
                    Ok(()),
                    "{who:?}"
                );
                assert_eq!(state(&pool, again.id).await, None);
            }
            Some(message) => {
                assert_eq!(result, Err(OpError::forbidden(message)), "{who:?}");
                assert_eq!(state(&pool, approval.id).await, Some(EntryState::Submitted));
            }
        }
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn every_profile_submits_its_own_week(pool: PgPool) {
    for &profile in BuiltInProfile::ALL {
        let ids = seed_profile(&pool, profile).await;
        let approval = submitted(&pool, &ids).await;
        assert_eq!(
            (approval.user_id, approval.state),
            (ids.user_id, EntryState::Submitted),
            "{profile:?}"
        );
    }
}

/// A managed approver's request covers the whole week or nothing: one week
/// outside their people denies the list and the approval, and changes nothing.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn managed_approval_is_denied_whole_when_one_week_is_outside_it(pool: PgPool) {
    let (approver, _, managed) = fixture(&pool, Who::ProjectManager { manages: true }).await;
    let outsider = add_person(&pool, &approver, BuiltInProfile::Member).await;
    let unmanaged = submitted(&pool, &outsider).await;

    assert_eq!(
        list_as(&pool, &approver).await.unwrap_err(),
        OpError::forbidden(NOT_COVERED)
    );
    assert_eq!(
        approve_as(&pool, &approver, &[managed.id, unmanaged.id]).await,
        Err(OpError::forbidden(NOT_COVERED))
    );
    for approval in [managed.id, unmanaged.id] {
        assert_eq!(state(&pool, approval).await, Some(EntryState::Submitted));
    }
    // The managed week alone is the approver's to decide.
    assert_eq!(approve_as(&pool, &approver, &[managed.id]).await, Ok(1));
}

/// Only the stored Administrator identity decides its own week; holding every
/// approval grant is not enough.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn only_an_administrator_decides_their_own_week(pool: PgPool) {
    for (profile, allowed) in [
        (BuiltInProfile::Administrator, true),
        (BuiltInProfile::ExecutiveManager, false),
        (BuiltInProfile::ProjectManager, false),
    ] {
        let ids = seed_profile(&pool, profile).await;
        let own = submitted(&pool, &ids).await;
        let approved = approve_as(&pool, &ids, &[own.id]).await;
        let reopened = reopen_as(&pool, &ids, own.id).await;
        if allowed {
            assert_eq!(approved, Ok(1), "{profile:?}");
            assert_eq!(reopened, Ok(()), "{profile:?}");
        } else {
            let refusal = OpError::forbidden(OWN_SUBMISSION_FORBIDDEN);
            assert_eq!(approved, Err(refusal.clone()), "{profile:?}");
            assert_eq!(reopened, Err(refusal), "{profile:?}");
            assert_eq!(state(&pool, own.id).await, Some(EntryState::Submitted));
        }
    }

    let ids = seed_profile(&pool, BuiltInProfile::Administrator).await;
    let own = submitted(&pool, &ids).await;
    crate::ops::testing::provision(
        &pool,
        &ids,
        horae_core::permissions::catalog::Permission::ALL,
        false,
    )
    .await;
    assert_eq!(
        approve_as(&pool, &ids, &[own.id]).await,
        Err(OpError::forbidden(OWN_SUBMISSION_FORBIDDEN))
    );
}

/// The submission and decisions announce themselves once committed, with the
/// week's tracked minutes.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn decisions_dispatch_their_events_after_commit(pool: PgPool) {
    let (approver, teammate, first) =
        fixture(&pool, Who::Profile(BuiltInProfile::ExecutiveManager)).await;
    reopen_as(&pool, &approver, first.id).await.unwrap();

    let events = Recorder::default();
    let mut write = ops::begin(&pool, caller(&teammate)).await.unwrap();
    let result = submit_week(&mut write.tx, &write.actor, week()).await;
    let approval = write.commit(result, &events).await.unwrap();
    let mut write = ops::begin(&pool, caller(&approver)).await.unwrap();
    let result = approve(&mut write.tx, &write.actor, &[approval.id]).await;
    write.commit(result, &events).await.unwrap();
    let mut write = ops::begin(&pool, caller(&approver)).await.unwrap();
    let result = reopen(&mut write.tx, &write.actor, approval.id).await;
    write.commit(result, &events).await.unwrap();

    let announced: Vec<(&str, Uuid, i32)> = events
        .take()
        .iter()
        .map(|event| match event {
            AppEvent::TimesheetSubmitted { submission, .. } => {
                ("submitted", submission.id, submission.total_minutes)
            }
            AppEvent::SubmissionApproved { submission, .. } => {
                ("approved", submission.id, submission.total_minutes)
            }
            AppEvent::SubmissionRejected { submission, .. } => {
                ("rejected", submission.id, submission.total_minutes)
            }
            other => panic!("unexpected event {other:?}"),
        })
        .collect();
    assert_eq!(
        announced,
        [
            ("submitted", approval.id, 60),
            ("approved", approval.id, 60),
            ("rejected", approval.id, 60),
        ]
    );
}
