//! Legacy Reports downloads checked against hand-derived fixture values
//! (specs/004-invoice-timesheet-exports plan, "Acceptance Increment").

use super::*;
use crate::reports::xlsx_cells::{assert_cell, assert_text_cell, xlsx_part};

const CLIENTS: [&str; 2] = ["Export client A", "Export client B"];
/// Project name and the index of its client.
const PROJECTS: [(&str, usize); 3] = [("Export alpha", 0), ("Export beta", 0), ("Export gamma", 1)];
const TASK: &str = "Export task";
const ACTOR: &str = "Test User";
const COLLEAGUE: &str = "Export colleague";
const FOREIGN_NOTE: &str = "Foreign organization entry";
const HEADERS: [&str; 8] = [
    "Date",
    "Project",
    "Task",
    "User",
    "Hours",
    "Rounded Hours",
    "Billable",
    "Notes",
];

/// One entry and what every download must show for it. Rounding is 15
/// minutes to the nearest. The XLSX texts are what the writer emits for
/// minutes / 60; they stay literals so this test does no float arithmetic.
struct Entry {
    date: &'static str,
    project: usize,
    colleague: bool,
    raw: i32,
    rounded: i32,
    billable: bool,
    notes: &'static str,
    xlsx_notes: &'static str,
    hours: &'static str,
    rounded_hours: &'static str,
    xlsx_hours: &'static str,
    xlsx_rounded: &'static str,
}

const ENTRIES: [Entry; 6] = [
    Entry {
        date: "2041-03-04",
        project: 0,
        colleague: false,
        raw: 50,
        rounded: 45,
        billable: true,
        notes: "Legacy export, \"one\"",
        xlsx_notes: "Legacy export, \"one\"",
        hours: "0.83",
        rounded_hours: "0.75",
        xlsx_hours: "0.8333333333333334",
        xlsx_rounded: "0.75",
    },
    Entry {
        date: "2041-03-04",
        project: 0,
        colleague: true,
        raw: 50,
        rounded: 45,
        billable: true,
        notes: "Legacy export <two> & more",
        xlsx_notes: "Legacy export &lt;two&gt; &amp; more",
        hours: "0.83",
        rounded_hours: "0.75",
        xlsx_hours: "0.8333333333333334",
        xlsx_rounded: "0.75",
    },
    Entry {
        date: "2041-03-05",
        project: 1,
        colleague: false,
        raw: 53,
        rounded: 60,
        billable: false,
        notes: "Legacy export three",
        xlsx_notes: "Legacy export three",
        hours: "0.88",
        rounded_hours: "1.00",
        xlsx_hours: "0.8833333333333333",
        xlsx_rounded: "1",
    },
    Entry {
        date: "2041-03-05",
        project: 1,
        colleague: false,
        raw: 7,
        rounded: 0,
        billable: true,
        notes: "",
        xlsx_notes: "",
        hours: "0.12",
        rounded_hours: "0.00",
        xlsx_hours: "0.11666666666666667",
        xlsx_rounded: "0",
    },
    Entry {
        date: "2041-03-06",
        project: 2,
        colleague: false,
        raw: 90,
        rounded: 90,
        billable: true,
        notes: "Legacy export excluded",
        xlsx_notes: "Legacy export excluded",
        hours: "1.50",
        rounded_hours: "1.50",
        xlsx_hours: "1.5",
        xlsx_rounded: "1.5",
    },
    // Outside the period: must never appear.
    Entry {
        date: "2041-03-07",
        project: 0,
        colleague: false,
        raw: 60,
        rounded: 60,
        billable: true,
        notes: "Legacy export outside",
        xlsx_notes: "Legacy export outside",
        hours: "1.00",
        rounded_hours: "1.00",
        xlsx_hours: "1",
        xlsx_rounded: "1",
    },
];

/// The unfiltered CSV, byte for byte: the regression proof that this
/// increment leaves the export unchanged.
const UNFILTERED_CSV: &str = "Date,Project,Task,User,Hours,Rounded Hours,Billable,Notes
2041-03-04,Export alpha,Export task,Test User,0.83,0.75,Yes,\"Legacy export, \"\"one\"\"\"
2041-03-04,Export alpha,Export task,Export colleague,0.83,0.75,Yes,Legacy export <two> & more
2041-03-05,Export beta,Export task,Test User,0.88,1.00,No,Legacy export three
2041-03-05,Export beta,Export task,Test User,0.12,0.00,Yes,
2041-03-06,Export gamma,Export task,Test User,1.50,1.50,Yes,Legacy export excluded
";

fn fixture_id(n: u128) -> Uuid {
    Uuid::from_u128(0x019f3200_0000_7000_8000_000000000000 + n)
}

async fn download(api: &Api, cookie: &str, format: &str, filters: &str) -> Vec<u8> {
    let response = api
        .client
        .get(format!(
            "{}/api/reports/export/{format}?from=2041-03-04&to=2041-03-06{filters}",
            api.base
        ))
        .header("cookie", cookie)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK, "{format}{filters}");
    response.bytes().await.unwrap().to_vec()
}

fn user(entry: &Entry) -> &'static str {
    if entry.colleague { COLLEAGUE } else { ACTOR }
}

fn yes_no(billable: bool) -> &'static str {
    if billable { "Yes" } else { "No" }
}

pub(super) async fn check(pool: &PgPool, api: &Api) {
    let ids = crate::server_fns::test_seed::seed(pool, OrgRole::Manager).await;
    sqlx::query!(
        "UPDATE organizations SET round_minutes = 15, round_dir = 'nearest' WHERE id = $1",
        ids.org_id
    )
    .execute(pool)
    .await
    .unwrap();
    for (n, name) in CLIENTS.iter().enumerate() {
        sqlx::query!(
            "INSERT INTO clients (id, org_id, name, currency) VALUES ($1, $2, $3, 'EUR')",
            fixture_id(10 + n as u128),
            ids.org_id,
            name
        )
        .execute(pool)
        .await
        .unwrap();
    }
    for (n, (name, client)) in PROJECTS.iter().enumerate() {
        sqlx::query!(
            "INSERT INTO projects (id, org_id, client_id, name, currency) VALUES ($1, $2, $3, $4, 'EUR')",
            fixture_id(20 + n as u128),
            ids.org_id,
            fixture_id(10 + *client as u128),
            name
        )
        .execute(pool)
        .await
        .unwrap();
    }
    let (task, colleague) = (fixture_id(30), fixture_id(31));
    sqlx::query!(
        "INSERT INTO tasks (id, org_id, name) VALUES ($1, $2, $3)",
        task,
        ids.org_id,
        TASK
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO users (id, org_id, email, name) VALUES ($1, $2, 'export-colleague@test.com', $3)",
        colleague,
        ids.org_id,
        COLLEAGUE
    )
    .execute(pool)
    .await
    .unwrap();
    // Ascending IDs fix the order of rows sharing a date, project and task.
    for (n, entry) in ENTRIES.iter().enumerate() {
        sqlx::query!(
            "INSERT INTO time_entries (id, org_id, user_id, project_id, task_id, spent_date, minutes, billable, notes)
             VALUES ($1, $2, $3, $4, $5, $6::text::date, $7, $8, $9)",
            fixture_id(100 + n as u128),
            ids.org_id,
            if entry.colleague { colleague } else { ids.user_id },
            fixture_id(20 + entry.project as u128),
            task,
            entry.date,
            entry.raw,
            entry.billable,
            (!entry.notes.is_empty()).then_some(entry.notes),
        )
        .execute(pool)
        .await
        .unwrap();
    }
    // Another organization's client with an entry in the period: a legacy
    // filter naming it must not reach across tenants.
    let foreign = crate::server_fns::test_seed::seed(pool, OrgRole::Admin).await;
    sqlx::query!(
        "INSERT INTO time_entries (id, org_id, user_id, project_id, task_id, spent_date, minutes, billable, notes)
             VALUES ($1, $2, $3, $4, $5, $6::text::date, $7, $8, $9)",
        fixture_id(200),
        foreign.org_id,
        foreign.user_id,
        foreign.project_id,
        foreign.task_id,
        "2041-03-05",
        60,
        true,
        Some(FOREIGN_NOTE),
    )
    .execute(pool)
    .await
    .unwrap();
    // The route takes the legacy path only at policy 0; never rely on a default.
    let policy = sqlx::query_scalar!(
        "SELECT permission_policy_version FROM organizations WHERE id = $1",
        ids.org_id
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(policy, 0);
    let cookie = api.cookie(ids.user_id).await;

    let (client_a, beta, gamma) = (fixture_id(10), fixture_id(21), fixture_id(22));
    // (filters, expected entries, raw/rounded/billable-rounded minutes)
    let states: [(String, &[usize], [i64; 3]); 7] = [
        (String::new(), &[0, 1, 2, 3, 4], [250, 240, 180]),
        (format!("&project_id={gamma}"), &[4], [90, 90, 90]),
        (
            format!("&client_id={client_a}"),
            &[0, 1, 2, 3],
            [160, 150, 90],
        ),
        (
            format!("&client_id={client_a}&project_id={beta}"),
            &[2, 3],
            [60, 60, 0],
        ),
        // A project outside the client: valid files without entry rows.
        (
            format!("&client_id={client_a}&project_id={gamma}"),
            &[],
            [0, 0, 0],
        ),
        (format!("&client_id={}", foreign.client_id), &[], [0, 0, 0]),
        // The colleague shares E1's project and date: only the person differs.
        (format!("&user_id={colleague}"), &[1], [50, 45, 45]),
    ];
    for (filters, expected, totals) in states {
        let entries: Vec<&Entry> = expected.iter().map(|&n| &ENTRIES[n]).collect();
        let minutes = [
            entries.iter().map(|e| i64::from(e.raw)).sum::<i64>(),
            entries.iter().map(|e| i64::from(e.rounded)).sum(),
            entries
                .iter()
                .filter(|e| e.billable)
                .map(|e| i64::from(e.rounded))
                .sum(),
        ];
        assert_eq!(minutes, totals, "{filters}");

        let csv = download(api, &cookie, "csv", &filters).await;
        if filters.is_empty() {
            assert_eq!(std::str::from_utf8(&csv).unwrap(), UNFILTERED_CSV);
        }
        let mut reader = csv::Reader::from_reader(csv.as_slice());
        assert_eq!(reader.headers().unwrap(), HEADERS.as_slice(), "{filters}");
        let rows: Vec<Vec<String>> = reader
            .records()
            .map(|row| row.unwrap().iter().map(str::to_owned).collect())
            .collect();
        let expected_rows: Vec<Vec<String>> = entries
            .iter()
            .map(|e| {
                [
                    e.date,
                    PROJECTS[e.project].0,
                    TASK,
                    user(e),
                    e.hours,
                    e.rounded_hours,
                    yes_no(e.billable),
                    e.notes,
                ]
                .map(str::to_owned)
                .to_vec()
            })
            .collect();
        assert_eq!(rows, expected_rows, "{filters}");
        if filters == format!("&client_id={client_a}") {
            // Displayed row hours add up to 2.66 while the exact total, summed
            // in minutes, reads 2.67: totals must never be added from decimals.
            let hundredths: i64 = rows
                .iter()
                .map(|row| row[4].replace('.', "").parse::<i64>().unwrap())
                .sum();
            assert_eq!(hundredths, 266);
            assert_eq!(horae_core::duration::format_hours2(totals[0]), "2.67");
        }

        let xlsx = download(api, &cookie, "xlsx", &filters).await;
        let sheet = xlsx_part(&xlsx, "xl/worksheets/sheet1.xml");
        assert_eq!(
            sheet.matches("<row ").count(),
            entries.len() + 1,
            "{filters}"
        );
        for (column, header) in ["A", "B", "C", "D", "E", "F", "G", "H"].iter().zip(HEADERS) {
            assert_text_cell(&xlsx, &format!("{column}1"), header);
        }
        for (n, e) in entries.iter().enumerate() {
            let row = n + 2;
            for (column, text) in [
                ("A", e.date),
                ("B", PROJECTS[e.project].0),
                ("C", TASK),
                ("D", user(e)),
                ("G", yes_no(e.billable)),
            ] {
                assert_text_cell(&xlsx, &format!("{column}{row}"), text);
            }
            assert_cell(&sheet, &format!("E{row}"), e.xlsx_hours);
            assert_cell(&sheet, &format!("F{row}"), e.xlsx_rounded);
            if e.notes.is_empty() {
                // The writer leaves an empty note's cell out entirely.
                assert!(!sheet.contains(&format!("r=\"H{row}\"")), "{filters}");
            } else {
                assert_text_cell(&xlsx, &format!("H{row}"), e.xlsx_notes);
            }
        }
        let strings = xlsx_part(&xlsx, "xl/sharedStrings.xml");
        for (n, e) in ENTRIES.iter().enumerate() {
            if !expected.contains(&n) && !e.notes.is_empty() {
                assert!(!strings.contains(e.xlsx_notes), "{filters}: {}", e.notes);
            }
        }
        assert!(!strings.contains(FOREIGN_NOTE), "{filters}");
    }
}
