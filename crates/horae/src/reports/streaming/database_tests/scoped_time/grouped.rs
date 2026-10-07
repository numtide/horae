use super::*;
use crate::models::time_report::TimeReportGrouping;

mod active_projects;

async fn project_entries(pool: &PgPool, ids: &SeedIds, owner: Uuid, count: usize) -> Vec<Uuid> {
    let projects: Vec<_> = (0..count).map(|_| Uuid::now_v7()).collect();
    let entries: Vec<_> = (0..count).map(|_| Uuid::now_v7()).collect();
    sqlx::query!(
        "INSERT INTO projects (id,org_id,client_id,name,currency)
        SELECT id,$2,$3,'Same name','EUR' FROM unnest($1::uuid[]) id",
        &projects,
        ids.org_id,
        ids.client_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!("INSERT INTO time_entries (id,org_id,user_id,project_id,task_id,spent_date,minutes,billable)
        SELECT id,$3,$4,project_id,$5,'2026-09-07',60,true FROM unnest($1::uuid[],$2::uuid[]) pairs(id,project_id)",
        &entries, &projects, ids.org_id, owner, ids.task_id).execute(pool).await.unwrap();
    projects
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn grouped_csv_preserves_distinct_ids_beyond_ten_thousand_groups(pool: PgPool) {
    let ids = scoped(&pool, OrgRole::Member, &[Permission::TimeReadOwn]).await;
    project_entries(&pool, &ids, ids.user_id, 10_001).await;
    let response = groups::download(
        pool,
        ids.org_id,
        ids.user_id,
        params(),
        TimeReportGrouping::Project,
    )
    .await
    .unwrap();
    let bytes = body(response).await;
    let records = csv::Reader::from_reader(bytes.as_slice())
        .records()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(records.len(), 10_001);
    assert!(
        records
            .iter()
            .all(|row| row.iter().collect::<Vec<_>>() == ["Same name", "1.00", "1.00", "0.00"])
    );
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn grouped_csv_checks_last_context_before_emitting_any_group_bytes(pool: PgPool) {
    for revoke_last in [false, true] {
        let ids = scoped(&pool, OrgRole::Member, &[Permission::TimeReadManaged]).await;
        let other = person(&pool, ids.org_id).await;
        let projects = project_entries(&pool, &ids, other, 129).await;
        let edges: Vec<_> = projects.iter().map(|_| Uuid::now_v7()).collect();
        sqlx::query!(
            "INSERT INTO project_management_assignments (id,org_id,manager_id,project_id)
            SELECT id,$3,$4,project_id FROM unnest($1::uuid[],$2::uuid[]) pairs(id,project_id)",
            &edges,
            &projects,
            ids.org_id,
            ids.user_id
        )
        .execute(&pool)
        .await
        .unwrap();
        let mut tx = pool.begin().await.unwrap();
        let (authority, _) = Authority::time(&mut tx, ids.org_id, ids.user_id)
            .await
            .unwrap();
        cursor::groups::declare(
            &mut tx,
            ids.org_id,
            ids.user_id,
            &params().time_query().unwrap(),
            TimeReportGrouping::Client,
        )
        .await
        .unwrap();
        if revoke_last {
            sqlx::query!(
                "DELETE FROM project_management_assignments WHERE project_id=$1",
                projects[128]
            )
            .execute(&pool)
            .await
            .unwrap();
        }
        // Later entry edits cannot replace the original contexts or totals.
        sqlx::query!(
            "UPDATE time_entries SET project_id=$2,minutes=120 WHERE org_id=$1",
            ids.org_id,
            ids.project_id
        )
        .execute(&pool)
        .await
        .unwrap();
        let (sender, mut receiver) = mpsc::channel(1);
        let result =
            groups::deliver(&mut tx, &sender, &authority, TimeReportGrouping::Client).await;
        if revoke_last {
            assert_eq!(result.unwrap_err(), StatusCode::FORBIDDEN);
            assert!(receiver.try_recv().is_err());
        } else {
            result.unwrap();
            let bytes = receiver.recv().await.unwrap();
            let records = csv::Reader::from_reader(bytes.as_slice())
                .records()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert_eq!(records.len(), 1);
            assert_eq!(&records[0][1], "129.00");
        }
        tx.rollback().await.unwrap();
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn grouped_csv_revocation_under_backpressure_does_not_hold_gates(pool: PgPool) {
    let ids = scoped(&pool, OrgRole::Member, &[Permission::TimeReadAll]).await;
    let other = person(&pool, ids.org_id).await;
    project_entries(&pool, &ids, other, 10).await;
    let response = groups::download(
        pool.clone(),
        ids.org_id,
        ids.user_id,
        params(),
        TimeReportGrouping::Project,
    )
    .await
    .unwrap();
    let mut writer = pool.begin().await.unwrap();
    sqlx::query!("SET LOCAL lock_timeout='1s'")
        .execute(&mut *writer)
        .await
        .unwrap();
    crate::db::lock_organization(
        &mut writer,
        ids.org_id,
        crate::db::OrganizationLock::AccessChange,
    )
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE person_permission_states SET grants=ARRAY['time_read_own'] WHERE user_id=$1",
        ids.user_id
    )
    .execute(&mut *writer)
    .await
    .unwrap();
    writer.commit().await.unwrap();
    let mut stream = response.into_body().into_data_stream();
    let mut bytes = Vec::new();
    let mut failed = false;
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(chunk) => bytes.extend_from_slice(&chunk),
            Err(_) => {
                failed = true;
                break;
            }
        }
    }
    assert!(failed, "revocation must interrupt the body");
    assert!(csv::Reader::from_reader(bytes.as_slice()).records().count() <= 2);
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn grouped_csv_invalid_source_authority_cannot_be_repaired_after_declare(pool: PgPool) {
    for populated in [false, true] {
        let ids = scoped(&pool, OrgRole::Member, &[Permission::TimeReadOwn]).await;
        if populated {
            add_entries(&pool, &ids, 1).await;
        }
        let mut tx = pool.begin().await.unwrap();
        Authority::time(&mut tx, ids.org_id, ids.user_id)
            .await
            .unwrap();
        sqlx::query!(
            "UPDATE person_permission_states SET catalog_version=99 WHERE user_id=$1",
            ids.user_id
        )
        .execute(&pool)
        .await
        .unwrap();
        cursor::groups::declare(
            &mut tx,
            ids.org_id,
            ids.user_id,
            &params().time_query().unwrap(),
            TimeReportGrouping::Client,
        )
        .await
        .unwrap();
        sqlx::query!(
            "UPDATE person_permission_states SET catalog_version=1 WHERE user_id=$1",
            ids.user_id
        )
        .execute(&pool)
        .await
        .unwrap();
        let row = cursor::groups::fetch(&mut tx, 1)
            .await
            .unwrap()
            .pop()
            .unwrap();
        assert_eq!(
            row.into_fragment().err(),
            Some(StatusCode::INTERNAL_SERVER_ERROR)
        );
        tx.rollback().await.unwrap();
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn grouped_csv_bounds_native_fragments_and_preserves_large_unicode_labels(pool: PgPool) {
    let ids = scoped(&pool, OrgRole::Member, &[Permission::TimeReadOwn]).await;
    project_entries(&pool, &ids, ids.user_id, 3).await;
    let name = "界,\"quoted\"\n".repeat(10_000);
    sqlx::query!(
        "UPDATE clients SET name=$2 WHERE id=$1",
        ids.client_id,
        name
    )
    .execute(&pool)
    .await
    .unwrap();
    let mut tx = pool.begin().await.unwrap();
    Authority::time(&mut tx, ids.org_id, ids.user_id)
        .await
        .unwrap();
    cursor::groups::declare(
        &mut tx,
        ids.org_id,
        ids.user_id,
        &params().time_query().unwrap(),
        TimeReportGrouping::Client,
    )
    .await
    .unwrap();
    for last in [false, false, true] {
        let mut rows = cursor::groups::fetch(&mut tx, 128).await.unwrap();
        assert_eq!(rows.len(), 1);
        let fragment = rows.pop().unwrap().into_fragment().unwrap().unwrap();
        assert_eq!(fragment.last_context, last);
        assert_eq!(fragment.rounded_minutes, 180);
    }
    assert!(
        cursor::groups::fetch(&mut tx, 128)
            .await
            .unwrap()
            .is_empty()
    );
    tx.rollback().await.unwrap();
    let response = groups::download(
        pool,
        ids.org_id,
        ids.user_id,
        params(),
        TimeReportGrouping::Client,
    )
    .await
    .unwrap();
    let bytes = body(response).await;
    let records = csv::Reader::from_reader(bytes.as_slice())
        .records()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(&records[0][0], name);
    assert_eq!(&records[0][1], "3.00");
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn grouped_csv_keeps_authority_between_fragment_batches_and_releases_on_cancel(pool: PgPool) {
    // This database-local probe pauses the third native fetch, after the first
    // 128 contexts have been authorized. It never changes production SQL.
    sqlx::query!(r#"CREATE OR REPLACE FUNCTION fetch_csv_export_rows(max_rows integer)
        RETURNS SETOF record LANGUAGE plpgsql SECURITY INVOKER AS $$
        DECLARE
            source refcursor := 'horae_csv';
            item record;
            payload_bytes bigint := 0;
            calls integer := COALESCE(NULLIF(current_setting('horae_test.csv_fetches',true),''),'0')::integer;
        BEGIN
            PERFORM set_config('horae_test.csv_fetches',(calls+1)::text,true);
            IF calls=2 THEN PERFORM pg_advisory_xact_lock(733212); END IF;
            IF max_rows IS NULL OR max_rows < 1 OR max_rows > 128 THEN
                RAISE EXCEPTION 'Invalid CSV batch size';
            END IF;
            FOR position IN 1..max_rows LOOP
                FETCH NEXT FROM source INTO item;
                EXIT WHEN NOT FOUND;
                IF item.export_bytes IS NULL OR item.export_bytes < 0 THEN
                    RAISE EXCEPTION 'Invalid CSV payload weight';
                END IF;
                payload_bytes := payload_bytes + item.export_bytes;
                RETURN NEXT item;
                EXIT WHEN payload_bytes >= 65536;
            END LOOP;
        END; $$"#).execute(&pool).await.unwrap();
    for cancel in [false, true] {
        let ids = scoped(&pool, OrgRole::Member, &[Permission::TimeReadAll]).await;
        let other = person(&pool, ids.org_id).await;
        project_entries(&pool, &ids, other, 257).await;
        let mut blocker = pool.begin().await.unwrap();
        let blocker_pid = sqlx::query_scalar!("SELECT pg_backend_pid()")
            .fetch_one(&mut *blocker)
            .await
            .unwrap()
            .unwrap();
        sqlx::query!("SELECT pg_advisory_xact_lock(733212)")
            .execute(&mut *blocker)
            .await
            .unwrap();
        let export_pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_with(pool.connect_options().as_ref().clone())
            .await
            .unwrap();
        let worker_pool = export_pool.clone();
        let pending = tokio::spawn(async move {
            groups::download(
                worker_pool,
                ids.org_id,
                ids.user_id,
                params(),
                TimeReportGrouping::Client,
            )
            .await
        });
        crate::server_fns::test_seed::wait_for_blocked(&pool, blocker_pid).await;
        let reader_pid = sqlx::query_scalar!("SELECT pid FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid))", blocker_pid)
            .fetch_one(&pool).await.unwrap().unwrap();
        let writer_pool = pool.clone();
        let writer = tokio::spawn(async move {
            let mut tx = writer_pool.begin().await.unwrap();
            crate::db::lock_organization(
                &mut tx,
                ids.org_id,
                crate::db::OrganizationLock::AccessChange,
            )
            .await
            .unwrap();
            sqlx::query!("UPDATE person_permission_states SET grants=ARRAY['time_read_own'] WHERE user_id=$1", ids.user_id)
                .execute(&mut *tx).await.unwrap();
            tx.commit().await.unwrap();
        });
        crate::server_fns::test_seed::wait_for_blocked(&pool, reader_pid).await;
        if cancel {
            pending.abort();
            assert!(pending.await.unwrap_err().is_cancelled());
        } else {
            blocker.rollback().await.unwrap();
            let response = pending.await.unwrap().unwrap();
            let bytes = body(response).await;
            let records = csv::Reader::from_reader(bytes.as_slice())
                .records()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert_eq!(&records[0][1], "257.00");
            tokio::time::timeout(Duration::from_secs(5), writer)
                .await
                .unwrap()
                .unwrap();
            export_pool.close().await;
            continue;
        }
        blocker.rollback().await.unwrap();
        tokio::time::timeout(Duration::from_secs(5), writer)
            .await
            .unwrap()
            .unwrap();
        let connection = tokio::time::timeout(Duration::from_secs(5), export_pool.acquire())
            .await
            .unwrap()
            .unwrap();
        drop(connection);
        export_pool.close().await;
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn grouped_csv_exports_all_four_dimensions_with_exact_rounded_hours(pool: PgPool) {
    let ids = scoped(&pool, OrgRole::Member, &[Permission::TimeReadOwn]).await;
    let entry = time_entry(&pool, &ids, EntryState::Open).await;
    sqlx::query!(
        "UPDATE time_entries SET minutes=61,rounded_minutes=75 WHERE id=$1",
        entry
    )
    .execute(&pool)
    .await
    .unwrap();
    let other = person(&pool, ids.org_id).await;
    add_entries(
        &pool,
        &SeedIds {
            user_id: other,
            ..ids
        },
        1,
    )
    .await;
    for (dimension, header) in [
        (TimeReportGrouping::Client, "Client"),
        (TimeReportGrouping::Project, "Project"),
        (TimeReportGrouping::Task, "Task"),
        (TimeReportGrouping::Person, "Teammate"),
    ] {
        let response = groups::download(pool.clone(), ids.org_id, ids.user_id, params(), dimension)
            .await
            .unwrap();
        let bytes = body(response).await;
        let mut reader = csv::Reader::from_reader(bytes.as_slice());
        assert_eq!(
            reader.headers().unwrap().iter().collect::<Vec<_>>(),
            [header, "Hours", "Billable Hours", "Non-billable Hours"]
        );
        let records = reader.records().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0].iter().skip(1).collect::<Vec<_>>(),
            ["1.25", "1.25", "0.00"]
        );
        assert!(
            !String::from_utf8(bytes)
                .unwrap()
                .contains(&other.to_string())
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn grouped_csv_has_no_workbook_source_entry_cap(pool: PgPool) {
    let ids = scoped(&pool, OrgRole::Member, &[Permission::TimeReadOwn]).await;
    add_entries(&pool, &ids, 10_001).await;
    let response = groups::download(
        pool,
        ids.org_id,
        ids.user_id,
        params(),
        TimeReportGrouping::Project,
    )
    .await
    .unwrap();
    let bytes = body(response).await;
    let records = csv::Reader::from_reader(bytes.as_slice())
        .records()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(&records[0][1], "10001.00");
}

#[sqlx::test(migrations = "./migrations")]
#[serial_test::serial]
async fn grouped_csv_empty_source_still_requires_canonical_authority(pool: PgPool) {
    let ids = scoped(&pool, OrgRole::Manager, &[Permission::TimeReadOwn]).await;
    let response = groups::download(
        pool.clone(),
        ids.org_id,
        ids.user_id,
        params(),
        TimeReportGrouping::Client,
    )
    .await
    .unwrap();
    assert_eq!(
        body(response).await,
        b"Client,Hours,Billable Hours,Non-billable Hours\n"
    );
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version=0 WHERE id=$1",
        ids.org_id
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        groups::download(
            pool,
            ids.org_id,
            ids.user_id,
            params(),
            TimeReportGrouping::Client
        )
        .await
        .unwrap_err(),
        StatusCode::FORBIDDEN
    );
}
