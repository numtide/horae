//! Kani proofs over whole input types: no panic or overflow, and exact
//! arithmetic, for every value rather than a sample. Built only by `cargo kani`
//! (the `kani` cfg), so normal builds and tests never see them.
//!
//! The string parsers (`duration::parse`, `time_of_day::parse`) have no proof
//! here: through `trim`, lowercasing and `str::parse`, `time_of_day` ran out
//! of 8 GB at three bytes and `duration` (a 60-step `i128` loop) ran past five
//! minutes at three. The proptest properties `parse_never_panics` and
//! `parse_stays_inside_the_day` in `tests/properties.rs` cover them instead.

use crate::invoice::{InvoiceAmountError, invoice_amounts, line_amount_cents};
use crate::project::Percentage;
use crate::types::RoundDir;

fn any_dir() -> RoundDir {
    match kani::any::<u8>() % 3 {
        0 => RoundDir::Up,
        1 => RoundDir::Down,
        _ => RoundDir::Nearest,
    }
}

fn any_percentage() -> Percentage {
    let basis_points: u16 = kani::any();
    kani::assume(basis_points <= 10_000);
    Percentage::try_from(basis_points).unwrap()
}

/// Rounding any `u32` never wraps: it lands on a multiple of the increment, on
/// the side its direction names, less than one increment away, or saturates to
/// `u32::MAX` when that multiple does not fit. Increments are boundary values:
/// a symbolic `u32` divisor exceeds 30 minutes.
#[kani::proof]
fn rounding_lands_on_the_named_side_for_every_u32() {
    let minutes: u32 = kani::any();
    let inc = match kani::any::<u8>() % 6 {
        0 => 1,
        1 => 2,
        2 => 7,
        3 => 15,
        4 => i16::MAX as u32,
        _ => u32::MAX,
    };
    let dir = any_dir();
    let rounded = u64::from(crate::rounding::round(minutes, inc, dir));
    let (minutes, inc) = (u64::from(minutes), u64::from(inc));
    // Saturation is right only when the multiple the direction requires, the
    // next one up, does not fit in `u32`.
    let needs_above = match dir {
        RoundDir::Up => minutes % inc != 0,
        RoundDir::Nearest => 2 * (minutes % inc) >= inc,
        RoundDir::Down => false,
    };
    if needs_above && (minutes / inc + 1) * inc > u64::from(u32::MAX) {
        assert!(rounded == u64::from(u32::MAX));
        return;
    }
    assert!(rounded.abs_diff(minutes) < inc);
    match dir {
        RoundDir::Up => assert!(rounded >= minutes),
        RoundDir::Down => assert!(rounded <= minutes),
        RoundDir::Nearest => {
            let gap = 2 * rounded.abs_diff(minutes);
            assert!(gap < inc || (gap == inc && rounded > minutes));
        }
    }
}

/// For every non-negative subtotal and any three percentages, invoice amounts
/// never panic or wrap (Kani's own overflow and panic checks), keep the
/// subtotal they were given and never make the discount negative.
#[kani::proof]
fn invoice_amounts_never_panic_for_every_subtotal() {
    let subtotal: i64 = kani::any();
    kani::assume(subtotal >= 0);
    let amounts = invoice_amounts(
        subtotal,
        any_percentage(),
        any_percentage(),
        any_percentage(),
    );
    kani::cover!(amounts.is_err());
    if let Ok(a) = amounts {
        assert!(a.subtotal_cents == subtotal && a.discount_cents >= 0);
    }
}

/// For every non-negative amount and any two tax rates, both taxes are
/// non-negative and the total is exactly the amount plus them, and it fails
/// only for amounts past a third of `i64::MAX`. With the discount zero the
/// amount is the taxable amount. After a discount, the next proof keeps that
/// non-negative only for subtotals up to 2^28 cents; above it, the proptest
/// properties sample it.
#[kani::proof]
fn invoice_taxes_are_non_negative_and_add_up_for_every_amount() {
    let taxable: i64 = kani::any();
    kani::assume(taxable >= 0);
    let (tax1, tax2) = (any_percentage(), any_percentage());
    match invoice_amounts(taxable, Percentage::ZERO, tax1, tax2) {
        Ok(a) => {
            kani::cover!();
            assert!(a.discount_cents == 0 && a.tax1_cents >= 0 && a.tax2_cents >= 0);
            assert!(
                i128::from(a.total_cents)
                    == i128::from(taxable) + i128::from(a.tax1_cents) + i128::from(a.tax2_cents)
            );
        }
        Err(e) => {
            kani::cover!();
            // Each tax is at most the amount, so only an amount past a third
            // of `i64::MAX` can overflow. The exact total in `i128` ran past
            // five minutes.
            assert!(e == InvoiceAmountError::Overflow && taxable > i64::MAX / 3);
        }
    }
}

/// For every subtotal up to 2^28 cents (2.68 million in a two-decimal
/// currency) and any discount, the discount stays within the subtotal and
/// leaves a non-negative taxable amount. `discount ≤ subtotal` needs CBMC to
/// bound a symbolic 128-bit quotient: 2^32 took two minutes and `i64` did not
/// finish in ten. The proptest properties cover the whole range and the
/// half-up rounding of every component.
#[kani::proof]
fn invoice_discount_stays_within_the_subtotal() {
    let subtotal: i64 = kani::any();
    kani::assume(0 <= subtotal && subtotal <= 1 << 28);
    let a = invoice_amounts(
        subtotal,
        any_percentage(),
        Percentage::ZERO,
        Percentage::ZERO,
    )
    .unwrap();
    assert!(0 <= a.discount_cents && a.discount_cents <= subtotal);
    assert!(a.total_cents == subtotal - a.discount_cents);
}

/// A line amount never panics for any `i64` rate and `i32` minutes, and for
/// stored non-negative inputs it is rate × minutes / 60 rounded half up.
#[kani::proof]
fn line_amount_is_half_up_for_every_input() {
    let (rate, minutes): (i64, i32) = (kani::any(), kani::any());
    if let Ok(cents) = line_amount_cents(rate, minutes)
        && rate >= 0
        && minutes >= 0
    {
        let error = i128::from(cents) * 60 - i128::from(rate) * i128::from(minutes);
        assert!(-29 <= error && error <= 30);
    }
}

/// The progress percentage never panics and stays within 0–100 for every
/// consumption and positive budget.
#[kani::proof]
fn used_percent_is_bounded_for_every_input() {
    let (consumed, budget): (i64, i64) = (kani::any(), kani::any());
    match crate::budget::used_percent(consumed, budget) {
        Some(percent) => assert!(budget > 0 && percent <= 100),
        None => assert!(budget <= 0),
    }
}

/// Snapping any `i32` minute to the drag grid never overflows and lands on a
/// non-negative multiple of the step.
#[kani::proof]
fn snap_never_overflows() {
    let step = crate::time_of_day::SNAP_STEP;
    let snapped = crate::time_of_day::snap(kani::any(), step);
    assert!(snapped >= 0 && snapped % i32::from(step) == 0);
}
