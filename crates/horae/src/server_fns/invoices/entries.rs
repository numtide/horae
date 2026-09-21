use super::*;

pub(super) struct EntryWithRates {
    pub(super) entry_id: uuid::Uuid,
    pub(super) project_id: uuid::Uuid,
    pub(super) minutes: i32,
    pub(super) project_name: String,
    pub(super) task_name: String,
    pub(super) notes: Option<String>,
    pub(super) spent_date: chrono::NaiveDate,
    pub(super) rate_cents: Option<i64>,
    pub(super) currency: String,
}

pub(super) async fn read(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    org_id: uuid::Uuid,
    client_id: uuid::Uuid,
    (from, to): (chrono::NaiveDate, chrono::NaiveDate),
    selected: Option<&[uuid::Uuid]>,
    mode: SourceRead,
) -> Result<Vec<EntryWithRates>, ServerFnError> {
    // Both reads share eligibility and rate rules; only generation locks sources.
    macro_rules! query {
        ($suffix:literal) => {
            sqlx::query_as!(
                EntryWithRates,
                r#"SELECT
             te.id as entry_id,
             te.project_id,
             effective_minutes(te.minutes, te.rounded_minutes, o.round_minutes, o.round_dir) as "minutes!",
             p.name as project_name,
             t.name as task_name,
             te.notes,
             te.spent_date as "spent_date: chrono::NaiveDate",
             resolve_project_rate(ps.rate_mode, pt.rate_cents, a.rate_cents, p.rate_cents,
               CASE WHEN ps.project_id IS NULL OR p.currency = o.default_currency THEN u.billable_rate_cents END,
               CASE WHEN p.currency = c.currency THEN c.default_rate_cents END
             ) as "rate_cents?",
             CASE WHEN ps.project_id IS NULL THEN c.currency ELSE p.currency END as "currency!"
           FROM time_entries te
           JOIN projects p ON p.id = te.project_id
           JOIN clients c ON c.id = p.client_id
           LEFT JOIN project_settings ps ON ps.project_id = p.id
           JOIN tasks t ON t.id = te.task_id
           LEFT JOIN project_tasks pt ON pt.project_id = te.project_id AND pt.task_id = te.task_id
           LEFT JOIN assignments a ON a.project_id = te.project_id AND a.user_id = te.user_id
           JOIN users u ON u.id = te.user_id
           JOIN organizations o ON o.id = te.org_id
           WHERE te.org_id = $1
             AND p.client_id = $2
             AND te.billable = true
             AND p.project_type <> 'non_billable'
             AND (ps.project_id IS NULL OR p.project_type = 'time_and_materials')
             AND COALESCE(pt.billable, t.billable_default)
             AND NOT te.is_running
             AND te.invoice_id IS NULL
             AND te.state IN ('open', 'approved')
             AND te.spent_date >= $3
             AND te.spent_date <= $4
             AND ($5::uuid[] IS NULL OR p.id = ANY($5))
           ORDER BY te.spent_date, te.id
           "# + $suffix,
                org_id,
                client_id,
                from as chrono::NaiveDate,
                to as chrono::NaiveDate,
                selected,
            )
            .fetch_all(&mut **tx)
            .await
            .map_err(server_err)?
        };
    }
    let entries = match mode {
        SourceRead::Preview => query!("LIMIT 10001"),
        SourceRead::Generate => query!("FOR UPDATE OF te"),
    };
    if matches!(mode, SourceRead::Preview) && entries.len() > 10_000 {
        return Err(conflict(
            "Too much time for one preview; select fewer projects or a shorter period.",
        ));
    }
    Ok(entries)
}

impl EntryWithRates {
    pub(super) fn amount(&self) -> Result<i64, ServerFnError> {
        horae_core::invoice::line_amount_cents(self.rate_cents.unwrap_or(0), self.minutes)
            .map_err(|_| conflict("Invoice line amount exceeds the supported range."))
    }

    pub(super) fn description(&self) -> String {
        match &self.notes {
            Some(notes) => format!(
                "{} — {} ({}): {notes}",
                self.spent_date, self.project_name, self.task_name
            ),
            None => format!(
                "{} — {} ({})",
                self.spent_date, self.project_name, self.task_name
            ),
        }
    }
}
