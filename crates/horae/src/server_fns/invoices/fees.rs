use super::*;
use chrono::{Datelike, Months, NaiveDate};
use horae_core::project::{MonthlyFeeDay, monthly_fee_date};
use uuid::Uuid;

const MAX_FEE_OCCURRENCES: usize = 10_000;

pub(super) struct FeeLine {
    pub id: Uuid,
    pub description: String,
    pub amount_cents: i64,
    pub currency: String,
}

struct Occurrence {
    project_id: Uuid,
    milestone_id: Option<Uuid>,
    period_key: String,
    due_on: NaiveDate,
    description: String,
    amount_cents: i64,
    currency: String,
}

/// Prepare stable occurrences inside the invoice transaction. Only monthly
/// fees use the range's lower bound; overdue single fees and milestones remain
/// available until claimed. No schedule sends or issues an invoice itself.
pub(super) async fn prepare_fees(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: Uuid,
    client_id: Uuid,
    from: NaiveDate,
    to: NaiveDate,
) -> Result<Vec<FeeLine>, ServerFnError> {
    if from > to {
        return Err(err(BAD_REQUEST, "Invoice period ends before it starts"));
    }
    let projects = sqlx::query!(
        r#"SELECT p.id, p.name, p.currency,
                  COALESCE(p.starts_on, (p.created_at AT TIME ZONE 'UTC')::date) as "starts_on!: NaiveDate",
                  p.ends_on as "ends_on: NaiveDate", s.fee_mode as "fee_mode!",
                  s.fee_amount_cents, s.monthly_day
           FROM projects p JOIN project_settings s ON s.project_id = p.id
           WHERE p.org_id = $1 AND p.client_id = $2 AND p.project_type = 'fixed_fee'
             AND s.fee_mode IS NOT NULL
           ORDER BY p.id LIMIT 1001 FOR SHARE OF p, s"#,
        org_id, client_id,
    ).fetch_all(&mut **tx).await.map_err(server_err)?;
    if projects.len() > 1000 {
        return Err(conflict(
            "Too many fee projects for one invoice; select fewer projects.",
        ));
    }
    let mut occurrences = Vec::new();
    for project in &projects {
        match project.fee_mode.as_str() {
            "single" if project.starts_on <= to => {
                occurrences.push(Occurrence {
                    project_id: project.id,
                    milestone_id: None,
                    period_key: "single".into(),
                    due_on: project.starts_on,
                    description: format!("{} — Fixed fee", project.name),
                    amount_cents: project
                        .fee_amount_cents
                        .ok_or_else(|| conflict("Project fee amount is missing"))?,
                    currency: project.currency.clone(),
                });
            }
            "milestones" => {
                let milestones = sqlx::query!(
                    r#"SELECT id, name, due_on as "due_on: NaiveDate", amount_cents
                       FROM project_fee_milestones WHERE project_id = $1 AND org_id = $2 AND due_on <= $3
                       ORDER BY position FOR SHARE"#,
                    project.id, org_id, to as NaiveDate,
                ).fetch_all(&mut **tx).await.map_err(server_err)?;
                for milestone in milestones {
                    occurrences.push(Occurrence {
                        project_id: project.id,
                        milestone_id: Some(milestone.id),
                        period_key: format!("milestone:{}", milestone.id),
                        due_on: milestone.due_on,
                        description: format!("{} — {}", project.name, milestone.name),
                        amount_cents: milestone.amount_cents,
                        currency: project.currency.clone(),
                    });
                }
            }
            "monthly" => {
                let start = from.max(project.starts_on);
                let end = project.ends_on.map_or(to, |end| end.min(to));
                if start > end {
                    continue;
                }
                let months = (end.year() - start.year()) * 12 + end.month() as i32
                    - start.month() as i32
                    + 1;
                if months > 1200 {
                    return Err(err(
                        BAD_REQUEST,
                        "Monthly fee periods must not exceed 100 years",
                    ));
                }
                let day = match project.monthly_day.as_deref() {
                    Some("first") => MonthlyFeeDay::First,
                    Some("fifteenth") => MonthlyFeeDay::Fifteenth,
                    Some("last") => MonthlyFeeDay::Last,
                    _ => return Err(conflict("Project monthly fee day is missing")),
                };
                let amount = project
                    .fee_amount_cents
                    .ok_or_else(|| conflict("Project fee amount is missing"))?;
                let mut month = start
                    .with_day(1)
                    .ok_or_else(|| err(BAD_REQUEST, "Invalid invoice period"))?;
                for _ in 0..months {
                    let due = monthly_fee_date(month.year(), month.month(), day)
                        .map_err(|error| err(BAD_REQUEST, error.to_string()))?;
                    if due >= start && due <= end {
                        occurrences.push(Occurrence {
                            project_id: project.id,
                            milestone_id: None,
                            period_key: format!("month:{}", month.format("%Y-%m")),
                            due_on: due,
                            description: format!(
                                "{} — {} fee",
                                project.name,
                                month.format("%Y-%m")
                            ),
                            amount_cents: amount,
                            currency: project.currency.clone(),
                        });
                    }
                    month = month.checked_add_months(Months::new(1)).ok_or_else(|| {
                        err(BAD_REQUEST, "Invoice period exceeds the supported calendar")
                    })?;
                }
            }
            _ => {}
        }
        if occurrences.len() > MAX_FEE_OCCURRENCES {
            return Err(conflict(
                "Too many fees for one invoice; select a shorter period.",
            ));
        }
    }
    let ids: Vec<Uuid> = occurrences.iter().map(|_| Uuid::now_v7()).collect();
    let project_ids: Vec<Uuid> = occurrences.iter().map(|fee| fee.project_id).collect();
    let milestone_ids: Vec<Option<Uuid>> = occurrences.iter().map(|fee| fee.milestone_id).collect();
    let keys: Vec<&str> = occurrences
        .iter()
        .map(|fee| fee.period_key.as_str())
        .collect();
    let dates: Vec<NaiveDate> = occurrences.iter().map(|fee| fee.due_on).collect();
    let descriptions: Vec<&str> = occurrences
        .iter()
        .map(|fee| fee.description.as_str())
        .collect();
    let amounts: Vec<i64> = occurrences.iter().map(|fee| fee.amount_cents).collect();
    let currencies: Vec<&str> = occurrences
        .iter()
        .map(|fee| fee.currency.as_str())
        .collect();
    sqlx::query!(
        "INSERT INTO project_fee_occurrences (id,org_id,project_id,milestone_id,period_key,due_on,description,amount_cents,currency)
         SELECT id,$2,project_id,milestone_id,period_key,due_on,description,amount_cents,currency
         FROM unnest($1::uuid[],$3::uuid[],$4::uuid[],$5::text[],$6::date[],$7::text[],$8::bigint[],$9::text[])
           AS fees(id,project_id,milestone_id,period_key,due_on,description,amount_cents,currency)
         ON CONFLICT (project_id,period_key) DO NOTHING",
        &ids, org_id, &project_ids, &milestone_ids as &[Option<Uuid>], &keys as &[&str],
        &dates as &[NaiveDate], &descriptions as &[&str], &amounts, &currencies as &[&str],
    ).execute(&mut **tx).await.map_err(server_err)?;

    let fees = sqlx::query_as!(
        FeeLine,
        "SELECT f.id,f.description,f.amount_cents,f.currency
         FROM project_fee_occurrences f JOIN projects p ON p.id = f.project_id
         WHERE f.org_id = $1 AND p.client_id = $2 AND p.project_type = 'fixed_fee'
           AND f.invoice_id IS NULL AND f.due_on <= $4
           AND (f.period_key NOT LIKE 'month:%' OR f.due_on >= $3)
         ORDER BY f.due_on,f.project_id,f.period_key LIMIT 10001 FOR UPDATE OF f",
        org_id,
        client_id,
        from as NaiveDate,
        to as NaiveDate,
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(server_err)?;
    if fees.len() > MAX_FEE_OCCURRENCES {
        return Err(conflict(
            "Too many fees for one invoice; select a shorter period.",
        ));
    }
    Ok(fees)
}
