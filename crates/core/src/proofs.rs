//! Kani proofs over whole input types: no panic or overflow, and exact
//! arithmetic, for every value rather than a sample. Built only by `cargo kani`
//! (the `kani` cfg), so normal builds and tests never see them.

use crate::invoice::{invoice_amounts, line_amount_cents};
use crate::money::{AddError, Money, add};
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

/// A string of at most `N` arbitrary bytes, when they are valid UTF-8.
fn any_str<const N: usize>(bytes: &[u8; N]) -> Option<&str> {
    let len: usize = kani::any();
    kani::assume(len <= N);
    core::str::from_utf8(&bytes[..len]).ok()
}

fn any_currency() -> [u8; 3] {
    if kani::any() { *b"EUR" } else { *b"USD" }
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

/// Money addition is exact or refused: it succeeds only within one currency
/// and returns the true sum, and an `i64` overflow is an error, never a wrap.
#[kani::proof]
fn money_addition_is_exact_or_refused() {
    let a = Money::new(kani::any(), any_currency());
    let b = Money::new(kani::any(), any_currency());
    let exact = i128::from(a.cents) + i128::from(b.cents);
    match add(a, b) {
        Ok(sum) => {
            assert!(a.currency == b.currency && sum.currency == a.currency);
            assert!(i128::from(sum.cents) == exact);
        }
        Err(AddError::CurrencyMismatch(_)) => assert!(a.currency != b.currency),
        Err(AddError::Overflow) => {
            assert!(a.currency == b.currency && i64::try_from(exact).is_err());
        }
    }
}

/// For every non-negative subtotal and valid percentages, invoice amounts
/// never panic, the discount stays within the subtotal and the total is
/// exactly the discounted subtotal plus both taxes.
#[kani::proof]
fn invoice_amounts_add_up_for_every_subtotal() {
    let subtotal: i64 = kani::any();
    kani::assume(subtotal >= 0);
    if let Ok(a) = invoice_amounts(
        subtotal,
        any_percentage(),
        any_percentage(),
        any_percentage(),
    ) {
        assert!(0 <= a.discount_cents && a.discount_cents <= subtotal);
        assert!(a.tax1_cents >= 0 && a.tax2_cents >= 0);
        assert!(
            i128::from(a.total_cents)
                == i128::from(subtotal) - i128::from(a.discount_cents)
                    + i128::from(a.tax1_cents)
                    + i128::from(a.tax2_cents)
        );
    }
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

/// Parsing any short string as a time of day never panics and stays in the day.
#[kani::proof]
#[kani::unwind(9)]
fn time_of_day_parse_never_panics() {
    let bytes: [u8; 7] = kani::any();
    if let Some(input) = any_str(&bytes)
        && let Some(minute) = crate::time_of_day::parse(input)
    {
        assert!(minute < crate::time_of_day::DAY_MINUTES);
    }
}

/// Parsing any short string as a duration never panics.
#[kani::proof]
#[kani::unwind(62)]
fn duration_parse_never_panics() {
    let bytes: [u8; 4] = kani::any();
    if let Some(input) = any_str(&bytes) {
        let _ = crate::duration::parse(input);
    }
}
