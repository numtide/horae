use super::*;
use crate::models::permission_audit::AuditCursor;
use crate::models::permission_editor::PermissionRequester;
use crate::server_fns::permissions::calls::audit::page;

async fn receipts(pool: &PgPool, ids: &SeedIds, count: i64) -> Vec<Uuid> {
    let mut records = Vec::new();
    for revision in 0..count {
        let command = TemplateCommand {
            request_id: Uuid::now_v7(),
            expected_access_revision: revision,
            action: TemplateAction::Create {
                name: format!("Historical template {revision}"),
                grants: BuiltInProfile::Member.selection().iter().collect(),
            },
        };
        super::super::calls::templates::execute(pool, ids.org_id, ids.user_id, &command)
            .await
            .unwrap();
        records.push(receipt_id(pool, command.request_id).await);
    }
    sqlx::query!(
        "UPDATE permission_change_receipts SET created_at='2026-09-07 12:00:00+00' WHERE org_id=$1",
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    records.sort_unstable_by(|a, b| b.cmp(a));
    records
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn history_pages_scope_before_limit_and_use_stable_exclusive_timestamp_id_bounds(
    pool: PgPool,
) {
    let ids = fixture(&pool, true).await;
    let expected = receipts(&pool, &ids, 27).await;
    let foreign = fixture(&pool, true).await;
    let foreign_ids = receipts(&pool, &foreign, 3).await;
    let first = page(&pool, ids.org_id, ids.user_id, None, None)
        .await
        .unwrap();
    assert_eq!(
        first.requester,
        PermissionRequester {
            org_id: ids.org_id,
            user_id: ids.user_id
        }
    );
    assert_eq!(
        first
            .entries
            .iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        expected[..25]
    );
    let cursor = first.next_after.unwrap();
    assert_eq!(cursor.id, expected[24]);
    let second = page(
        &pool,
        ids.org_id,
        ids.user_id,
        Some(&cursor),
        Some(first.requester),
    )
    .await
    .unwrap();
    assert_eq!(
        second
            .entries
            .iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        expected[25..]
    );
    assert!(second.next_after.is_none());
    for entry in first.entries.iter().chain(&second.entries) {
        assert_eq!(
            Some(entry.clone()),
            read(&pool, ids.org_id, ids.user_id, entry.id)
                .await
                .unwrap()
        );
        let wire = serde_json::to_value(entry).unwrap();
        for private in ["request_id", "intent", "result"] {
            assert!(wire.get(private).is_none());
        }
    }
    // Neither a foreign receipt nor a nonexistent ID must be resolved as a record.
    for id in [foreign_ids[0], Uuid::from_u128(u128::MAX)] {
        let bound = AuditCursor {
            created_at: cursor.created_at,
            id,
        };
        let bounded = page(&pool, ids.org_id, ids.user_id, Some(&bound), None)
            .await
            .unwrap();
        let expected: Vec<_> = expected
            .iter()
            .copied()
            .filter(|entry| *entry < id)
            .take(25)
            .collect();
        assert_eq!(
            bounded
                .entries
                .iter()
                .map(|entry| entry.id)
                .collect::<Vec<_>>(),
            expected
        );
    }
    let oldest = second.entries.last().unwrap();
    let end = AuditCursor {
        created_at: oldest.created_at,
        id: oldest.id,
    };
    let empty = page(&pool, ids.org_id, ids.user_id, Some(&end), None)
        .await
        .unwrap();
    assert!(empty.entries.is_empty() && empty.next_after.is_none());
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn history_page_rejects_malformed_history_without_partial_success(pool: PgPool) {
    let ids = fixture(&pool, true).await;
    let records = receipts(&pool, &ids, 3).await;
    sqlx::query!(
        "UPDATE permission_change_receipts SET audit='{}'::jsonb WHERE id=$1",
        records[1]
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        page(&pool, ids.org_id, ids.user_id, None, None).await,
        Err(e) if e == history_unavailable()
    ));
    assert!(
        read(&pool, ids.org_id, ids.user_id, records[0])
            .await
            .unwrap()
            .is_some()
    );
    assert_unlocked(&pool, &ids).await;
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn history_page_reauthorizes_empty_and_continuation_requests_and_binds_requester(
    pool: PgPool,
) {
    let ids = fixture(&pool, true).await;
    let empty = page(&pool, ids.org_id, ids.user_id, None, None)
        .await
        .unwrap();
    assert!(empty.entries.is_empty() && empty.next_after.is_none());
    let nonadmin = person(&pool, ids.org_id, false, BuiltInProfile::Administrator).await;
    for expected in [
        PermissionRequester {
            org_id: Uuid::now_v7(),
            user_id: ids.user_id,
        },
        PermissionRequester {
            org_id: ids.org_id,
            user_id: nonadmin,
        },
    ] {
        assert!(matches!(
            page(&pool, ids.org_id, ids.user_id, None, Some(expected)).await,
            Err(e) if e == forbidden()
        ));
    }
    assert!(matches!(
        page(&pool, ids.org_id, nonadmin, None, None).await,
        Err(e) if e == forbidden()
    ));
    let records = receipts(&pool, &ids, 26).await;
    let first = page(&pool, ids.org_id, ids.user_id, None, None)
        .await
        .unwrap();
    assert_eq!(first.entries[0].id, records[0]);
    sqlx::query!(
        "UPDATE person_permission_states SET is_administrator=false WHERE user_id=$1",
        ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        page(
            &pool,
            ids.org_id,
            ids.user_id,
            first.next_after.as_ref(),
            Some(first.requester)
        )
        .await,
        Err(e) if e == forbidden()
    ));
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn history_page_waits_for_revocation_commit_or_rollback_at_each_gate(pool: PgPool) {
    for direct_activity in [false, true] {
        for commit in [false, true] {
            let ids = fixture(&pool, true).await;
            receipts(&pool, &ids, 1).await;
            let mut change = pool.begin().await.unwrap();
            if direct_activity {
                sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
                    .execute(&mut *change)
                    .await
                    .unwrap();
            } else {
                crate::db::lock_organization(
                    &mut change,
                    ids.org_id,
                    crate::db::OrganizationLock::AccessChange,
                )
                .await
                .unwrap();
                sqlx::query!(
                    "UPDATE person_permission_states SET is_administrator=false WHERE user_id=$1",
                    ids.user_id
                )
                .execute(&mut *change)
                .await
                .unwrap();
            }
            let blocker = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
                .fetch_one(&mut *change)
                .await
                .unwrap();
            let db = pool.clone();
            let mut reader = tokio::task::JoinSet::new();
            reader.spawn(async move { page(&db, ids.org_id, ids.user_id, None, None).await });
            wait_for_blocked(&pool, blocker).await;
            if commit {
                change.commit().await.unwrap();
            } else {
                change.rollback().await.unwrap();
            }
            let result = reader.join_next().await.unwrap().unwrap();
            if commit && direct_activity {
                // The deactivated session is signed out.
                assert!(matches!(result, Err(e) if e == OpError::unauthenticated()));
            } else if commit {
                assert!(matches!(result, Err(e) if e == forbidden()));
            } else {
                assert_eq!(result.unwrap().entries.len(), 1);
            }
        }
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn admitted_history_page_finishes_before_revocation_at_each_gate(pool: PgPool) {
    for direct_activity in [false, true] {
        let ids = fixture(&pool, true).await;
        let records = receipts(&pool, &ids, 1).await;
        let mut hold = pool.begin().await.unwrap();
        sqlx::query!("LOCK TABLE permission_change_receipts IN ACCESS EXCLUSIVE MODE")
            .execute(&mut *hold)
            .await
            .unwrap();
        let holder = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
            .fetch_one(&mut *hold)
            .await
            .unwrap();
        let db = pool.clone();
        let mut reader = tokio::task::JoinSet::new();
        reader.spawn(async move { page(&db, ids.org_id, ids.user_id, None, None).await });
        wait_for_blocked(&pool, holder).await;
        let reader_pid=sqlx::query_scalar!("SELECT pid FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid))",holder).fetch_one(&pool).await.unwrap().unwrap();
        let db = pool.clone();
        let mut writer = tokio::task::JoinSet::new();
        writer.spawn(async move {
            let mut tx = db.begin().await.unwrap();
            if direct_activity {
                sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
                    .execute(&mut *tx)
                    .await
                    .unwrap();
            } else {
                crate::db::lock_organization(
                    &mut tx,
                    ids.org_id,
                    crate::db::OrganizationLock::AccessChange,
                )
                .await
                .unwrap();
                sqlx::query!(
                    "UPDATE person_permission_states SET is_administrator=false WHERE user_id=$1",
                    ids.user_id
                )
                .execute(&mut *tx)
                .await
                .unwrap();
            }
            tx.commit().await.unwrap();
        });
        wait_for_blocked(&pool, reader_pid).await;
        hold.commit().await.unwrap();
        assert_eq!(
            reader.join_next().await.unwrap().unwrap().unwrap().entries[0].id,
            records[0]
        );
        writer.join_next().await.unwrap().unwrap();
        // A deactivated session is signed out; a revoked identity is refused.
        let refused = if direct_activity {
            OpError::unauthenticated()
        } else {
            forbidden()
        };
        assert!(matches!(
            page(&pool, ids.org_id, ids.user_id, None, None).await,
            Err(e) if e == refused
        ));
    }
}

#[sqlx::test(migrator = "crate::db::MIGRATOR")]
#[serial_test::serial]
async fn cancelling_history_page_releases_both_authority_gates(pool: PgPool) {
    let ids = fixture(&pool, true).await;
    receipts(&pool, &ids, 1).await;
    let mut hold = pool.begin().await.unwrap();
    sqlx::query!("LOCK TABLE permission_change_receipts IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *hold)
        .await
        .unwrap();
    let holder = sqlx::query_scalar!("SELECT pg_backend_pid() AS \"pid!\"")
        .fetch_one(&mut *hold)
        .await
        .unwrap();
    let db = pool.clone();
    let mut reader = tokio::task::JoinSet::new();
    reader.spawn(async move { page(&db, ids.org_id, ids.user_id, None, None).await });
    wait_for_blocked(&pool, holder).await;
    reader.abort_all();
    assert!(
        reader
            .join_next()
            .await
            .unwrap()
            .unwrap_err()
            .is_cancelled()
    );
    hold.rollback().await.unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        let mut tx = pool.begin().await.unwrap();
        crate::db::lock_organization(
            &mut tx,
            ids.org_id,
            crate::db::OrganizationLock::AccessChange,
        )
        .await
        .unwrap();
        sqlx::query!("UPDATE users SET active=false WHERE id=$1", ids.user_id)
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.rollback().await.unwrap();
    })
    .await
    .unwrap();
    assert_unlocked(&pool, &ids).await;
}

mod filters {
    use super::*;
    use crate::models::permission_audit::{AuditFilter, AuditKind, AuditPeriod};
    use crate::server_fns::permissions::calls::audit::filtered;

    /// Stores a receipt directly, `days_ago` days before now.
    async fn receipt(
        pool: &PgPool,
        org: Uuid,
        actor: Uuid,
        audit: serde_json::Value,
        days_ago: i32,
    ) -> Uuid {
        let id = Uuid::now_v7();
        sqlx::query!(
            "INSERT INTO permission_change_receipts
               (id, org_id, actor_user_id, request_id, format_version, intent, result, audit, created_at)
             VALUES ($1, $2, $3, $4, 1, '{}', '{}', $5, now() - make_interval(days => $6))",
            id,
            org,
            actor,
            Uuid::now_v7(),
            audit,
            days_ago
        )
        .execute(pool)
        .await
        .unwrap();
        id
    }

    fn person_change(user: Uuid) -> serde_json::Value {
        let grants = json!(BuiltInProfile::Member.selection());
        let snapshot = |revision| {
            json!({"grants": grants, "is_administrator": false,
                "source": {"kind": "individual"}, "revision": revision})
        };
        json!({"user_id": user, "previous_access_revision": 0, "access_revision": 1,
            "change": {"catalog_version": 1, "before": snapshot(0), "after": snapshot(1),
            "removed_projects": [], "removed_people": []}})
    }

    fn manager_added(project: Uuid, manager: Uuid) -> serde_json::Value {
        json!({"project_id": project, "previous_access_revision": 0, "access_revision": 1,
            "change": {"added": [{"id": Uuid::now_v7(), "manager_id": manager, "revision": 0}],
            "removed": []}})
    }

    fn manager_removed(project: Uuid, manager: Uuid) -> serde_json::Value {
        json!({"project_id": project, "previous_access_revision": 0, "access_revision": 1,
            "change": {"added": [],
            "removed": [{"id": Uuid::now_v7(), "manager_id": manager, "revision": 0}]}})
    }

    /// A change to `user` that also ended their management of `managed`.
    fn management_ended(user: Uuid, managed: Uuid) -> serde_json::Value {
        let mut change = person_change(user);
        change["change"]["removed_people"] =
            json!([{"id": Uuid::now_v7(), "subject_id": managed, "revision": 0}]);
        change
    }

    fn profile_created() -> serde_json::Value {
        json!({"previous_access_revision": 0, "access_revision": 1, "before": null,
            "after": {"id": Uuid::now_v7(), "name": "Readers", "catalog_version": 1,
            "grants": BuiltInProfile::Member.selection(), "revision": 0},
            "detached_people": []})
    }

    /// A deleted profile whose person keeps its permissions as their own.
    fn profile_deleted_detaching(person: Uuid) -> serde_json::Value {
        let template = Uuid::now_v7();
        let grants = json!(BuiltInProfile::Member.selection());
        json!({"previous_access_revision": 0, "access_revision": 1, "after": null,
            "before": {"id": template, "name": "Readers", "catalog_version": 1,
            "grants": grants, "revision": 0},
            "detached_people": [{"user_id": person, "catalog_version": 1, "grants": grants,
            "is_administrator": false, "previous_template_id": template,
            "previous_applied_revision": 0, "previous_revision": 0, "revision": 1}]})
    }

    /// Newest first, `days` ago. B is `other`; each receipt reaches B through
    /// a different part of its document, or not at all (`profile`).
    struct History {
        ids: SeedIds,
        other: Uuid,
        /// B, as the actor, changed the admin (1).
        b_acted: Uuid,
        /// The admin changed B (2).
        b_changed: Uuid,
        /// B stopped managing a project (3).
        b_unmanaged: Uuid,
        /// The admin stopped managing B (4).
        b_unlinked: Uuid,
        /// A profile B had was deleted, and B kept its permissions (5).
        b_detached: Uuid,
        /// B became a manager of a project (40).
        b_managed: Uuid,
        /// A profile with nobody on it (100).
        profile: Uuid,
    }

    async fn history(pool: &PgPool) -> History {
        let ids = fixture(pool, true).await;
        let other = super::super::person(pool, ids.org_id, false, BuiltInProfile::Member).await;
        let admin = ids.user_id;
        let b_acted = receipt(pool, ids.org_id, other, person_change(admin), 1).await;
        let b_changed = receipt(pool, ids.org_id, admin, person_change(other), 2).await;
        let b_unmanaged = receipt(
            pool,
            ids.org_id,
            admin,
            manager_removed(ids.project_id, other),
            3,
        )
        .await;
        let b_unlinked = receipt(pool, ids.org_id, admin, management_ended(admin, other), 4).await;
        let b_detached =
            receipt(pool, ids.org_id, admin, profile_deleted_detaching(other), 5).await;
        let b_managed = receipt(
            pool,
            ids.org_id,
            admin,
            manager_added(ids.project_id, other),
            40,
        )
        .await;
        let profile = receipt(pool, ids.org_id, admin, profile_created(), 100).await;
        // Another organization's history never shows.
        let foreign = fixture(pool, true).await;
        receipt(
            pool,
            foreign.org_id,
            foreign.user_id,
            person_change(other),
            1,
        )
        .await;
        History {
            ids,
            other,
            b_acted,
            b_changed,
            b_unmanaged,
            b_unlinked,
            b_detached,
            b_managed,
            profile,
        }
    }

    async fn shown(pool: &PgPool, ids: &SeedIds, filter: AuditFilter) -> Vec<Uuid> {
        filtered(pool, ids.org_id, ids.user_id, &filter, None, None)
            .await
            .unwrap()
            .entries
            .iter()
            .map(|entry| entry.id)
            .collect()
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    #[serial_test::serial]
    async fn each_filter_narrows_the_history_and_they_combine(pool: PgPool) {
        let h = history(&pool).await;
        let kind = |kind| AuditFilter {
            kind: Some(kind),
            ..AuditFilter::default()
        };
        let period = |period| AuditFilter {
            period: Some(period),
            ..AuditFilter::default()
        };
        let b = |kind| AuditFilter {
            kind: Some(kind),
            person: Some(h.other),
            period: None,
        };
        let everything = vec![
            h.b_acted,
            h.b_changed,
            h.b_unmanaged,
            h.b_unlinked,
            h.b_detached,
            h.b_managed,
            h.profile,
        ];
        for (filter, expected) in [
            (AuditFilter::default(), everything.clone()),
            (
                kind(AuditKind::Person),
                vec![h.b_acted, h.b_changed, h.b_unlinked],
            ),
            (kind(AuditKind::Managers), vec![h.b_unmanaged, h.b_managed]),
            (kind(AuditKind::Profile), vec![h.b_detached, h.profile]),
            // A person's history: what they did and what was done to them,
            // whichever part of the receipt names them.
            (AuditFilter::person(h.other), everything[..6].to_vec()),
            // Actor, `user_id` and `removed_people[].subject_id`.
            (
                b(AuditKind::Person),
                vec![h.b_acted, h.b_changed, h.b_unlinked],
            ),
            // `removed[].manager_id` and `added[].manager_id`.
            (b(AuditKind::Managers), vec![h.b_unmanaged, h.b_managed]),
            // `detached_people[].user_id`; the empty profile names nobody.
            (b(AuditKind::Profile), vec![h.b_detached]),
            (AuditFilter::person(h.ids.user_id), everything.clone()),
            (AuditFilter::person(Uuid::now_v7()), vec![]),
            (period(AuditPeriod::Days7), everything[..5].to_vec()),
            (period(AuditPeriod::Days90), everything[..6].to_vec()),
            (
                AuditFilter {
                    kind: Some(AuditKind::Managers),
                    person: Some(h.other),
                    period: Some(AuditPeriod::Days30),
                },
                vec![h.b_unmanaged],
            ),
        ] {
            assert_eq!(shown(&pool, &h.ids, filter).await, expected, "{filter:?}");
        }
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    #[serial_test::serial]
    async fn a_page_names_the_people_projects_and_profiles_it_refers_to(pool: PgPool) {
        let h = history(&pool).await;
        let page = filtered(
            &pool,
            h.ids.org_id,
            h.ids.user_id,
            &AuditFilter::default(),
            None,
            None,
        )
        .await
        .unwrap();
        assert_eq!(page.person_name(h.other), Some("Audit person"));
        assert!(page.person_name(h.ids.user_id).is_some());
        assert!(page.project_name(h.ids.project_id).is_some());
        // Only this organization's people are offered by the person filter.
        assert_eq!(page.people.len(), 2);
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    #[serial_test::serial]
    async fn a_filtered_history_pages_with_its_cursor(pool: PgPool) {
        let ids = fixture(&pool, true).await;
        let other = super::super::person(&pool, ids.org_id, false, BuiltInProfile::Member).await;
        let mut expected = Vec::new();
        for day in 0..27 {
            expected.push(receipt(&pool, ids.org_id, ids.user_id, person_change(other), day).await);
            // Receipts of another kind in between must not count toward a page.
            receipt(&pool, ids.org_id, ids.user_id, profile_created(), day).await;
        }
        let filter = AuditFilter {
            kind: Some(AuditKind::Person),
            ..AuditFilter::default()
        };
        let first = filtered(&pool, ids.org_id, ids.user_id, &filter, None, None)
            .await
            .unwrap();
        assert_eq!(first.entries.len(), 25);
        let cursor = first.next_after.unwrap();
        let second = filtered(
            &pool,
            ids.org_id,
            ids.user_id,
            &filter,
            Some(&cursor),
            Some(first.requester),
        )
        .await
        .unwrap();
        assert!(second.next_after.is_none());
        // Only the first page carries the people.
        assert_eq!(first.people.len(), 2);
        assert!(second.people.is_empty());
        let ids_shown: Vec<_> = first
            .entries
            .iter()
            .chain(&second.entries)
            .map(|entry| entry.id)
            .collect();
        assert_eq!(ids_shown, expected);
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    #[serial_test::serial]
    async fn a_member_is_refused_whatever_the_filter(pool: PgPool) {
        let ids = fixture(&pool, false).await;
        receipt(
            &pool,
            ids.org_id,
            ids.user_id,
            person_change(ids.user_id),
            1,
        )
        .await;
        // Not even the history of their own access.
        for filter in [AuditFilter::default(), AuditFilter::person(ids.user_id)] {
            assert_eq!(
                filtered(&pool, ids.org_id, ids.user_id, &filter, None, None)
                    .await
                    .unwrap_err(),
                forbidden()
            );
        }
    }

    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    #[serial_test::serial]
    async fn days_and_times_are_in_the_viewers_zone(pool: PgPool) {
        let ids = fixture(&pool, true).await;
        let id = receipt(&pool, ids.org_id, ids.user_id, profile_created(), 0).await;
        sqlx::query!(
            "UPDATE permission_change_receipts SET created_at='2026-09-07 23:30:00+00' WHERE id=$1",
            id
        )
        .execute(&pool)
        .await
        .unwrap();
        let local =
            |page: &crate::models::permission_audit::AuditPage| page.entries[0].local.to_string();
        // The person's own zone puts it on the next day.
        sqlx::query!(
            "UPDATE users SET timezone='Europe/Madrid' WHERE id=$1",
            ids.user_id
        )
        .execute(&pool)
        .await
        .unwrap();
        let madrid = page(&pool, ids.org_id, ids.user_id, None, None)
            .await
            .unwrap();
        assert_eq!(madrid.time_zone, "Europe/Madrid");
        assert_eq!(local(&madrid), "2026-09-08 01:30:00");
        // Without one, the organization's zone decides.
        sqlx::query!("UPDATE users SET timezone=NULL WHERE id=$1", ids.user_id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query!(
            "UPDATE organizations SET timezone='America/New_York' WHERE id=$1",
            ids.org_id
        )
        .execute(&pool)
        .await
        .unwrap();
        let new_york = page(&pool, ids.org_id, ids.user_id, None, None)
            .await
            .unwrap();
        assert_eq!(new_york.time_zone, "America/New_York");
        assert_eq!(local(&new_york), "2026-09-07 19:30:00");
    }

    /// Madrid leaves summer time at 01:00 UTC on 25 October 2026 (03:00 CEST
    /// becomes 02:00 CET): each instant takes the offset in force at that
    /// instant, not the offset of today.
    #[sqlx::test(migrator = "crate::db::MIGRATOR")]
    #[serial_test::serial]
    async fn local_days_follow_the_offset_in_force_across_a_dst_change(pool: PgPool) {
        let ids = fixture(&pool, true).await;
        sqlx::query!(
            "UPDATE users SET timezone='Europe/Madrid' WHERE id=$1",
            ids.user_id
        )
        .execute(&pool)
        .await
        .unwrap();
        let instants = [
            "2026-10-25 23:30:00+00",
            "2026-10-25 01:30:00+00",
            "2026-10-25 00:30:00+00",
            "2026-10-24 22:30:00+00",
        ];
        for at in instants {
            let id = receipt(&pool, ids.org_id, ids.user_id, profile_created(), 0).await;
            sqlx::query!(
                "UPDATE permission_change_receipts SET created_at=$2::text::timestamptz WHERE id=$1",
                id,
                at
            )
            .execute(&pool)
            .await
            .unwrap();
        }
        let page = page(&pool, ids.org_id, ids.user_id, None, None)
            .await
            .unwrap();
        let local: Vec<_> = page
            .entries
            .iter()
            .map(|entry| entry.local.to_string())
            .collect();
        assert_eq!(
            local,
            [
                // CET (+1): after midnight on the 26th.
                "2026-10-26 00:30:00",
                // The repeated hour: 02:30 CET, then 02:30 CEST an hour earlier.
                "2026-10-25 02:30:00",
                "2026-10-25 02:30:00",
                // CEST (+2): already the 25th.
                "2026-10-25 00:30:00",
            ]
        );
    }
}
