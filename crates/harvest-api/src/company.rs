//! `GET /company` (spec 021 FR-044 item 1): the organization's name and week
//! start in Harvest's shape, with Horae's fixed display conventions and the
//! feature flags of what Horae has. Every active person may read it.

use super::*;

pub(super) async fn company(user: AuthUser, State(db): State<PgPool>) -> ApiResult<HarvestCompany> {
    let mut read = begin_read(&db, &user, "Company").await?;
    let row = sqlx::query!(
        "SELECT name, week_start FROM organizations WHERE id = $1",
        user.org_id
    )
    .fetch_one(&mut *read.tx)
    .await
    .map_err(internal);
    let row = finish_read(read, row).await?;
    Ok(Json(HarvestCompany {
        base_uri: None,
        full_domain: None,
        name: row.name,
        is_active: true,
        week_start_day: week_start_day(row.week_start)?.to_owned(),
        // Horae records durations, with an optional start time.
        wants_timestamp_timers: false,
        // The Timesheet shows hours as H:MM and times of day on a 24-hour clock.
        time_format: "hours_minutes".to_owned(),
        date_format: None,
        plan_type: None,
        clock: "24h".to_owned(),
        currency_code_display: None,
        currency_symbol_display: None,
        decimal_symbol: None,
        thousands_separator: None,
        color_scheme: None,
        weekly_capacity: None,
        expense_feature: false,
        invoice_feature: true,
        estimate_feature: false,
        approval_feature: true,
        team_feature: true,
    }))
}

/// `organizations.week_start` counts from Monday as 1, as the Timesheet reads it.
fn week_start_day(day: i16) -> Result<&'static str, ApiError> {
    const DAYS: [&str; 7] = [
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
    ];
    usize::try_from(day - 1)
        .ok()
        .and_then(|index| DAYS.get(index).copied())
        .ok_or_else(|| internal(format!("week_start {day} is not between 1 and 7")))
}
