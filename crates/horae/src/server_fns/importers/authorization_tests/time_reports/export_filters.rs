use super::*;
use horae_core::permissions::catalog::{Permission, PermissionSelection};

const NOTES: [&str; 5] = ["Export A", "Export B", "Export C", "Hidden D", "Other E"];

async fn assert_download(response: reqwest::Response, format: &str, expected: &[&str]) {
    assert_eq!(response.status(), StatusCode::OK);
    let data = response.bytes().await.unwrap();
    if format == "csv" {
        let records = csv::Reader::from_reader(data.as_ref())
            .records()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let mut notes: Vec<_> = records.iter().map(|row| &row[7]).collect();
        let mut expected = expected.to_vec();
        notes.sort_unstable();
        expected.sort_unstable();
        assert_eq!(notes, expected);
    } else {
        let mut archive = zip::ZipArchive::new(Cursor::new(data)).unwrap();
        let mut sheet = String::new();
        archive
            .by_name("xl/worksheets/sheet1.xml")
            .unwrap()
            .read_to_string(&mut sheet)
            .unwrap();
        assert_eq!(sheet.matches("<row ").count(), expected.len() + 1);
        let mut strings = String::new();
        archive
            .by_name("xl/sharedStrings.xml")
            .unwrap()
            .read_to_string(&mut strings)
            .unwrap();
        for note in NOTES {
            assert_eq!(strings.contains(note), expected.contains(&note), "{note}");
        }
    }
}

pub(super) async fn check(pool: &PgPool, api: &Api) {
    let ids = crate::server_fns::test_seed::seed(pool, OrgRole::Member).await;
    let foreign = crate::server_fns::test_seed::seed(pool, OrgRole::Admin).await;
    let managed = Uuid::now_v7();
    let hidden = Uuid::now_v7();
    for user in [managed, hidden] {
        sqlx::query!(
            "INSERT INTO users (id,org_id,email,name) VALUES ($1,$2,$3,'Colleague')",
            user,
            ids.org_id,
            format!("{user}@test.com")
        )
        .execute(pool)
        .await
        .unwrap();
    }
    let client = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO clients (id,org_id,name,currency) VALUES ($1,$2,'Second client','EUR')",
        client,
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    let second = Uuid::now_v7();
    let excluded = Uuid::now_v7();
    for (project, client) in [(second, client), (excluded, ids.client_id)] {
        sqlx::query!(
            "INSERT INTO projects (id,org_id,client_id,name,currency) VALUES ($1,$2,$3,$4,'EUR')",
            project,
            ids.org_id,
            client,
            project.to_string()
        )
        .execute(pool)
        .await
        .unwrap();
    }
    let task = Uuid::now_v7();
    sqlx::query!(
        "INSERT INTO tasks (id,org_id,name) VALUES ($1,$2,'Second task')",
        task,
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    let tags = [Uuid::now_v7(), Uuid::now_v7()];
    for (tag, project) in [(tags[0], ids.project_id), (tags[1], second)] {
        sqlx::query!(
            "INSERT INTO project_tags (id,org_id,name) VALUES ($1,$2,$3)",
            tag,
            ids.org_id,
            tag.to_string()
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query!(
            "INSERT INTO project_tag_links (id,org_id,project_id,tag_id) VALUES ($1,$2,$3,$4)",
            Uuid::now_v7(),
            ids.org_id,
            project,
            tag
        )
        .execute(pool)
        .await
        .unwrap();
    }
    for (owner, project, task, note) in [
        (ids.user_id, ids.project_id, ids.task_id, NOTES[0]),
        (managed, second, task, NOTES[1]),
        (ids.user_id, second, ids.task_id, NOTES[2]),
        (hidden, ids.project_id, ids.task_id, NOTES[3]),
        (ids.user_id, excluded, ids.task_id, NOTES[4]),
    ] {
        sqlx::query!("INSERT INTO time_entries (id,org_id,user_id,project_id,task_id,spent_date,minutes,billable,notes) VALUES ($1,$2,$3,$4,$5,'2026-09-07',60,true,$6)", Uuid::now_v7(), ids.org_id, owner, project, task, note).execute(pool).await.unwrap();
    }
    sqlx::query!(
        "UPDATE organizations SET permission_policy_version=1 WHERE id=$1",
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "UPDATE users SET oidc_subject=id::text WHERE id=$1",
        ids.user_id
    )
    .execute(pool)
    .await
    .unwrap();
    let grants: Vec<String> = serde_json::from_value(json!(PermissionSelection::new(&[
        Permission::TimeReadOwn,
        Permission::TimeReadManaged
    ])))
    .unwrap();
    sqlx::query!("INSERT INTO person_permission_states (id,org_id,user_id,catalog_version,grants,is_administrator,source) VALUES ($1,$2,$3,1,$4,false,'individual')", Uuid::now_v7(), ids.org_id, ids.user_id, &grants).execute(pool).await.unwrap();
    sqlx::query!("INSERT INTO person_management_assignments (id,org_id,manager_id,managed_user_id) VALUES ($1,$2,$3,$4)", Uuid::now_v7(), ids.org_id, ids.user_id, managed).execute(pool).await.unwrap();
    let cookie = api.cookie(ids.user_id).await;
    let grouped = json!({"query": {
        "date_from":"2026-09-01", "date_to":"2026-09-30",
        "client_ids":[ids.client_id,client,foreign.client_id],
        "project_ids":[ids.project_id,second,foreign.project_id],
        "user_ids":[ids.user_id,managed,hidden,foreign.user_id],
        "task_ids":[ids.task_id,task], "tag_ids":[tags[0],tags[1],tags[0]],
        "group_by":"client", "after":null,
        "expected_requester":{"org_id":ids.org_id,"user_id":ids.user_id}
    }});
    // Exercise the same populated filter fixture through all four groupings.
    // Repeated tags/IDs must not multiply entries, and each dimension narrows
    // the authorized row set before aggregation.
    for dimension in ["client", "project", "task", "person"] {
        for (key, value, count) in [
            ("tag_ids", json!([tags[0], tags[1], tags[0]]), 3),
            ("task_ids", json!([task]), 1),
            ("user_ids", json!([managed, hidden]), 1),
            ("client_ids", json!([ids.client_id]), 1),
            ("tag_ids", json!([tags[0]]), 1),
            ("project_ids", json!([second, second]), 2),
            ("project_ids", json!([foreign.project_id]), 0),
        ] {
            let mut body = grouped.clone();
            body["query"]["group_by"] = json!(dimension);
            body["query"][key] = value;
            let response = api
                .json("list_visible_time_report_groups", body, &cookie)
                .await;
            assert_eq!(
                response["totals"],
                json!({
                    "entry_count":count,"total_minutes":count*60,
                    "rounded_minutes":count*60,"billable_minutes":count*60
                }),
                "{dimension}: {key}"
            );
            let entries: i64 = response["groups"]
                .as_array()
                .unwrap()
                .iter()
                .map(|group| group["totals"]["entry_count"].as_i64().unwrap())
                .sum();
            assert_eq!(entries, count);
        }
    }
    let binding = format!(
        "&expected_org_id={}&expected_user_id={}",
        ids.org_id, ids.user_id
    );
    let all = format!(
        "&client_ids={},{},{}&project_ids={},{},{}&user_ids={},{},{},{}&task_ids={},{}&tag_ids={},{},{}",
        ids.client_id,
        client,
        foreign.client_id,
        ids.project_id,
        second,
        foreign.project_id,
        ids.user_id,
        managed,
        hidden,
        foreign.user_id,
        ids.task_id,
        task,
        tags[0],
        tags[1],
        tags[0]
    );
    for format in ["csv", "xlsx"] {
        assert_download(
            download(api, Some(&cookie), format, &format!("{all}{binding}")).await,
            format,
            &NOTES[..3],
        )
        .await;
        for (filter, expected) in [
            (format!("&task_ids={task}"), vec![NOTES[1]]),
            (format!("&user_ids={managed},{hidden}"), vec![NOTES[1]]),
            (
                format!("&client_ids={}", ids.client_id),
                vec![NOTES[0], NOTES[4]],
            ),
            (format!("&tag_ids={}", tags[0]), vec![NOTES[0]]),
            (
                format!("&project_ids={second},{second}"),
                vec![NOTES[1], NOTES[2]],
            ),
            (
                format!("&client_ids={}&project_ids={second}", ids.client_id),
                vec![],
            ),
            (format!("&project_ids={}", foreign.project_id), vec![]),
            (
                format!("&project_id={second}&user_id={}", ids.user_id),
                vec![NOTES[2]],
            ),
        ] {
            assert_download(
                download(api, Some(&cookie), format, &filter).await,
                format,
                &expected,
            )
            .await;
        }
        for bad in [
            "&project_ids=invalid".to_owned(),
            format!("&user_ids={},", ids.user_id),
            format!("&project_id={second}&project_ids="),
            format!("&task_ids={task}&task_ids={task}"),
            format!("&expected_org_id={}", ids.org_id),
            "&expected_org_id=&expected_user_id=".to_owned(),
            "&after=".to_owned(),
            "&after".to_owned(),
        ] {
            assert_eq!(
                download(api, Some(&cookie), format, &bad).await.status(),
                StatusCode::BAD_REQUEST,
                "{format}: {bad}"
            );
        }
        for (org, user) in [(ids.org_id, managed), (foreign.org_id, ids.user_id)] {
            assert_eq!(
                download(
                    api,
                    Some(&cookie),
                    format,
                    &format!("&expected_org_id={org}&expected_user_id={user}")
                )
                .await
                .status(),
                StatusCode::FORBIDDEN
            );
        }
        // A real account switch must reject the original report's binding even
        // when the new account independently has export authority.
        let other_cookie = api.cookie(foreign.user_id).await;
        assert_eq!(
            download(api, Some(&other_cookie), format, &binding)
                .await
                .status(),
            StatusCode::FORBIDDEN
        );
    }

    // The entry flags remain true: filter effective project billability while
    // preserving the managed-person/own union and excluding the hidden user.
    sqlx::query!(
        "UPDATE projects SET project_type='non_billable' WHERE id=$1",
        ids.project_id
    )
    .execute(pool)
    .await
    .unwrap();
    for (billability, expected) in [
        ("all", &NOTES[..3]),
        ("billable", &NOTES[1..3]),
        ("non_billable", &NOTES[..1]),
    ] {
        let count = expected.len() as i64;
        let mut grouped_request = grouped.clone();
        grouped_request["query"]["billability"] = json!(billability);
        let report = api
            .json(
                "list_visible_time_report_groups",
                grouped_request.clone(),
                &cookie,
            )
            .await;
        assert_eq!(report["totals"]["entry_count"], count);
        assert_eq!(report["totals"]["rounded_minutes"], count * 60);
        grouped_request["query"]
            .as_object_mut()
            .unwrap()
            .remove("group_by");
        let report = api
            .json("list_visible_time_report_entries", grouped_request, &cookie)
            .await;
        assert_eq!(report["totals"]["entry_count"], count);
        let mut actual: Vec<_> = report["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry["notes"].as_str().unwrap())
            .collect();
        actual.sort_unstable();
        assert_eq!(actual, expected);
        for format in ["csv", "xlsx"] {
            let filters = format!("{all}{binding}&billability={billability}");
            assert_download(
                download(api, Some(&cookie), format, &filters).await,
                format,
                expected,
            )
            .await;
            let response = api.client.get(format!(
                "{}/api/reports/time/grouped/{format}?from=2026-09-01&to=2026-09-30&group_by=client{filters}", api.base
            )).header("cookie", &cookie).send().await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let data = response.bytes().await.unwrap();
            let groups = if billability == "all" { 2 } else { 1 };
            if format == "csv" {
                let records = csv::Reader::from_reader(data.as_ref())
                    .records()
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap();
                assert_eq!(records.len(), groups);
                let hours: Vec<_> = records.iter().map(|row| &row[1]).collect();
                assert_eq!(
                    hours,
                    match billability {
                        "all" => vec!["1.00", "2.00"],
                        "billable" => vec!["2.00"],
                        _ => vec!["1.00"],
                    }
                );
            } else {
                let mut archive = zip::ZipArchive::new(Cursor::new(data)).unwrap();
                let mut sheet = String::new();
                archive
                    .by_name("xl/worksheets/sheet1.xml")
                    .unwrap()
                    .read_to_string(&mut sheet)
                    .unwrap();
                assert_eq!(sheet.matches("<row ").count(), groups + 1);
            }
        }
    }
    for format in ["csv", "xlsx"] {
        for invalid in ["", "true", "null", "unknown", "all&billability=billable"] {
            let filters = format!("&billability={invalid}");
            assert_eq!(
                download(api, Some(&cookie), format, &filters)
                    .await
                    .status(),
                StatusCode::BAD_REQUEST
            );
            let response = api.client.get(format!(
                "{}/api/reports/time/grouped/{format}?from=2026-09-01&to=2026-09-30&group_by=client{filters}", api.base
            )).header("cookie", &cookie).send().await.unwrap();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        }
    }
}
