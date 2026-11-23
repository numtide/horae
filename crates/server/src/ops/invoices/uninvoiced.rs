//! Clients with work still to invoice: unbilled time and fees due up to today.

use chrono::NaiveDate;
use uuid::Uuid;

use super::{PREPARE, READ, authorize_read};
use crate::models::invoice::{CurrencyAmount, LastInvoice, UninvoicedClient};
use crate::ops::{Actor, OpError, Tx};

/// One per-currency sum of a client's unbilled time (`kind = 'time'`) or of
/// its fees due (`'fee'`), with the client's last invoice.
struct Sum {
    client_id: Uuid,
    client_name: String,
    kind: String,
    currency: String,
    minutes: i64,
    cents: i64,
    last_id: Option<Uuid>,
    last_number: Option<String>,
    last_issued_on: Option<NaiveDate>,
    last_covered_to: Option<NaiveDate>,
}

/// Every active client with unbilled time or fees due up to the
/// organization's today, as preparing an invoice from the earliest date would
/// find them, in one read. Like a preview, it prices time from current
/// billable rates.
///
/// Time is eligible on the same conditions as `entries::read` (billable, not
/// running, not invoiced, open or approved), and fees on those of
/// `fees::preview_fees`: an occurrence already stored is due for what its
/// non-void invoices have not billed, and a scheduled one not stored yet for
/// its whole amount. A change to what can be invoiced changes all three.
pub async fn list(tx: &mut Tx, actor: &Actor) -> Result<Vec<UninvoicedClient>, OpError> {
    // Prices work and names each client's last invoice.
    authorize_read(actor, PREPARE)?;
    authorize_read(actor, READ)?;
    let org_id = actor.org_id();
    let today = crate::timezone::today_for_org(&mut **tx, org_id).await?;
    let rows = sqlx::query_as!(
        Sum,
        r#"WITH unbilled_time AS (
             SELECT p.client_id,
                    CASE WHEN ps.project_id IS NULL THEN c.currency ELSE p.currency END AS currency,
                    sum(m.minutes)::bigint AS minutes,
                    sum(line_amount_cents(COALESCE(resolve_project_rate(ps.rate_mode, pt.rate_cents, a.rate_cents, p.rate_cents,
                          CASE WHEN ps.project_id IS NULL OR p.currency = o.default_currency THEN u.billable_rate_cents END,
                          CASE WHEN p.currency = c.currency THEN c.default_rate_cents END), 0), m.minutes,
                          CASE WHEN ps.project_id IS NULL THEN c.currency ELSE p.currency END))::bigint AS cents
             FROM time_entries te
             JOIN projects p ON p.id = te.project_id
             JOIN clients c ON c.id = p.client_id
             LEFT JOIN project_settings ps ON ps.project_id = p.id
             JOIN tasks t ON t.id = te.task_id
             LEFT JOIN project_tasks pt ON pt.project_id = te.project_id AND pt.task_id = te.task_id
             LEFT JOIN assignments a ON a.project_id = te.project_id AND a.user_id = te.user_id
             JOIN users u ON u.id = te.user_id
             JOIN organizations o ON o.id = te.org_id
             CROSS JOIN LATERAL (SELECT effective_minutes(te.minutes, te.rounded_minutes, o.round_minutes, o.round_dir) AS minutes) m
             WHERE te.org_id = $1
               AND te.billable = true
               AND p.project_type <> 'non_billable'
               AND (ps.project_id IS NULL OR p.project_type = 'time_and_materials')
               AND COALESCE(pt.billable, t.billable_default)
               AND NOT te.is_running
               AND te.invoice_id IS NULL
               AND te.state IN ('open', 'approved')
               AND te.spent_date <= $2
             GROUP BY 1, 2
           ),
           fee_projects AS (
             SELECT p.id, p.client_id, p.currency, s.fee_mode, s.fee_amount_cents, s.monthly_day,
                    COALESCE(p.starts_on, (p.created_at AT TIME ZONE 'UTC')::date) AS starts_on,
                    LEAST(COALESCE(p.ends_on, $2), $2) AS ends_on
             FROM projects p JOIN project_settings s ON s.project_id = p.id
             WHERE p.org_id = $1 AND p.project_type = 'fixed_fee' AND s.fee_mode IS NOT NULL
           ),
           scheduled AS (
             SELECT f.id AS project_id, f.client_id, f.currency, 'single' AS period_key, f.fee_amount_cents AS amount
             FROM fee_projects f WHERE f.fee_mode = 'single' AND f.starts_on <= $2
             UNION ALL
             SELECT f.id, f.client_id, f.currency, 'milestone:' || m.id, m.amount_cents
             FROM fee_projects f JOIN project_fee_milestones m ON m.project_id = f.id AND m.org_id = $1
             WHERE f.fee_mode = 'milestones' AND m.due_on <= $2
             UNION ALL
             SELECT f.id, f.client_id, f.currency, 'month:' || to_char(month, 'YYYY-MM'), f.fee_amount_cents
             FROM fee_projects f
             CROSS JOIN LATERAL generate_series(date_trunc('month', f.starts_on), date_trunc('month', f.ends_on), interval '1 month') AS month
             CROSS JOIN LATERAL (SELECT CASE f.monthly_day
                 WHEN 'first' THEN month::date
                 WHEN 'fifteenth' THEN month::date + 14
                 WHEN 'last' THEN (month + interval '1 month' - interval '1 day')::date END AS due_on) d
             WHERE f.fee_mode = 'monthly' AND d.due_on BETWEEN f.starts_on AND f.ends_on
           ),
           fees_due AS (
             SELECT s.client_id, s.currency, s.amount::numeric AS cents
             FROM scheduled s
             WHERE s.amount > 0 AND NOT EXISTS (
               SELECT 1 FROM project_fee_occurrences o
               WHERE o.org_id = $1 AND o.project_id = s.project_id AND o.period_key = s.period_key)
             UNION ALL
             SELECT p.client_id, f.currency, f.amount_cents::numeric - billed.amount
             FROM project_fee_occurrences f JOIN projects p ON p.id = f.project_id
             CROSS JOIN LATERAL (
               SELECT COALESCE(sum(l.net_before_tax_cents::numeric), 0) AS amount
               FROM invoice_line_items l JOIN invoices i ON i.id = l.invoice_id
               WHERE l.fee_occurrence_id = f.id AND i.org_id = f.org_id AND i.status <> 'void'
             ) billed
             WHERE f.org_id = $1 AND p.project_type = 'fixed_fee' AND f.due_on <= $2
               AND f.amount_cents > billed.amount
           ),
           sums AS (
             SELECT client_id, 'time' AS kind, currency, minutes, cents FROM unbilled_time
             UNION ALL
             SELECT client_id, 'fee', currency, 0::bigint, sum(cents)::bigint FROM fees_due GROUP BY 1, 2, 3
           )
           SELECT c.id AS "client_id!", c.name AS "client_name!", s.kind AS "kind!",
                  btrim(s.currency) AS "currency!", s.minutes AS "minutes!", s.cents AS "cents!",
                  last.id AS "last_id?", last.number AS "last_number?",
                  last.issued_on AS "last_issued_on?: NaiveDate", last.covered_to AS "last_covered_to?: NaiveDate"
           FROM sums s JOIN clients c ON c.id = s.client_id AND c.org_id = $1 AND c.active
           LEFT JOIN LATERAL (
             SELECT i.id, i.number, i.issued_on,
                    COALESCE(GREATEST(max(te.spent_date), max(f.due_on),
                      (SELECT max(linked.spent_date) FROM time_entries linked WHERE linked.invoice_id = i.id)),
                      i.issued_on) AS covered_to
             FROM invoices i
             LEFT JOIN invoice_line_items l ON l.invoice_id = i.id
             LEFT JOIN time_entries te ON te.id = l.time_entry_id
             LEFT JOIN project_fee_occurrences f ON f.id = l.fee_occurrence_id
             WHERE i.org_id = $1 AND i.client_id = c.id AND i.status <> 'void'
             GROUP BY i.id
             ORDER BY i.issued_on DESC, i.created_at DESC, i.id DESC
             LIMIT 1
           ) last ON true
           ORDER BY c.name, c.id, s.kind DESC, s.currency"#,
        org_id,
        today as NaiveDate,
    )
    .fetch_all(&mut **tx)
    .await?;
    Ok(collect(rows))
}

/// Folds the per-currency rows into one entry per client, in the order read,
/// dropping a client with neither time nor a fee to bill.
fn collect(rows: Vec<Sum>) -> Vec<UninvoicedClient> {
    let mut clients: Vec<UninvoicedClient> = Vec::new();
    for row in rows {
        if clients
            .last()
            .is_none_or(|last| last.client_id != row.client_id)
        {
            let last_invoice = match (
                row.last_id,
                row.last_number,
                row.last_issued_on,
                row.last_covered_to,
            ) {
                (Some(id), Some(number), Some(issued_on), Some(covered_to)) => Some(LastInvoice {
                    id,
                    number,
                    issued_on,
                    covered_to,
                }),
                _ => None,
            };
            clients.push(UninvoicedClient {
                client_id: row.client_id,
                client_name: row.client_name,
                unbilled_minutes: 0,
                time: Vec::new(),
                fees: Vec::new(),
                last_invoice,
            });
        }
        let Some(client) = clients.last_mut() else {
            continue;
        };
        let amount = CurrencyAmount {
            currency: row.currency,
            cents: row.cents,
        };
        if row.kind == "time" {
            client.unbilled_minutes += row.minutes;
            client.time.push(amount);
        } else if amount.cents > 0 {
            client.fees.push(amount);
        }
    }
    clients.retain(|client| client.unbilled_minutes > 0 || !client.fees.is_empty());
    clients
}

/// The client's latest invoice that is not void, and the last day it billed:
/// where the next invoice for the client starts. An invoice imported from
/// Harvest has no lines for its time, so its linked entries count (FR-025).
pub async fn last_invoice(
    tx: &mut Tx,
    actor: &Actor,
    client_id: Uuid,
) -> Result<Option<LastInvoice>, OpError> {
    authorize_read(actor, PREPARE)?;
    authorize_read(actor, READ)?;
    Ok(sqlx::query_as!(
        LastInvoice,
        r#"SELECT i.id, i.number, i.issued_on AS "issued_on: NaiveDate",
                  COALESCE(GREATEST(max(te.spent_date), max(f.due_on),
                    (SELECT max(linked.spent_date) FROM time_entries linked WHERE linked.invoice_id = i.id)),
                    i.issued_on) AS "covered_to!: NaiveDate"
           FROM invoices i
           LEFT JOIN invoice_line_items l ON l.invoice_id = i.id
           LEFT JOIN time_entries te ON te.id = l.time_entry_id
           LEFT JOIN project_fee_occurrences f ON f.id = l.fee_occurrence_id
           WHERE i.org_id = $1 AND i.client_id = $2 AND i.status <> 'void'
           GROUP BY i.id
           ORDER BY i.issued_on DESC, i.created_at DESC, i.id DESC
           LIMIT 1"#,
        actor.org_id(),
        client_id,
    )
    .fetch_optional(&mut **tx)
    .await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(client: Uuid, kind: &str, currency: &str, minutes: i64, cents: i64) -> Sum {
        Sum {
            client_id: client,
            client_name: "Acme".into(),
            kind: kind.into(),
            currency: currency.into(),
            minutes,
            cents,
            last_id: None,
            last_number: None,
            last_issued_on: None,
            last_covered_to: None,
        }
    }

    #[test]
    fn rows_fold_into_one_client_with_time_and_fees_per_currency() {
        let acme = Uuid::now_v7();
        let clients = collect(vec![
            row(acme, "time", "EUR", 90, 9000),
            row(acme, "time", "USD", 30, 3000),
            row(acme, "fee", "EUR", 0, 50000),
        ]);
        assert_eq!(clients.len(), 1);
        assert_eq!(clients[0].unbilled_minutes, 120);
        assert_eq!(clients[0].time.len(), 2);
        assert_eq!(
            clients[0].fees,
            [CurrencyAmount {
                currency: "EUR".into(),
                cents: 50000
            }]
        );
    }

    #[test]
    fn a_client_with_only_zero_minutes_has_no_row() {
        assert!(collect(vec![row(Uuid::now_v7(), "time", "EUR", 0, 0)]).is_empty());
    }
}
