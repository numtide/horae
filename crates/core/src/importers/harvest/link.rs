//! Whether an imported time entry is linked to an imported invoice, marked
//! billed with no invoice, reopened or left alone (spec 025, research R-6).
//!
//! The import reads what Harvest says about the entry and what the entry is in
//! Horae now, and this table decides. Links Horae made (generation, or a
//! submission for approval) are never changed by an import.

use uuid::Uuid;

pub use super::types::LinkConflictKind;

/// What Harvest says about the entry's billing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Billing {
    NotBilled,
    /// Billed on a Harvest draft, which is not imported (Q1).
    Draft,
    /// Billed on a Harvest invoice imported as `invoice`. `voided_in_horae`
    /// is true while someone's void in Horae stands (FR-016).
    Imported {
        invoice: Uuid,
        voided_in_horae: bool,
    },
    /// Billed on an invoice Horae could not import or Harvest no longer lists.
    Missing,
}

/// What the entry is in Horae now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Current {
    /// Open or approved, with no invoice.
    Unbilled,
    Submitted,
    /// Invoiced with no invoice: only an import does that.
    BilledWithoutInvoice,
    OnImported(Uuid),
    /// On an invoice made in Horae.
    OnHorae,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkAction {
    Keep,
    /// Link to this imported invoice and freeze the rounding.
    Link(Uuid),
    /// Invoiced with no invoice, so Horae never offers it for billing (Q4).
    MarkBilled,
    /// Back to open with no invoice and no frozen rounding (Q7).
    Unlink,
    Conflict(LinkConflictKind),
}

pub fn decide(harvest: Billing, current: Current) -> LinkAction {
    use LinkAction::*;
    let billed = matches!(harvest, Billing::Imported { .. } | Billing::Missing);
    match (harvest, current) {
        (_, Current::OnHorae) if billed => Conflict(LinkConflictKind::BilledInHorae),
        (_, Current::OnHorae) => Keep,
        (
            Billing::Imported {
                voided_in_horae: false,
                ..
            }
            | Billing::Missing,
            Current::Submitted,
        ) => Conflict(LinkConflictKind::SubmittedInHorae),
        (_, Current::Submitted) => Keep,
        (Billing::Imported { invoice, .. }, Current::OnImported(on)) if on == invoice => Keep,
        (
            Billing::Imported {
                invoice,
                voided_in_horae: false,
            },
            _,
        ) => Link(invoice),
        // A void made in Horae stands until Harvest changes the invoice.
        (Billing::Imported { .. }, Current::Unbilled) => Keep,
        (Billing::Imported { .. }, _) => Unlink,
        (Billing::Missing, Current::BilledWithoutInvoice) => Keep,
        (Billing::Missing, _) => MarkBilled,
        (Billing::NotBilled | Billing::Draft, Current::Unbilled) => Keep,
        (Billing::NotBilled | Billing::Draft, _) => Unlink,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const I: Uuid = Uuid::from_u128(1);
    const J: Uuid = Uuid::from_u128(2);

    fn on(invoice: Uuid) -> Billing {
        Billing::Imported {
            invoice,
            voided_in_horae: false,
        }
    }

    fn voided(invoice: Uuid) -> Billing {
        Billing::Imported {
            invoice,
            voided_in_horae: true,
        }
    }

    #[test]
    fn every_row_of_the_table() {
        use Current::*;
        use LinkAction::*;
        for (harvest, current, expected) in [
            (on(I), Unbilled, Link(I)),
            (on(I), OnImported(I), Keep),
            (on(I), OnImported(J), Link(I)),
            (on(I), BilledWithoutInvoice, Link(I)),
            (voided(I), Unbilled, Keep),
            (voided(I), OnImported(I), Keep),
            (voided(I), OnImported(J), Unlink),
            (on(I), OnHorae, Conflict(LinkConflictKind::BilledInHorae)),
            (
                Billing::Missing,
                OnHorae,
                Conflict(LinkConflictKind::BilledInHorae),
            ),
            (Billing::Draft, OnHorae, Keep),
            (Billing::NotBilled, OnHorae, Keep),
            (Billing::Draft, Unbilled, Keep),
            (Billing::Draft, OnImported(I), Unlink),
            (Billing::Draft, BilledWithoutInvoice, Unlink),
            (Billing::Missing, Unbilled, MarkBilled),
            (Billing::Missing, OnImported(I), MarkBilled),
            (Billing::Missing, BilledWithoutInvoice, Keep),
            (Billing::NotBilled, OnImported(I), Unlink),
            (Billing::NotBilled, BilledWithoutInvoice, Unlink),
            (Billing::NotBilled, Unbilled, Keep),
            (
                on(I),
                Submitted,
                Conflict(LinkConflictKind::SubmittedInHorae),
            ),
            (
                Billing::Missing,
                Submitted,
                Conflict(LinkConflictKind::SubmittedInHorae),
            ),
            (voided(I), Submitted, Keep),
            (Billing::NotBilled, Submitted, Keep),
            (Billing::Draft, Submitted, Keep),
        ] {
            assert_eq!(
                decide(harvest, current),
                expected,
                "{harvest:?} over {current:?}"
            );
        }
    }

    fn invoice() -> impl Strategy<Value = Uuid> {
        (0u128..3).prop_map(Uuid::from_u128)
    }

    fn billing() -> impl Strategy<Value = Billing> {
        prop_oneof![
            Just(Billing::NotBilled),
            Just(Billing::Draft),
            Just(Billing::Missing),
            (invoice(), any::<bool>()).prop_map(|(invoice, voided_in_horae)| {
                Billing::Imported {
                    invoice,
                    voided_in_horae,
                }
            }),
        ]
    }

    fn current() -> impl Strategy<Value = Current> {
        prop_oneof![
            Just(Current::Unbilled),
            Just(Current::Submitted),
            Just(Current::BilledWithoutInvoice),
            Just(Current::OnHorae),
            invoice().prop_map(Current::OnImported),
        ]
    }

    proptest! {
        #[test]
        fn horae_links_and_submissions_are_never_changed(
            harvest in billing(),
            current in prop_oneof![Just(Current::OnHorae), Just(Current::Submitted)],
        ) {
            prop_assert!(matches!(
                decide(harvest, current),
                LinkAction::Keep | LinkAction::Conflict(_)
            ));
        }

        #[test]
        fn a_link_only_goes_to_the_invoice_harvest_names(
            harvest in billing(),
            current in current(),
        ) {
            if let LinkAction::Link(invoice) = decide(harvest, current) {
                prop_assert_eq!(
                    harvest,
                    Billing::Imported { invoice, voided_in_horae: false }
                );
            }
        }
    }
}
