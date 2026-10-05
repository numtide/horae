//! Canonical grouped downloads share the ordinary time authority, not rates.

use super::*;
use crate::models::permission_editor::PermissionRequester;
use crate::models::time_report::{TimeReportGroup, TimeReportGrouping, TimeReportPolicy};

#[derive(Deserialize)]
pub struct GroupExportParams {
    group_by: TimeReportGrouping,
    #[serde(flatten)]
    filters: ExportParams,
}

pub async fn export_csv(
    session: Session,
    Query(params): Query<GroupExportParams>,
) -> Result<impl IntoResponse, StatusCode> {
    let (user_id, org_id) = require_session(&session).await?;
    let state = crate::state::global_state().await;
    streaming::groups::download(
        state.db.clone(),
        org_id,
        user_id,
        params.filters,
        params.group_by,
    )
    .await
}

pub async fn export_xlsx(
    session: Session,
    Query(params): Query<GroupExportParams>,
) -> Result<impl IntoResponse, StatusCode> {
    let (user_id, org_id) = require_session(&session).await?;
    if params.filters.expected_policy == Some(TimeReportPolicy::Legacy) {
        return Err(StatusCode::FORBIDDEN);
    }
    let query = params.filters.time_query()?;
    let permit = bounded::ExportPermit::acquire()?;
    let state = crate::state::global_state().await;
    let export = limits::time::groups::read(
        &state.db,
        PermissionRequester { user_id, org_id },
        &query,
        params.group_by,
    )
    .await?;
    let body = export
        .scope
        .render(permit, &state.db, move || {
            workbook(&export.rows, params.group_by)
        })
        .await?;
    Ok((
        [
            (
                axum::http::header::CONTENT_TYPE,
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                "attachment; filename=\"time-report.xlsx\"",
            ),
        ],
        body,
    ))
}

pub(super) fn headers(dimension: TimeReportGrouping) -> [&'static str; 4] {
    let name = match dimension {
        TimeReportGrouping::Client => "Client",
        TimeReportGrouping::Project => "Project",
        TimeReportGrouping::Task => "Task",
        TimeReportGrouping::Person => "Teammate",
    };
    [name, "Hours", "Billable Hours", "Non-billable Hours"]
}

pub(super) fn workbook(
    rows: &[TimeReportGroup],
    dimension: TimeReportGrouping,
) -> Result<Vec<u8>, StatusCode> {
    let mut workbook = rust_xlsxwriter::Workbook::new();
    let sheet = workbook.add_worksheet();
    for (column, label) in headers(dimension).into_iter().enumerate() {
        sheet
            .write_string(0, column as u16, label)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    for (index, group) in rows.iter().enumerate() {
        let nonbillable = group
            .totals
            .rounded_minutes
            .checked_sub(group.totals.billable_minutes)
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
        let values = [
            group.name.clone(),
            format_hours2(group.totals.rounded_minutes),
            format_hours2(group.totals.billable_minutes),
            format_hours2(nonbillable),
        ];
        for (column, value) in values.into_iter().enumerate() {
            sheet
                .write_string((index + 1) as u32, column as u16, value)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
    }
    bounded::workbook_bytes(&mut workbook)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(value: &str) -> Result<GroupExportParams, axum::extract::rejection::QueryRejection> {
        Query::<GroupExportParams>::try_from_uri(&format!("/?{value}").parse().unwrap())
            .map(|Query(params)| params)
    }

    #[test]
    fn active_projects_only_grouped_transport_is_optional_and_strict() {
        let base = "from=2026-09-07&to=2026-09-07&group_by=project";
        assert!(
            !parse(base)
                .unwrap()
                .filters
                .time_query()
                .unwrap()
                .active_projects_only
        );
        for active in [false, true] {
            assert_eq!(
                parse(&format!("{base}&active_projects_only={active}"))
                    .unwrap()
                    .filters
                    .time_query()
                    .unwrap()
                    .active_projects_only,
                active
            );
        }
        for value in ["", "1", "yes", "null", "true&active_projects_only=false"] {
            assert!(parse(&format!("{base}&active_projects_only={value}")).is_err());
        }
    }

    #[test]
    fn grouped_xlsx_transport_rejects_ambiguous_or_invalid_parameters() {
        let base = "from=2026-09-07&to=2026-09-07&group_by=project";
        let params = parse(base).unwrap();
        assert_eq!(params.group_by, TimeReportGrouping::Project);
        assert!(params.filters.time_query().is_ok());
        for suffix in [
            "&group_by=client",
            "&from=2026-09-08",
            "&expected_policy=",
            "&user_ids=x&user_ids=y",
        ] {
            assert!(parse(&format!("{base}{suffix}")).is_err(), "{suffix}");
        }
        for query in [
            "from=2026-09-07&to=2026-09-07",
            "from=2026-09-07&to=2026-09-07&group_by=unknown",
        ] {
            assert!(parse(query).is_err());
        }
        for suffix in [
            "&after=",
            "&user_ids=invalid",
            "&expected_user_id=019f3000-0000-7000-8000-000000000001",
        ] {
            let params = parse(&format!("{base}{suffix}")).unwrap();
            assert!(params.filters.time_query().is_err(), "{suffix}");
        }
    }
}
