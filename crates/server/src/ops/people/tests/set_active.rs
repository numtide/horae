//! `set_active`: archiving and restoring people.

use horae_core::permissions::catalog::BuiltInProfile;
use sqlx::PgPool;
use uuid::Uuid;

use super::super::{ARCHIVE_SELF, set_active};
use crate::models::user::UserListItem;
use crate::ops::testing::Recorder;
use crate::ops::{Caller, OpError};
use crate::testing::{SeedIds, add_person, seed};

async fn change(
    pool: &PgPool,
    actor: &SeedIds,
    user_id: Uuid,
    active: bool,
) -> Result<UserListItem, OpError> {
    let mut write = crate::ops::begin(pool, Caller::session(actor.org_id, actor.user_id)).await?;
    let result = set_active(&mut write.tx, &write.actor, user_id, active).await;
    write.commit(result, &Recorder::default()).await
}

/// Archiving yourself is refused even when another administrator remains, so
/// the last-administrator rule is not what stops it.
#[sqlx::test(migrator = "crate::db::MIGRATOR")]
async fn nobody_archives_their_own_account(pool: PgPool) {
    let ids = seed(&pool, BuiltInProfile::Administrator).await;
    let other = add_person(&pool, &ids, BuiltInProfile::Administrator).await;
    let people_admin = add_person(&pool, &ids, BuiltInProfile::PeopleAdmin).await;

    for actor in [&ids, &other, &people_admin] {
        assert_eq!(
            change(&pool, actor, actor.user_id, false).await,
            Err(OpError::forbidden(ARCHIVE_SELF)),
        );
    }
    let active = sqlx::query_scalar!(
        r#"SELECT COUNT(*) AS "count!" FROM users WHERE org_id = $1 AND active"#,
        ids.org_id,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(active, 3);

    // Restoring yourself is a no-op on an active account and stays allowed;
    // archiving someone else still works.
    assert!(change(&pool, &ids, ids.user_id, true).await.unwrap().active);
    assert!(
        !change(&pool, &ids, other.user_id, false)
            .await
            .unwrap()
            .active
    );
}
