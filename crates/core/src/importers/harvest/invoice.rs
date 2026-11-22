//! Harvest invoices: which ones Horae imports, under which status, and whether
//! their amounts add up exactly before anything is written (spec 025).

use crate::types::InvoiceStatus;

/// What the import does with a Harvest invoice of a given state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mapped {
    /// A draft: not imported, and its time stays open (Q1).
    Skip,
    Import(InvoiceStatus),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown invoice state {0:?}")]
pub struct UnknownState(pub String);

/// open → Sent, paid → Paid, closed (written off) → Closed (Q2), draft → skip.
pub fn map_state(state: &str) -> Result<Mapped, UnknownState> {
    match state {
        "draft" => Ok(Mapped::Skip),
        "open" => Ok(Mapped::Import(InvoiceStatus::Sent)),
        "paid" => Ok(Mapped::Import(InvoiceStatus::Paid)),
        "closed" => Ok(Mapped::Import(InvoiceStatus::Closed)),
        other => Err(UnknownState(other.to_owned())),
    }
}

/// An imported invoice's amounts in hundredths, as Harvest stated them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Amounts {
    pub discount: i64,
    pub tax1: i64,
    pub tax2: i64,
    pub total: i64,
    pub due: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AmountsError {
    #[error("the invoice total {0} is negative")]
    NegativeTotal(i64),
    #[error(
        "the lines, discount and taxes give {computed}, not the total {total} (a difference of {difference})"
    )]
    Mismatch {
        computed: i128,
        total: i64,
        difference: i128,
    },
    #[error("{0}")]
    OutOfRange(&'static str),
}

/// Checks, in integers, that the lines minus the discount plus both taxes
/// give the total, and that every amount fits what an invoice can hold: a
/// total of zero or more, a discount, taxes and lines subtotal of zero or
/// more, and an amount due between zero and the total. Lines alone may be
/// negative (credits).
pub fn check_amounts(lines: &[i64], amounts: Amounts) -> Result<(), AmountsError> {
    if amounts.total < 0 {
        return Err(AmountsError::NegativeTotal(amounts.total));
    }
    let subtotal: i128 = lines.iter().map(|&amount| i128::from(amount)).sum();
    if subtotal < 0 {
        return Err(AmountsError::OutOfRange(
            "the lines add up to less than zero",
        ));
    }
    if amounts.discount < 0 || amounts.tax1 < 0 || amounts.tax2 < 0 {
        return Err(AmountsError::OutOfRange("a discount or tax is negative"));
    }
    let computed = subtotal - i128::from(amounts.discount)
        + i128::from(amounts.tax1)
        + i128::from(amounts.tax2);
    if computed != i128::from(amounts.total) {
        return Err(AmountsError::Mismatch {
            computed,
            total: amounts.total,
            difference: computed - i128::from(amounts.total),
        });
    }
    if !(0..=amounts.total).contains(&amounts.due) {
        return Err(AmountsError::OutOfRange(
            "the amount due is not between zero and the total",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn states_map_to_horae_statuses() {
        assert_eq!(map_state("draft"), Ok(Mapped::Skip));
        assert_eq!(map_state("open"), Ok(Mapped::Import(InvoiceStatus::Sent)));
        assert_eq!(map_state("paid"), Ok(Mapped::Import(InvoiceStatus::Paid)));
        assert_eq!(
            map_state("closed"),
            Ok(Mapped::Import(InvoiceStatus::Closed))
        );
        assert_eq!(map_state("sent"), Err(UnknownState("sent".into())));
    }

    fn amounts(discount: i64, tax1: i64, tax2: i64, total: i64, due: i64) -> Amounts {
        Amounts {
            discount,
            tax1,
            tax2,
            total,
            due,
        }
    }

    #[test]
    fn amounts_that_add_up_pass_with_credit_lines() {
        // 1000 + 500 - 200 (credit) = 1300; -100 discount; +120 +30 taxes.
        assert_eq!(
            check_amounts(&[1000, 500, -200], amounts(100, 120, 30, 1350, 400)),
            Ok(())
        );
        assert_eq!(check_amounts(&[], amounts(0, 0, 0, 0, 0)), Ok(()));
    }

    #[test]
    fn a_one_cent_difference_is_reported_with_its_size() {
        assert_eq!(
            check_amounts(&[1000], amounts(0, 190, 0, 1189, 0)),
            Err(AmountsError::Mismatch {
                computed: 1190,
                total: 1189,
                difference: 1
            })
        );
    }

    #[test]
    fn negative_totals_and_out_of_range_amounts_are_refused() {
        assert_eq!(
            check_amounts(&[-500], amounts(0, 0, 0, -500, 0)),
            Err(AmountsError::NegativeTotal(-500))
        );
        for bad in [
            check_amounts(&[100], amounts(0, 0, 0, 100, 101)),
            check_amounts(&[100], amounts(0, 0, 0, 100, -1)),
            check_amounts(&[100], amounts(-10, 0, 0, 110, 0)),
            check_amounts(&[-100], amounts(0, 100, 0, 0, 0)),
        ] {
            assert!(matches!(bad, Err(AmountsError::OutOfRange(_))), "{bad:?}");
        }
    }

    #[test]
    fn sums_beyond_i64_do_not_overflow() {
        assert!(matches!(
            check_amounts(&[i64::MAX, i64::MAX], amounts(0, 0, 0, 0, 0)),
            Err(AmountsError::Mismatch { .. })
        ));
    }
}
