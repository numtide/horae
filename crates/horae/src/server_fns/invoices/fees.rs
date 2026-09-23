use super::*;
use crate::models::invoice::{InvoiceFeeBalance, InvoicePreviewLine, InvoiceSource};
use chrono::{Datelike, Months, NaiveDate};
use horae_core::project::{MonthlyFeeDay, monthly_fee_date};
use uuid::Uuid;

const MAX_FEE_OCCURRENCES: usize = 10_000;

pub(super) struct FeeLine {
    pub id: Uuid,
    pub project_id: Uuid,
    pub due_on: NaiveDate,
    pub period_key: String,
    pub description: String,
    pub amount_cents: i64,
    pub agreed_cents: i64,
    pub available: bool,
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

pub(in crate::server_fns) async fn preview_fees(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: Uuid,
    client_id: Uuid,
    from: NaiveDate,
    to: NaiveDate,
    selected: Option<&[Uuid]>,
) -> Result<Vec<InvoicePreviewLine>, ServerFnError> {
    let mut planned = scheduled_fees(
        tx,
        org_id,
        client_id,
        from,
        to,
        selected,
        SourceRead::Preview,
    )
    .await?;
    let projects: Vec<_> = planned.iter().map(|fee| fee.project_id).collect();
    let keys: Vec<_> = planned.iter().map(|fee| fee.period_key.as_str()).collect();
    // Existing identities win even outside this period. A changed schedule
    // must not replace a stored agreement, including a settled occurrence.
    let existing = sqlx::query!(
        "SELECT f.project_id, f.period_key FROM project_fee_occurrences f
         JOIN unnest($2::uuid[], $3::text[]) AS wanted(project_id, period_key)
           ON f.project_id = wanted.project_id AND f.period_key = wanted.period_key
         WHERE f.org_id = $1",
        org_id,
        &projects,
        &keys as &[&str],
    )
    .fetch_all(&mut **tx)
    .await
    .map_err(server_err)?;
    let existing: std::collections::BTreeSet<_> = existing
        .into_iter()
        .map(|row| (row.project_id, row.period_key))
        .collect();
    planned.retain(|fee| !existing.contains(&(fee.project_id, fee.period_key.clone())));
    let mut fees: Vec<_> = planned
        .into_iter()
        .map(|fee| {
            let remaining = fee.amount_cents;
            (fee, remaining, true)
        })
        .collect();
    for fee in stored_fees(
        tx,
        org_id,
        client_id,
        from,
        to,
        selected,
        SourceRead::Preview,
    )
    .await?
    {
        fees.push((
            Occurrence {
                project_id: fee.project_id,
                milestone_id: None,
                period_key: fee.period_key,
                due_on: fee.due_on,
                description: fee.description,
                amount_cents: fee.agreed_cents,
                currency: fee.currency,
            },
            fee.amount_cents,
            fee.available,
        ));
    }
    if fees.len() > MAX_FEE_OCCURRENCES {
        return Err(conflict(
            "Too many fees for one invoice; select a shorter period.",
        ));
    }
    fees.sort_by(|(a, _, _), (b, _, _)| {
        (a.due_on, a.project_id, &a.period_key).cmp(&(b.due_on, b.project_id, &b.period_key))
    });
    fees.into_iter()
        .map(|(fee, remaining, available)| {
            Ok(InvoicePreviewLine {
                selected: available,
                source: InvoiceSource::Fee {
                    project_id: fee.project_id,
                    period_key: fee.period_key,
                },
                project_id: fee.project_id,
                currency: fee.currency,
                description: fee.description,
                minutes: None,
                rate_cents: None,
                amount_cents: remaining.max(0),
                fee_balance: Some(InvoiceFeeBalance {
                    agreed_cents: fee.amount_cents,
                    invoiced_cents: fee
                        .amount_cents
                        .checked_sub(remaining)
                        .ok_or_else(|| conflict("Fee balance exceeds the supported range"))?,
                    remaining_cents: remaining,
                }),
                net_before_tax_cents: None,
            })
        })
        .collect()
}

/// Prepare stable occurrences inside the invoice transaction. Only monthly
/// fees use the range's lower bound; overdue single fees and milestones remain
/// available while they have a balance. No schedule sends an invoice itself.
pub(super) async fn prepare_fees(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: Uuid,
    client_id: Uuid,
    from: NaiveDate,
    to: NaiveDate,
    selected: Option<&[Uuid]>,
    fee_selection: Option<&[crate::models::invoice::InvoiceFeeSelection]>,
) -> Result<Vec<FeeLine>, ServerFnError> {
    let mut occurrences = scheduled_fees(
        tx,
        org_id,
        client_id,
        from,
        to,
        selected,
        SourceRead::Generate,
    )
    .await?;
    if let Some(selection) = fee_selection {
        let selected_sources: std::collections::BTreeSet<_> = selection
            .iter()
            .filter(|fee| fee.selected)
            .map(|fee| &fee.source)
            .collect();
        occurrences.retain(|fee| {
            selected_sources.contains(&InvoiceSource::Fee {
                project_id: fee.project_id,
                period_key: fee.period_key.clone(),
            })
        });
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

    stored_fees(
        tx,
        org_id,
        client_id,
        from,
        to,
        selected,
        SourceRead::Generate,
    )
    .await
}

struct FeeProject {
    id: Uuid,
    name: String,
    currency: String,
    starts_on: NaiveDate,
    ends_on: Option<NaiveDate>,
    fee_mode: String,
    fee_amount_cents: Option<i64>,
    monthly_day: Option<String>,
}

struct Milestone {
    id: Uuid,
    name: String,
    due_on: NaiveDate,
    amount_cents: i64,
}

async fn scheduled_fees(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: Uuid,
    client_id: Uuid,
    from: NaiveDate,
    to: NaiveDate,
    selected: Option<&[Uuid]>,
    mode: SourceRead,
) -> Result<Vec<Occurrence>, ServerFnError> {
    if from > to {
        return Err(err(BAD_REQUEST, "Invoice period ends before it starts"));
    }
    macro_rules! projects_query {
        ($suffix:literal) => {
            sqlx::query_as!(
                FeeProject,
                r#"SELECT p.id, p.name, p.currency,
                  COALESCE(p.starts_on, (p.created_at AT TIME ZONE 'UTC')::date) as "starts_on!: NaiveDate",
                  p.ends_on as "ends_on: NaiveDate", s.fee_mode as "fee_mode!",
                  s.fee_amount_cents, s.monthly_day
           FROM projects p JOIN project_settings s ON s.project_id = p.id
           WHERE p.org_id = $1 AND p.client_id = $2 AND p.project_type = 'fixed_fee'
             AND s.fee_mode IS NOT NULL
             AND ($3::uuid[] IS NULL OR p.id = ANY($3))
           ORDER BY p.id LIMIT 1001 "# + $suffix,
                org_id, client_id, selected,
            ).fetch_all(&mut **tx).await.map_err(server_err)?
        };
    }
    let projects = match mode {
        SourceRead::Preview => projects_query!(""),
        SourceRead::Generate => projects_query!("FOR SHARE OF p, s"),
    };
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
                macro_rules! milestones_query {
                    ($suffix:literal) => {
                        sqlx::query_as!(
                            Milestone,
                            r#"SELECT id, name, due_on as "due_on: NaiveDate", amount_cents
                       FROM project_fee_milestones WHERE project_id = $1 AND org_id = $2 AND due_on <= $3
                       ORDER BY position "# + $suffix,
                            project.id, org_id, to as NaiveDate,
                        ).fetch_all(&mut **tx).await.map_err(server_err)?
                    };
                }
                let milestones = match mode {
                    SourceRead::Preview => milestones_query!(""),
                    SourceRead::Generate => milestones_query!("FOR SHARE"),
                };
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
    Ok(occurrences)
}

async fn stored_fees(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: Uuid,
    client_id: Uuid,
    from: NaiveDate,
    to: NaiveDate,
    selected: Option<&[Uuid]>,
    mode: SourceRead,
) -> Result<Vec<FeeLine>, ServerFnError> {
    macro_rules! query {
        ($suffix:literal) => {
            sqlx::query_as!(
                FeeLine,
                r#"SELECT f.id,f.project_id,f.description, f.amount_cents AS agreed_cents,
                   (f.amount_cents > billed.amount OR (f.amount_cents = 0 AND billed.lines = 0)) AS "available!",
                   (f.amount_cents::numeric - billed.amount)::bigint AS "amount_cents!",
                   f.currency,f.period_key,f.due_on as "due_on: NaiveDate"
         FROM project_fee_occurrences f JOIN projects p ON p.id = f.project_id
         CROSS JOIN LATERAL (
           SELECT COALESCE(sum(l.net_before_tax_cents::numeric), 0) AS amount, count(*) AS lines
           FROM invoice_line_items l JOIN invoices i ON i.id = l.invoice_id
           WHERE l.fee_occurrence_id = f.id AND i.org_id = f.org_id AND i.status <> 'void'
         ) billed
         WHERE f.org_id = $1 AND p.client_id = $2 AND p.project_type = 'fixed_fee'
           AND f.due_on <= $4
           AND (f.period_key NOT LIKE 'month:%' OR f.due_on >= $3)
           AND ($5::uuid[] IS NULL OR f.project_id = ANY($5))
         ORDER BY f.due_on,f.project_id,f.period_key LIMIT 10001 "# + $suffix,
                org_id,
                client_id,
                from as NaiveDate,
                to as NaiveDate,
                selected,
            )
            .fetch_all(&mut **tx)
            .await
            .map_err(server_err)?
        };
    }
    let fees = match mode {
        SourceRead::Preview => query!(""),
        SourceRead::Generate => query!("FOR UPDATE OF f"),
    };
    if fees.len() > MAX_FEE_OCCURRENCES {
        return Err(conflict(
            "Too many fees for one invoice; select a shorter period.",
        ));
    }
    Ok(fees)
}
