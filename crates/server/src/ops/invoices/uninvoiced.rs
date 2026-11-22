//! Clients with work still to invoice: unbilled time and fees due up to today.

use std::collections::BTreeMap;

use chrono::NaiveDate;

use super::{PREPARE, SourceRead, authorize_read, entries, fees};
use crate::models::invoice::{CurrencyAmount, InvoicePreviewLine, UninvoicedClient};
use crate::ops::{Actor, OpError, Tx};

/// Every client with unbilled time or fees due up to the organization's
/// today, as preparing an invoice for it from the earliest date would find
/// them. Like a preview, it prices time from current billable rates.
///
/// shortcut: one preview per candidate client, which is fine for a
/// catalogue of tens of clients; aggregate in SQL if that grows to hundreds.
pub async fn list(tx: &mut Tx, actor: &Actor) -> Result<Vec<UninvoicedClient>, OpError> {
    authorize_read(actor, PREPARE)?;
    let org_id = actor.org_id();
    let today = crate::timezone::today_for_org(&mut **tx, org_id).await?;
    // The same open start as the new invoice's "Everything up to today";
    // `NaiveDate::MIN` is outside PostgreSQL's date range.
    let start = NaiveDate::from_ymd_opt(1970, 1, 1)
        .ok_or_else(|| OpError::Internal("Invalid start date".into()))?;
    let period = (start, today);
    // Candidates only: clients with any unbilled time, or with a fixed-fee
    // schedule. The previews below decide what is really due.
    let candidates = sqlx::query!(
        r#"SELECT c.id, c.name FROM clients c
           WHERE c.org_id = $1 AND (
             EXISTS (SELECT 1 FROM time_entries te JOIN projects p ON p.id = te.project_id
                     WHERE p.client_id = c.id AND te.org_id = $1 AND te.invoice_id IS NULL
                       AND te.billable AND te.state IN ('open', 'approved') AND te.spent_date <= $2)
             OR EXISTS (SELECT 1 FROM projects p JOIN project_settings s ON s.project_id = p.id
                        WHERE p.client_id = c.id AND p.project_type = 'fixed_fee'
                          AND s.fee_mode IS NOT NULL))
           ORDER BY c.name, c.id"#,
        org_id,
        today as NaiveDate,
    )
    .fetch_all(&mut **tx)
    .await?;
    let mut clients = Vec::new();
    for client in candidates {
        let mut lines: Vec<InvoicePreviewLine> =
            entries::read(tx, org_id, client.id, period, None, SourceRead::Preview)
                .await?
                .iter()
                .map(entries::EntryWithRates::preview)
                .collect::<Result<_, _>>()?;
        lines.extend(fees::preview_fees(tx, org_id, client.id, period.0, period.1, None).await?);
        if let Some(found) = summarize(client.id, client.name, &lines)? {
            clients.push(found);
        }
    }
    Ok(clients)
}

/// The client's row, or `None` when nothing selected is left to bill.
fn summarize(
    client_id: uuid::Uuid,
    client_name: String,
    lines: &[InvoicePreviewLine],
) -> Result<Option<UninvoicedClient>, OpError> {
    let mut unbilled_minutes = 0_i64;
    let mut time = BTreeMap::<String, i64>::new();
    let mut fees = BTreeMap::<String, i64>::new();
    let overflow = || OpError::conflict("Uninvoiced amounts exceed the supported range");
    for line in lines.iter().filter(|line| line.selected) {
        let sums = match line.minutes {
            Some(minutes) => {
                unbilled_minutes += i64::from(minutes);
                &mut time
            }
            None if line.amount_cents > 0 => &mut fees,
            None => continue,
        };
        let sum = sums.entry(line.currency.trim().to_owned()).or_default();
        *sum = sum.checked_add(line.amount_cents).ok_or_else(overflow)?;
    }
    if unbilled_minutes == 0 && fees.is_empty() {
        return Ok(None);
    }
    let amounts = |sums: BTreeMap<String, i64>| {
        sums.into_iter()
            .map(|(currency, cents)| CurrencyAmount { currency, cents })
            .collect()
    };
    Ok(Some(UninvoicedClient {
        client_id,
        client_name,
        unbilled_minutes,
        time: amounts(time),
        fees: amounts(fees),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::invoice::{InvoiceFeeBalance, InvoiceSource};
    use uuid::Uuid;

    fn line(
        minutes: Option<i32>,
        cents: i64,
        currency: &str,
        selected: bool,
    ) -> InvoicePreviewLine {
        let project_id = Uuid::now_v7();
        InvoicePreviewLine {
            source: match minutes {
                Some(_) => InvoiceSource::Time {
                    entry_id: Uuid::now_v7(),
                },
                None => InvoiceSource::Fee {
                    project_id,
                    period_key: "single".into(),
                },
            },
            selected,
            project_id,
            currency: currency.into(),
            description: "Line".into(),
            minutes,
            rate_cents: minutes.map(|_| 6000),
            amount_cents: cents,
            fee_balance: minutes.is_none().then_some(InvoiceFeeBalance {
                agreed_cents: cents,
                invoiced_cents: 0,
                remaining_cents: cents,
            }),
            net_before_tax_cents: None,
        }
    }

    #[test]
    fn sums_selected_time_and_fees_per_currency() {
        let lines = [
            line(Some(90), 9000, "EUR", true),
            line(Some(30), 3000, "USD", true),
            line(None, 50000, "EUR", true),
            line(None, 70000, "EUR", false),
        ];
        let client = summarize(Uuid::now_v7(), "Acme".into(), &lines)
            .unwrap()
            .unwrap();
        assert_eq!(client.unbilled_minutes, 120);
        assert_eq!(
            client.time,
            [
                CurrencyAmount {
                    currency: "EUR".into(),
                    cents: 9000
                },
                CurrencyAmount {
                    currency: "USD".into(),
                    cents: 3000
                },
            ]
        );
        assert_eq!(
            client.fees,
            [CurrencyAmount {
                currency: "EUR".into(),
                cents: 50000
            }]
        );
    }

    #[test]
    fn a_client_with_only_settled_fees_has_no_row() {
        let lines = [line(None, 0, "EUR", true), line(None, 100, "EUR", false)];
        assert_eq!(
            summarize(Uuid::now_v7(), "Acme".into(), &lines).unwrap(),
            None
        );
    }
}
