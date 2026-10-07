use super::*;

async fn snapshot(pool: &PgPool, org: Uuid) -> Option<serde_json::Value> {
    sqlx::query_scalar!(
        "SELECT jsonb_agg(to_jsonb(e) ORDER BY id) FROM time_entries e WHERE org_id=$1",
        org
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn grants(pool: &PgPool, f: &Fixture, permissions: &[Permission]) {
    let stored: Vec<String> = serde_json::from_value(
        serde_json::to_value(PermissionSelection::new(permissions)).unwrap(),
    )
    .unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants=$2 WHERE user_id=$1",
        f.ids.user_id,
        &stored
    )
    .execute(pool)
    .await
    .unwrap();
}

fn intents(f: &Fixture, id: Uuid) -> Vec<TimesheetCommand> {
    vec![
        TimesheetCommand::Create { entry: f.input() },
        TimesheetCommand::Update {
            entry_id: id,
            entry: f.input(),
        },
        TimesheetCommand::Delete {
            entry_ids: vec![id],
        },
        TimesheetCommand::StartTimer {
            project_id: f.ids.project_id,
            task_id: f.ids.task_id,
            notes: None,
        },
        TimesheetCommand::StopTimer { entry_id: id },
        TimesheetCommand::Reschedule {
            entry_id: id,
            spent_date: f.input().spent_date,
            start_minute: 600,
            minutes: 90,
        },
        TimesheetCommand::Reorder {
            spent_date: f.input().spent_date,
            entry_ids: vec![id],
        },
    ]
}

#[sqlx::test(migrations = "./migrations")]
async fn all_intents_reject_read_only_inactive_and_stale_identities_atomically(pool: PgPool) {
    let f = fixture(&pool).await;
    let id = f.entry(&pool).await;
    let before = snapshot(&pool, f.ids.org_id).await;
    grants(&pool, &f, &[Permission::TimeReadAll]).await;
    for command in intents(&f, id) {
        assert!(f.execute(&pool, command).await.is_err());
    }
    assert!(
        tracking(&pool, f.ids.org_id, f.ids.user_id, &f.context)
            .await
            .unwrap()
            .is_empty()
    );
    grants(&pool, &f, &[Permission::TimeWriteAll]).await;
    for user in [f.ids.user_id, f.context.subject_id] {
        sqlx::query!("UPDATE users SET active=false WHERE id=$1", user)
            .execute(&pool)
            .await
            .unwrap();
        for command in intents(&f, id) {
            assert!(f.execute(&pool, command).await.is_err());
        }
        sqlx::query!("UPDATE users SET active=true WHERE id=$1", user)
            .execute(&pool)
            .await
            .unwrap();
    }
    for changed in [
        TimesheetWriteContext {
            expected_policy: TimesheetPolicy::LegacyOwn,
            ..f.context
        },
        TimesheetWriteContext {
            expected_requester: PermissionRequester {
                user_id: Uuid::now_v7(),
                ..f.context.expected_requester
            },
            ..f.context
        },
        TimesheetWriteContext {
            expected_requester: PermissionRequester {
                org_id: Uuid::now_v7(),
                ..f.context.expected_requester
            },
            ..f.context
        },
        TimesheetWriteContext {
            subject_id: Uuid::now_v7(),
            ..f.context
        },
    ] {
        for command in intents(&f, id) {
            assert!(
                apply(&pool, f.ids.org_id, f.ids.user_id, &changed, command)
                    .await
                    .is_err()
            );
        }
    }
    assert_eq!(snapshot(&pool, f.ids.org_id).await, before);
}

#[sqlx::test(migrations = "./migrations")]
async fn project_management_covers_only_its_project_and_checks_both_move_ends(pool: PgPool) {
    let f = fixture(&pool).await;
    sqlx::query!(
        "DELETE FROM person_management_assignments WHERE manager_id=$1",
        f.ids.user_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let id = f.entry(&pool).await;
    assert!(
        f.execute(
            &pool,
            TimesheetCommand::Update {
                entry_id: id,
                entry: f.input()
            }
        )
        .await
        .is_err()
    );
    sqlx::query!("INSERT INTO project_management_assignments (id,org_id,project_id,manager_id) VALUES ($1,$2,$3,$4)",Uuid::now_v7(),f.ids.org_id,f.ids.project_id,f.ids.user_id).execute(&pool).await.unwrap();
    f.execute(
        &pool,
        TimesheetCommand::Update {
            entry_id: id,
            entry: f.input(),
        },
    )
    .await
    .unwrap();
    let other = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO projects (id,org_id,client_id,name,currency) VALUES ($1,$2,$3,'Other','EUR')",
        other,
        f.ids.org_id,
        f.ids.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        other,
        f.context.subject_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO project_tasks (project_id,task_id,billable) VALUES ($1,$2,true)",
        other,
        f.ids.task_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let destination = TimesheetEntryInput {
        project_id: other,
        ..f.input()
    };
    assert!(
        f.execute(
            &pool,
            TimesheetCommand::Update {
                entry_id: id,
                entry: destination.clone()
            }
        )
        .await
        .is_err()
    );
    grants(&pool, &f, &[Permission::TimeWriteAll]).await;
    f.execute(
        &pool,
        TimesheetCommand::Update {
            entry_id: id,
            entry: destination,
        },
    )
    .await
    .unwrap();
    grants(&pool, &f, &[Permission::TimeWriteManaged]).await;
    assert!(
        f.execute(
            &pool,
            TimesheetCommand::Update {
                entry_id: id,
                entry: f.input()
            }
        )
        .await
        .is_err()
    );
    assert_eq!(
        sqlx::query_scalar!("SELECT project_id FROM time_entries WHERE id=$1", id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        other
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn bulk_sets_do_not_partially_modify_foreign_locked_or_duplicate_entries(pool: PgPool) {
    let f = fixture(&pool).await;
    let id = f.entry(&pool).await;
    let foreign = seed(&pool, OrgRole::Member).await;
    let foreign_id = time_entry(&pool, &foreign, EntryState::Open).await;
    let locked = f.entry(&pool).await;
    sqlx::query!(
        "UPDATE time_entries SET state='approved' WHERE id=$1",
        locked
    )
    .execute(&pool)
    .await
    .unwrap();
    let before = snapshot(&pool, f.ids.org_id).await;
    for bad in [foreign_id, locked, Uuid::now_v7(), id] {
        for command in [
            TimesheetCommand::Delete {
                entry_ids: vec![id, bad],
            },
            TimesheetCommand::Reorder {
                spent_date: "2026-09-08".parse().unwrap(),
                entry_ids: vec![id, bad],
            },
        ] {
            assert!(f.execute(&pool, command).await.is_err());
        }
        assert_eq!(snapshot(&pool, f.ids.org_id).await, before);
    }
    assert!(
        sqlx::query_scalar!("SELECT id FROM time_entries WHERE id=$1", foreign_id)
            .fetch_optional(&pool)
            .await
            .unwrap()
            .is_some()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn delegated_stop_requires_restored_tracking_but_owner_keeps_terminal_recovery(pool: PgPool) {
    let f = fixture(&pool).await;
    let timer = f.entry(&pool).await;
    sqlx::query!("UPDATE time_entries SET is_running=true,started_at=now()-interval '15 minutes' WHERE id=$1",timer).execute(&pool).await.unwrap();
    sqlx::query!(
        "DELETE FROM assignments WHERE user_id=$1",
        f.context.subject_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let before = snapshot(&pool, f.ids.org_id).await;
    assert!(
        f.execute(&pool, TimesheetCommand::StopTimer { entry_id: timer })
            .await
            .is_err()
    );
    assert_eq!(snapshot(&pool, f.ids.org_id).await, before);
    sqlx::query!(
        "INSERT INTO assignments (id,project_id,user_id) VALUES ($1,$2,$3)",
        Uuid::now_v7(),
        f.ids.project_id,
        f.context.subject_id
    )
    .execute(&pool)
    .await
    .unwrap();
    f.execute(&pool, TimesheetCommand::StopTimer { entry_id: timer })
        .await
        .unwrap();
    sqlx::query!("UPDATE time_entries SET is_running=true,started_at=now()-interval '15 minutes' WHERE id=$1",timer).execute(&pool).await.unwrap();
    sqlx::query!(
        "DELETE FROM assignments WHERE user_id=$1",
        f.context.subject_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let stored: Vec<String> = serde_json::from_value(
        serde_json::to_value(PermissionSelection::new(&[Permission::TimeWriteOwn])).unwrap(),
    )
    .unwrap();
    sqlx::query!("INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source) VALUES ($1,$2,$3,1,$4,false,'individual')",Uuid::now_v7(),f.ids.org_id,f.context.subject_id,&stored).execute(&pool).await.unwrap();
    let own = TimesheetWriteContext {
        expected_requester: PermissionRequester {
            user_id: f.context.subject_id,
            ..f.context.expected_requester
        },
        ..f.context
    };
    assert!(
        apply(
            &pool,
            f.ids.org_id,
            f.context.subject_id,
            &own,
            TimesheetCommand::Update {
                entry_id: timer,
                entry: f.input()
            }
        )
        .await
        .is_err()
    );
    apply(
        &pool,
        f.ids.org_id,
        f.context.subject_id,
        &own,
        TimesheetCommand::StopTimer { entry_id: timer },
    )
    .await
    .unwrap();
    assert_eq!(
        sqlx::query_scalar!("SELECT minutes FROM time_entries WHERE id=$1", timer)
            .fetch_one(&pool)
            .await
            .unwrap(),
        15
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn coverage_conflicts_are_explicit_pending_submitted_editing_integration(pool: PgPool) {
    let f = fixture(&pool).await;
    let id = f.entry(&pool).await;
    sqlx::query!("INSERT INTO approvals (id,org_id,user_id,period_start,period_end,state) VALUES ($1,$2,$3,'2026-09-07','2026-09-13','submitted')",Uuid::now_v7(),f.ids.org_id,f.context.subject_id).execute(&pool).await.unwrap();
    let before = snapshot(&pool, f.ids.org_id).await;
    for command in [
        TimesheetCommand::Create { entry: f.input() },
        TimesheetCommand::Update {
            entry_id: id,
            entry: f.input(),
        },
        TimesheetCommand::Delete {
            entry_ids: vec![id],
        },
    ] {
        assert!(matches!(
            f.execute(&pool, command).await,
            Err(ServerFnError::ServerError { code: CONFLICT, .. })
        ));
    }
    assert_eq!(snapshot(&pool, f.ids.org_id).await, before);
}

#[sqlx::test(migrations = "./migrations")]
async fn legacy_own_history_is_editable_without_becoming_new_tracking_authority(pool: PgPool) {
    let mut f = fixture(&pool).await;
    let owner = f.context.subject_id;
    f.context.expected_requester.user_id = owner;
    f.context.expected_policy = TimesheetPolicy::LegacyOwn;
    f.ids.user_id = owner;
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version=0 WHERE id=$1",
        f.ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let id = f.entry(&pool).await;
    sqlx::query!(
        "UPDATE projects SET active=false WHERE id=$1",
        f.ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!("DELETE FROM assignments WHERE user_id=$1", owner)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        tracking(&pool, f.ids.org_id, owner, &f.context)
            .await
            .unwrap()
            .is_empty()
    );
    f.execute(
        &pool,
        TimesheetCommand::Update {
            entry_id: id,
            entry: f.input(),
        },
    )
    .await
    .unwrap();
    assert!(
        f.execute(&pool, TimesheetCommand::Create { entry: f.input() })
            .await
            .is_err()
    );
    assert!(
        f.execute(
            &pool,
            TimesheetCommand::StartTimer {
                project_id: f.ids.project_id,
                task_id: f.ids.task_id,
                notes: None
            }
        )
        .await
        .is_err()
    );
    let mut foreign = f.context;
    foreign.subject_id = Uuid::now_v7();
    assert!(
        apply(
            &pool,
            f.ids.org_id,
            owner,
            &foreign,
            TimesheetCommand::Delete {
                entry_ids: vec![id]
            }
        )
        .await
        .is_err()
    );
    f.execute(
        &pool,
        TimesheetCommand::Delete {
            entry_ids: vec![id],
        },
    )
    .await
    .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn task_grants_archives_and_canonical_admin_role_do_not_bypass_tracking(pool: PgPool) {
    let f = fixture(&pool).await;
    let id = f.entry(&pool).await;
    sqlx::query!("INSERT INTO project_task_settings (id,org_id,project_id,task_id,restricted) VALUES ($1,$2,$3,$4,true)",Uuid::now_v7(),f.ids.org_id,f.ids.project_id,f.ids.task_id).execute(&pool).await.unwrap();
    assert!(
        tracking(&pool, f.ids.org_id, f.ids.user_id, &f.context)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        f.execute(
            &pool,
            TimesheetCommand::Update {
                entry_id: id,
                entry: f.input()
            }
        )
        .await
        .is_err()
    );
    sqlx::query!("INSERT INTO project_task_members (id,org_id,project_id,task_id,user_id) VALUES ($1,$2,$3,$4,$5)",Uuid::now_v7(),f.ids.org_id,f.ids.project_id,f.ids.task_id,f.context.subject_id).execute(&pool).await.unwrap();
    f.execute(
        &pool,
        TimesheetCommand::Update {
            entry_id: id,
            entry: f.input(),
        },
    )
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE clients SET active=false WHERE id=$1",
        f.ids.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        f.execute(
            &pool,
            TimesheetCommand::Delete {
                entry_ids: vec![id]
            }
        )
        .await
        .is_err()
    );
    sqlx::query!(
        "UPDATE clients SET active=true WHERE id=$1",
        f.ids.client_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE users SET org_role='admin' WHERE org_id=$1",
        f.ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "DELETE FROM assignments WHERE user_id=$1",
        f.context.subject_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        f.execute(&pool, TimesheetCommand::Create { entry: f.input() })
            .await
            .is_err()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn no_op_emits_no_effects_and_changed_payload_uses_normalized_effective_values(pool: PgPool) {
    let f = fixture(&pool).await;
    let id = f.entry(&pool).await;
    let mut input = f.input();
    input.minutes = 60;
    input.notes = None;
    let effects = apply(
        &pool,
        f.ids.org_id,
        f.ids.user_id,
        &f.context,
        TimesheetCommand::Update {
            entry_id: id,
            entry: input.clone(),
        },
    )
    .await
    .unwrap();
    assert!(effects.entries.is_empty());
    assert!(effects.projects.is_empty());
    input.start_minute = Some(1438);
    input.minutes = 120;
    sqlx::query!(
        "UPDATE projects SET project_type='non_billable' WHERE id=$1",
        f.ids.project_id
    )
    .execute(&pool)
    .await
    .unwrap();
    let effects = apply(
        &pool,
        f.ids.org_id,
        f.ids.user_id,
        &f.context,
        TimesheetCommand::Update {
            entry_id: id,
            entry: input,
        },
    )
    .await
    .unwrap();
    assert_eq!(effects.entries.len(), 1);
    let (entry, event) = &effects.entries[0];
    assert!(matches!(event, TimeEntryEvent::Updated));
    assert!(!entry.billable);
    assert_eq!(entry.user_id, f.context.subject_id);
    assert_eq!(entry.minutes + entry.start_minute.unwrap(), 1440);
    let stored = sqlx::query!(
        "SELECT minutes,start_minute,billable FROM time_entries WHERE id=$1",
        id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (stored.minutes, stored.start_minute, stored.billable),
        (entry.minutes, entry.start_minute, entry.billable)
    );
}
