use horae_core::types::InvoiceStatus;
use uuid::Uuid;

use super::{PREPARE, authorize_prepare, authorize_read, balances, defaults};
use crate::models::invoice::{
    InvoiceDefaults, InvoiceDraftEdit, InvoiceDraftSave, InvoiceEditReview, InvoiceEditor,
    InvoiceExcessConfirmation, InvoiceFeeEdit, InvoiceFeeReview,
};
use crate::models::{Invoice, InvoiceWithLines};
use crate::ops::{Actor, Done, OpError, Tx};

async fn snapshot(
    tx: &mut Tx,
    org_id: Uuid,
    id: Uuid,
) -> Result<(Invoice, i64, Vec<balances::BalanceLine>), OpError> {
    super::refuse_imported(tx, org_id, id).await?;
    let invoice = super::fetch_invoice_metadata(&mut **tx, id, org_id)
        .await?
        .ok_or_else(|| OpError::not_found("Invoice not found"))?;
    if invoice.status != InvoiceStatus::Draft {
        return Err(OpError::conflict("Only draft invoices can be edited"));
    }
    let revision = sqlx::query_scalar!(
        "SELECT edit_revision FROM invoices WHERE id=$1 AND org_id=$2",
        id,
        org_id
    )
    .fetch_one(&mut **tx)
    .await?;
    let rows = balances::read_lines(tx, org_id, id).await?;
    if rows
        .iter()
        .try_fold(0_i64, |sum, row| sum.checked_add(row.amount_cents))
        != Some(invoice.subtotal_cents)
    {
        return Err(OpError::conflict(
            "Invoice lines do not match its saved subtotal; editing is unavailable.",
        ));
    }
    Ok((invoice, revision, rows))
}

/// A draft's editable values and fee lines, reviewed against current fee
/// balances.
pub async fn load(tx: &mut Tx, actor: &Actor, id: Uuid) -> Result<InvoiceEditor, OpError> {
    authorize_read(actor, PREPARE)?;
    let (invoice, revision, mut rows) = snapshot(tx, actor.org_id(), id).await?;
    let edit = InvoiceDraftEdit {
        revision,
        defaults: InvoiceDefaults {
            terms_days: invoice.terms_days as i16,
            po_number: invoice.po_number,
            discount_bps: invoice.discount_bps,
            tax1_bps: invoice.tax1_bps,
            tax2_name: invoice.tax2_name,
            tax2_bps: invoice.tax2_bps,
        },
        fees: rows
            .iter()
            .filter(|row| row.fee_id.is_some())
            .map(|row| InvoiceFeeEdit {
                line_id: row.id,
                description: row.description.clone(),
                amount_cents: row.amount_cents,
            })
            .collect(),
        subject: invoice.subject,
    };
    let (review, _) = evaluate(&mut rows, &edit)?;
    Ok(InvoiceEditor {
        edit,
        review,
        currency: invoice.currency,
    })
}

/// Reviews an edit of a draft without saving it.
pub async fn review(
    tx: &mut Tx,
    actor: &Actor,
    id: Uuid,
    edit: &InvoiceDraftEdit,
) -> Result<InvoiceEditReview, OpError> {
    authorize_read(actor, PREPARE)?;
    let (_, revision, mut rows) = snapshot(tx, actor.org_id(), id).await?;
    if revision != edit.revision {
        return Err(OpError::conflict(
            "This invoice changed. Reload it before reviewing your edits.",
        ));
    }
    let (review, _) = evaluate(&mut rows, edit)?;
    Ok(review)
}

fn evaluate(
    rows: &mut [balances::BalanceLine],
    edit: &InvoiceDraftEdit,
) -> Result<(InvoiceEditReview, Vec<i64>), OpError> {
    if edit.subject.chars().count() > 200 || edit.subject.contains('\0') {
        return Err(OpError::bad_input(
            "The subject must contain at most 200 characters",
        ));
    }
    let edits: std::collections::BTreeMap<_, _> =
        edit.fees.iter().map(|fee| (fee.line_id, fee)).collect();
    if edits.len() != edit.fees.len()
        || edits.len() != rows.iter().filter(|row| row.fee_id.is_some()).count()
    {
        return Err(OpError::bad_input(
            "Supply each existing fee line exactly once",
        ));
    }
    for row in rows.iter_mut().filter(|row| row.fee_id.is_some()) {
        let fee = edits
            .get(&row.id)
            .ok_or_else(|| OpError::bad_input("Fee line does not belong to this invoice"))?;
        if fee.description.trim().is_empty()
            || fee.description.chars().count() > 1000
            || fee.description.contains('\0')
        {
            return Err(OpError::bad_input(
                "Fee descriptions must contain 1–1000 characters without NUL",
            ));
        }
        // Existing zero-valued sources remain editable without inventing a charge.
        if fee.amount_cents < 0 || (fee.amount_cents == 0 && row.amount_cents != 0) {
            return Err(OpError::bad_input(
                "An invoiced fee amount must be positive",
            ));
        }
        row.amount_cents = fee.amount_cents;
        row.description = fee.description.trim().to_owned();
    }
    let gross: Vec<_> = rows.iter().map(|row| row.amount_cents).collect();
    let net = balances::allocate(&gross, edit.defaults.discount_bps)?;
    let subtotal = rows
        .iter()
        .try_fold(0_i64, |sum, row| sum.checked_add(row.amount_cents))
        .ok_or_else(|| OpError::conflict("Invoice amount exceeds the supported range"))?;
    let amounts = defaults::amounts(&edit.defaults, subtotal)?;
    let mut fees = Vec::with_capacity(edits.len());
    for (row, net_cents) in rows
        .iter()
        .zip(&net)
        .filter(|(row, _)| row.fee_id.is_some())
    {
        let (Some(agreed_cents), Some(project_id), Some(period_key)) =
            (row.agreed_cents, row.project_id, row.period_key.as_ref())
        else {
            return Err(OpError::Internal(
                "Invoice fee source is unavailable".into(),
            ));
        };
        let remaining_cents = row
            .other_invoiced_cents
            .checked_add(*net_cents)
            .and_then(|invoiced| agreed_cents.checked_sub(invoiced))
            .ok_or_else(|| OpError::conflict("Fee balance exceeds the supported range"))?;
        let excess_cents = if remaining_cents < 0 {
            remaining_cents
                .checked_neg()
                .ok_or_else(|| OpError::conflict("Fee excess exceeds the supported range"))?
        } else {
            0
        };
        fees.push(InvoiceFeeReview {
            line_id: row.id,
            project_id,
            period_key: period_key.clone(),
            agreed_cents,
            other_invoiced_cents: row.other_invoiced_cents,
            net_cents: *net_cents,
            remaining_cents,
            excess_cents,
        });
    }
    Ok((InvoiceEditReview { fees, amounts }, net))
}

/// Saves a reviewed edit of a draft. A repeated request returns the invoice as
/// it saved it.
pub async fn save(
    tx: &mut Tx,
    actor: &Actor,
    id: Uuid,
    request: &InvoiceDraftSave,
) -> Result<Done<InvoiceWithLines>, OpError> {
    if request.request_id.get_version_num() != 7
        || request.edit.revision < 1
        || request.edit.fees.len() > 10000
    {
        return Err(OpError::bad_input(
            "Invalid invoice request identity, revision or line count",
        ));
    }
    let mut canonical = request.clone();
    canonical.edit.fees.sort_by_key(|fee| fee.line_id);
    canonical.review.fees.sort_by_key(|fee| fee.line_id);
    canonical.confirmed_excess.sort_by_key(|fee| fee.line_id);
    let payload = serde_json::to_value(&canonical).map_err(|error| {
        tracing::error!(%error, "Invoice edit did not serialize");
        OpError::Internal("Could not complete the request. Please retry.".into())
    })?;
    // The saved edit reprices fee lines against the same balances the editor
    // showed, as its review did.
    authorize_prepare(actor)?;
    let org_id = actor.org_id();
    let actor_id = actor.id();
    balances::lock_invoices(tx, org_id).await?;
    let size = sqlx::query_scalar!(
        r#"SELECT octet_length($1::jsonb::text) AS "size!""#,
        payload
    )
    .fetch_one(&mut **tx)
    .await?;
    if size > 262144 {
        return Err(OpError::bad_input("Invoice edit exceeds 256 KiB"));
    }
    sqlx::query_scalar!(
        "SELECT id FROM invoices WHERE id=$1 AND org_id=$2 FOR UPDATE",
        id,
        org_id
    )
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| OpError::not_found("Invoice not found"))?;
    let (invoice, revision, mut rows) = snapshot(tx, org_id, id).await?;
    if let Some(previous) = sqlx::query!("SELECT actor_id,invoice_id,completed_revision,payload FROM invoice_edit_requests WHERE id=$1 AND org_id=$2", request.request_id, org_id)
        .fetch_optional(&mut **tx).await? {
        if previous.actor_id != actor_id || previous.invoice_id != id || previous.payload != payload || previous.completed_revision != revision {
            return Err(OpError::conflict("This request or invoice changed. Reload the invoice before saving."));
        }
    } else {
        if revision != request.edit.revision { return Err(OpError::conflict("This invoice changed. Reload it before saving your edits.")); }
        let (mut review, net) = evaluate(&mut rows, &request.edit)?;
        review.fees.sort_by_key(|fee| fee.line_id);
        if review != canonical.review { return Err(OpError::conflict("Fee balances changed. Review the invoice again before saving.")); }
        let excess: Vec<_> = review.fees.iter().filter(|fee| fee.excess_cents > 0)
            .map(|fee| InvoiceExcessConfirmation { line_id: fee.line_id, excess_cents: fee.excess_cents }).collect();
        if excess != canonical.confirmed_excess { return Err(OpError::conflict("Confirm the exact amount exceeding the fee balance before saving.")); }
        let ids: Vec<_> = rows.iter().map(|row| row.id).collect();
        let descriptions: Vec<_> = rows.iter().map(|row| row.description.clone()).collect();
        let gross: Vec<_> = rows.iter().map(|row| row.amount_cents).collect();
        let discounts: Vec<_> = gross.iter().zip(net).map(|(gross, net)| gross - net).collect();
        sqlx::query!("UPDATE invoice_line_items l SET description=v.description,amount_cents=v.amount,allocated_discount_cents=v.discount
            FROM unnest($1::uuid[],$2::text[],$3::bigint[],$4::bigint[]) AS v(id,description,amount,discount)
            WHERE l.id=v.id AND l.invoice_id=$5", &ids, &descriptions, &gross, &discounts, id)
            .execute(&mut **tx).await?;
        let values = &request.edit.defaults;
        let due_on = horae_core::project::payment_due_date(invoice.issued_on, values.terms_days as u16).map_err(OpError::bad_input)?;
        let amounts = review.amounts;
        sqlx::query!("UPDATE invoices SET due_on=$3,po_number=$4,discount_bps=$5,tax1_bps=$6,
            tax2_name=$7,tax2_bps=$8,discount_cents=$9,tax1_cents=$10,tax2_cents=$11,total_cents=$12,subject=$13
            WHERE org_id=$1 AND id=$2", org_id,id,due_on as chrono::NaiveDate,values.po_number,values.discount_bps,
            values.tax1_bps,values.tax2_name,values.tax2_bps,amounts.discount_cents,amounts.tax1_cents,amounts.tax2_cents,amounts.total_cents,
            request.edit.subject.trim())
            .execute(&mut **tx).await?;
        sqlx::query!("INSERT INTO invoice_edit_requests (id,org_id,actor_id,invoice_id,completed_revision,payload)
            SELECT $1,$2,$3,id,edit_revision,$5 FROM invoices WHERE id=$4 AND org_id=$2",
            request.request_id,org_id,actor_id,id,payload).execute(&mut **tx).await?;
    }
    Ok(Done::new(super::with_lines(tx, org_id, id).await?))
}
