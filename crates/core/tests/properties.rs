//! Properties of horae-core's time and money arithmetic.
//!
//! Each property states its invariant and the bug it would catch. Small domains
//! are enumerated outright; the rest are sampled with proptest. Bounded proofs
//! over whole input types live in `src/proofs.rs` (`cargo kani`).

use chrono::{Datelike, Days, NaiveDate, Weekday};
use proptest::prelude::*;

const WEEKDAYS: [Weekday; 7] = [
    Weekday::Mon,
    Weekday::Tue,
    Weekday::Wed,
    Weekday::Thu,
    Weekday::Fri,
    Weekday::Sat,
    Weekday::Sun,
];

/// Inputs that exercise the parsers' separators, signs and digits far more
/// often than arbitrary Unicode does.
fn parser_input() -> impl Strategy<Value = String> {
    prop_oneof!["\\PC{0,40}", "[0-9:.+ \\-]{0,45}", "[0-9: apmAPM]{0,12}"]
}

mod duration {
    use super::*;
    use horae_core::duration::{format_decimal, format_hhmm, format_hours2, parse};
    use horae_core::importers::harvest::convert::{hours_to_minutes, money_to_cents};

    /// The Harvest API's wire hours ("H.dd", half up) read back to the same
    /// minutes for every duration up to 1,000 days, so a client that echoes an
    /// entry's hours never changes it. Enumerated: the domain is small.
    #[test]
    fn api_hours_round_trip_for_every_minute_up_to_a_thousand_days() {
        for minutes in 0..=1_440_000 {
            assert_eq!(hours_to_minutes(&format_hours2(minutes)), Ok(minutes));
        }
    }

    proptest! {
        /// Shown "H:MM" parses back to the same minutes, so saving an entry
        /// without touching its duration never changes it.
        #[test]
        fn hhmm_round_trips(minutes in any::<u32>()) {
            prop_assert_eq!(parse(&format_hhmm(minutes.into())).unwrap(), minutes);
        }

        /// Shown decimal hours ("1.5", at most two places) parse back to the
        /// same minutes: the display step is 0.3 minutes, under the half
        /// minute that would move a saved value.
        #[test]
        fn decimal_display_round_trips(minutes in any::<u32>()) {
            prop_assert_eq!(parse(&format_decimal(minutes.into())).unwrap(), minutes);
        }

        /// "H:MM", exact decimal hours "H.dd" and whole hours "H" agree on the
        /// minutes, so "1:15", "1.25" and "75" spellings cannot disagree.
        #[test]
        fn spellings_agree(minutes in (0..=u32::MAX / 3).prop_map(|k| k * 3)) {
            // Multiples of three minutes are exactly two decimal places of an hour.
            let decimal = format!("{}.{:02}", minutes / 60, minutes % 60 * 5 / 3);
            prop_assert_eq!(parse(&decimal).unwrap(), minutes);
            prop_assert_eq!(parse(&format_hhmm(minutes.into())).unwrap(), minutes);
            if minutes % 60 == 0 {
                prop_assert_eq!(parse(&(minutes / 60).to_string()).unwrap(), minutes);
            }
        }

        /// Decimal hours and amounts scale to round-half-up(value × 60) minutes
        /// and round-half-up(value × 100) cents, checked against an exact
        /// rational reference: catches a drift in the remainder loop that
        /// replaces the overflowing `fraction × scale` product.
        #[test]
        fn decimals_scale_with_exact_half_up_rounding(
            whole in 0..1_000_000_000_u128,
            fraction in "[0-9]{0,18}",
        ) {
            let text = format!("{whole}.{fraction}");
            let denominator = 10_u128.pow(fraction.len() as u32);
            let numerator = whole * denominator + fraction.parse::<u128>().unwrap_or(0);
            let half_up = |scale: u128| (2 * numerator * scale + denominator) / (2 * denominator);
            prop_assert_eq!(hours_to_minutes(&text).unwrap() as u128, half_up(60));
            prop_assert_eq!(money_to_cents(&text).unwrap() as u128, half_up(100));
            prop_assert_eq!(parse(&text).ok().map(u128::from), u32::try_from(half_up(60)).ok().map(u128::from));
        }

        /// No input string makes the duration parser panic.
        #[test]
        fn parse_never_panics(input in parser_input()) {
            let _ = parse(&input);
            let _ = hours_to_minutes(&input);
        }
    }
}

mod time_of_day {
    use super::*;
    use horae_core::time_of_day::{
        DAY_MINUTES, SNAP_STEP, clamp_to_day, format, format_12h, parse, snap,
    };

    /// Every minute of the day reads back from both its 24h and 12h display,
    /// which pins the 12am/12pm edges.
    #[test]
    fn every_minute_round_trips_through_24h_and_12h() {
        for minute in 0..DAY_MINUTES {
            assert_eq!(parse(&format(minute)), Some(minute), "{}", format(minute));
            assert_eq!(
                parse(&format_12h(minute)),
                Some(minute),
                "{}",
                format_12h(minute)
            );
        }
    }

    /// Snapping a start time lands on the nearest grid step and never moves a
    /// later time before an earlier one, so drags cannot reorder entries.
    #[test]
    fn snap_moves_each_minute_to_the_nearest_step_in_order() {
        let mut previous = 0;
        for minute in 0..i32::from(DAY_MINUTES) {
            let snapped = snap(minute, SNAP_STEP);
            assert_eq!(snapped % i32::from(SNAP_STEP), 0, "{minute}");
            assert!(
                2 * (snapped - minute).abs() < i32::from(SNAP_STEP),
                "{minute}"
            );
            assert!(snapped >= previous, "{minute}");
            previous = snapped;
        }
    }

    proptest! {
        /// Parsed times are always inside the day, for any input string.
        #[test]
        fn parse_stays_inside_the_day(input in parser_input()) {
            if let Some(minute) = parse(&input) {
                prop_assert!(minute < DAY_MINUTES);
            }
        }

        /// Clamping never lets an entry cross midnight and only shortens an
        /// entry that would.
        #[test]
        fn clamp_to_day_never_crosses_midnight(start in 0..=DAY_MINUTES, duration in any::<u32>()) {
            let clamped = clamp_to_day(start, duration);
            prop_assert!(u32::from(start) + clamped <= u32::from(DAY_MINUTES));
            prop_assert_eq!(clamped == duration, u32::from(start) + duration <= u32::from(DAY_MINUTES));
        }

        /// Snapping any `i32` never overflows: it lands on a non-negative step
        /// within half a step, or on the last step `i32` can hold.
        #[test]
        fn snap_never_overflows(
            value in prop_oneof![Just(i32::MAX), i32::MAX - 100..=i32::MAX, any::<i32>()],
            step in 1..=u16::MAX,
        ) {
            let snapped = snap(value, step);
            let (expected, step) = (i64::from(value.max(0)), i64::from(step));
            let last = i64::from(i32::MAX) / step * step;
            prop_assert!(snapped >= 0 && i64::from(snapped) % step == 0);
            // The last step is an answer only where the nearest one is past it.
            prop_assert!(
                2 * (i64::from(snapped) - expected).abs() <= step
                    || (i64::from(snapped) == last && expected + step / 2 > last)
            );
        }
    }
}

mod rounding {
    use super::*;
    use horae_core::rounding::round;
    use horae_core::types::RoundDir;

    fn direction() -> impl Strategy<Value = RoundDir> {
        prop_oneof![
            Just(RoundDir::Up),
            Just(RoundDir::Down),
            Just(RoundDir::Nearest)
        ]
    }

    // The stored domain: `time_entries.minutes` is a non-negative integer and
    // `organizations.round_minutes` a smallint.
    const MAX_MINUTES: u32 = i32::MAX as u32;
    const MAX_INCREMENT: u32 = i16::MAX as u32;

    /// Stored (minutes, increment) pairs, weighted toward the inputs a slipped
    /// comparison gets wrong: 0, exact multiples and exact Nearest ties
    /// (`k × inc + inc / 2`, even `inc`), which uniform draws almost never hit.
    fn stored_case() -> impl Strategy<Value = (u32, u32)> {
        let multiple = (1..=MAX_INCREMENT).prop_flat_map(|inc| {
            (
                (0..=MAX_MINUTES / inc).prop_map(move |k| k * inc),
                Just(inc),
            )
        });
        let tie = (1..=MAX_INCREMENT / 2).prop_flat_map(|half| {
            let inc = 2 * half;
            (
                (0..=(MAX_MINUTES - half) / inc).prop_map(move |k| k * inc + half),
                Just(inc),
            )
        });
        prop_oneof![
            (0..=MAX_MINUTES, 1..=MAX_INCREMENT),
            (Just(0), 1..=MAX_INCREMENT),
            multiple,
            tie,
        ]
    }

    /// `rounded` is a multiple of `inc` on the side `dir` names, less than one
    /// increment away; nearest takes the closer multiple and a tie goes up.
    fn lands_on_the_named_side(minutes: u32, inc: u32, dir: RoundDir, rounded: u64) -> bool {
        let (minutes, inc) = (u64::from(minutes), u64::from(inc));
        let gap = rounded.abs_diff(minutes);
        rounded.is_multiple_of(inc)
            && gap < inc
            && match dir {
                RoundDir::Up => rounded >= minutes,
                RoundDir::Down => rounded <= minutes,
                RoundDir::Nearest => 2 * gap < inc || (2 * gap == inc && rounded > minutes),
            }
    }

    proptest! {
        /// Stored minutes round to a multiple of the increment on the side the
        /// direction names, within one increment, ties up. Catches billing in
        /// the wrong direction or by a whole extra increment.
        #[test]
        fn round_lands_on_the_named_side_within_one_increment(
            (minutes, inc) in stored_case(),
            dir in direction(),
        ) {
            let rounded = round(minutes, inc, dir);
            prop_assert!(lands_on_the_named_side(minutes, inc, dir, rounded.into()), "{rounded}");
        }

        /// Near `u32::MAX` rounding never wraps to a small value: it returns
        /// the true multiple when that fits, and `u32::MAX` only when the
        /// multiple the direction requires (the next one up) does not.
        #[test]
        fn round_saturates_instead_of_wrapping(
            minutes in prop_oneof![Just(u32::MAX), u32::MAX - 100_000..=u32::MAX, any::<u32>()],
            inc in prop_oneof![1..=100_u32, 1..=u32::MAX],
            dir in direction(),
        ) {
            let rounded = round(minutes, inc, dir);
            let above = (u64::from(minutes / inc) + 1) * u64::from(inc);
            let needs_above = match dir {
                RoundDir::Up => minutes % inc != 0,
                RoundDir::Nearest => 2 * u64::from(minutes % inc) >= u64::from(inc),
                RoundDir::Down => false,
            };
            if needs_above && above > u64::from(u32::MAX) {
                prop_assert_eq!(rounded, u32::MAX);
            } else {
                prop_assert!(lands_on_the_named_side(minutes, inc, dir, rounded.into()), "{rounded}");
            }
        }

        /// Rounding twice equals rounding once, and more worked time never
        /// rounds to fewer billed minutes.
        #[test]
        fn round_is_idempotent_and_monotonic(
            a in 0..=MAX_MINUTES,
            b in 0..=MAX_MINUTES,
            inc in 0..=MAX_INCREMENT,
            dir in direction(),
        ) {
            let (low, high) = (a.min(b), a.max(b));
            prop_assert_eq!(round(round(low, inc, dir), inc, dir), round(low, inc, dir));
            prop_assert!(round(low, inc, dir) <= round(high, inc, dir));
        }
    }
}

mod money {
    use super::*;
    use horae_core::money::{
        Withheld, format_cents, format_cents_plain, parse_cents, sum_by_currency,
    };

    proptest! {
        /// Typed amounts read back exactly from both display forms, grouped or
        /// plain, so editing an amount without touching it never changes it.
        #[test]
        fn displayed_amounts_parse_back(
            cents in prop_oneof![Just(i64::MIN), Just(i64::MAX), any::<i64>(), (i64::MIN / 10..=i64::MAX / 10).prop_map(|t| t * 10)],
        ) {
            let plain = format_cents_plain(cents);
            prop_assert_eq!(parse_cents(&plain).unwrap(), cents);
            // One fractional digit means tenths: "12.5" is 12.50, not 12.05.
            if let Some(tenths) = plain.strip_suffix('0').filter(|p| p.contains('.')) {
                prop_assert_eq!(parse_cents(tenths).unwrap(), cents);
            }
            let shown = format_cents(cents, "EUR");
            prop_assert_eq!(parse_cents(shown.strip_prefix("EUR ").unwrap()).unwrap(), cents);
            // A third decimal is refused rather than rounded, so an API amount
            // never lands a fraction of a cent off.
            prop_assert!(parse_cents(&(plain.clone() + "1")).is_err());
        }

        /// Each currency's total is the exact sum of that currency's amounts
        /// only, whatever the input order, and one withheld amount hides every
        /// total. Catches currency mixing and order-dependent overflow.
        #[test]
        fn currency_totals_are_exact_per_currency_and_order_free(
            (amounts, shuffled) in prop::collection::vec(
                (prop::sample::select(vec!["EUR", "USD", "eur"]), any::<i64>()),
                0..32,
            )
            .prop_flat_map(|amounts| (Just(amounts.clone()), Just(amounts).prop_shuffle())),
            withheld_at in any::<prop::sample::Index>(),
        ) {
            let mut expected = std::collections::BTreeMap::<&str, i128>::new();
            for &(currency, cents) in &amounts {
                *expected.entry(currency).or_default() += i128::from(cents);
            }
            let expected: Vec<_> = expected
                .into_iter()
                .map(|(currency, sum)| (currency, i64::try_from(sum).ok()))
                .collect();
            let visible = |list: &[(&'static str, i64)]| {
                list.iter().map(|&(c, a)| (c, Some(a))).collect::<Vec<_>>()
            };
            prop_assert_eq!(sum_by_currency(visible(&amounts)), Ok(expected.clone()));
            prop_assert_eq!(sum_by_currency(visible(&shuffled)), Ok(expected));

            let mut hidden = visible(&amounts);
            hidden.insert(withheld_at.index(hidden.len() + 1), ("EUR", None));
            prop_assert_eq!(sum_by_currency(hidden), Err(Withheld));
        }

        /// No input string makes the amount parser panic.
        #[test]
        fn parse_cents_never_panics(input in parser_input()) {
            let _ = parse_cents(&input);
        }
    }
}

mod invoice {
    use super::*;
    use horae_core::invoice::{
        InvoiceAmountError, allocate_invoice_discount, invoice_amounts, line_amount_cents,
    };
    use horae_core::project::Percentage;

    fn percentage() -> impl Strategy<Value = Percentage> {
        (0..=10_000_u16).prop_map(|bps| Percentage::try_from(bps).unwrap())
    }

    fn half() -> Percentage {
        Percentage::try_from(5_000).unwrap()
    }

    /// `amount × bps / 10_000` rounded half up lies within half a cent:
    /// `portion × 10_000 − amount × bps ∈ (−5_000, 5_000]`, a tie rounding up.
    fn is_half_up_portion(portion: i64, amount: i64, percentage: Percentage) -> bool {
        let error = i128::from(portion) * 10_000
            - i128::from(amount) * i128::from(percentage.basis_points());
        (-4_999..=5_000).contains(&error)
    }

    proptest! {
        /// The discount and each tax are their exact share rounded half up, the
        /// discount never exceeds the subtotal, and the total is exactly
        /// subtotal − discount + taxes, or `Overflow` precisely when that does
        /// not fit in `i64`.
        #[test]
        fn invoice_components_round_half_up_and_add_up(
            (subtotal, discount, tax1, tax2) in prop_oneof![
                (prop_oneof![Just(i64::MAX), 0..=i64::MAX, 0..=10_000_i64], percentage(), percentage(), percentage()),
                // An odd subtotal at 50% is an exact half-cent tie, which must round up.
                ((0..=i64::MAX / 2).prop_map(|k| 2 * k + 1), Just(half()), Just(half()), Just(half())),
            ],
        ) {
            let half_up = |amount: i64, p: Percentage| {
                (i128::from(amount) * i128::from(p.basis_points()) + 5_000) / 10_000
            };
            let discount_cents = half_up(subtotal, discount);
            let taxable = i128::from(subtotal) - discount_cents;
            let taxable_i64 = i64::try_from(taxable).unwrap();
            let total = taxable + half_up(taxable_i64, tax1) + half_up(taxable_i64, tax2);
            match invoice_amounts(subtotal, discount, tax1, tax2) {
                Ok(a) => {
                    prop_assert!(is_half_up_portion(a.discount_cents, subtotal, discount));
                    prop_assert!((0..=subtotal).contains(&a.discount_cents));
                    let taxable = subtotal - a.discount_cents;
                    prop_assert!(is_half_up_portion(a.tax1_cents, taxable, tax1));
                    prop_assert!(is_half_up_portion(a.tax2_cents, taxable, tax2));
                    prop_assert_eq!(a.total_cents, taxable + a.tax1_cents + a.tax2_cents);
                }
                Err(err) => {
                    prop_assert_eq!(err, InvoiceAmountError::Overflow);
                    prop_assert!(total > i128::from(i64::MAX));
                }
            }
        }

        /// Discounted line amounts add up to the invoice's discounted
        /// subtotal, stay within 0..=gross, and each line's discount is within
        /// one cent of its exact proportional share; the leftover cents go to
        /// the largest remainders, earlier lines first on a tie. Catches a lost
        /// or duplicated cent between the invoice header and its lines.
        #[test]
        fn discount_allocation_conserves_the_header_discount(
            gross in prop_oneof![
                prop::collection::vec(0..=i64::MAX / 64, 0..64),
                // Small, often equal lines tie on remainders, so the tie order counts.
                prop::collection::vec(0..=20_i64, 0..16),
            ],
            discount in percentage(),
        ) {
            let net = allocate_invoice_discount(&gross, discount).unwrap();
            let subtotal: i64 = gross.iter().sum();
            let header = invoice_amounts(subtotal, discount, Percentage::ZERO, Percentage::ZERO).unwrap();
            prop_assert_eq!(net.len(), gross.len());
            prop_assert_eq!(net.iter().sum::<i64>(), header.total_cents);
            // (remainder, gets a leftover cent) per line, in source order.
            let mut lines = Vec::new();
            for (&net, &gross) in net.iter().zip(&gross) {
                prop_assert!((0..=gross).contains(&net));
                let share = i128::from(gross - net);
                if subtotal == 0 {
                    prop_assert_eq!(share, 0);
                    continue;
                }
                let exact = i128::from(gross) * i128::from(header.discount_cents);
                prop_assert!((share * i128::from(subtotal) - exact).abs() < i128::from(subtotal));
                let floor = exact / i128::from(subtotal);
                lines.push((exact % i128::from(subtotal), share > floor));
            }
            for (i, &(won, extra)) in lines.iter().enumerate() {
                for (j, &(lost, missed)) in lines.iter().enumerate() {
                    if extra && !missed {
                        prop_assert!(won > lost || (won == lost && i < j), "line {i} over {j}");
                    }
                }
            }
        }

        /// For stored (non-negative) rates and minutes, a line bills
        /// rate × minutes / 60 rounded half up, errs exactly when that exceeds
        /// `i64`, and more minutes never bill less.
        #[test]
        fn line_amount_is_half_up_and_monotonic(
            rate in prop_oneof![Just(i64::MAX), 0..=i64::MAX, 0..=1_000_000_i64],
            a in 0..=i32::MAX,
            b in 0..=i32::MAX,
        ) {
            let (low, high) = (a.min(b), a.max(b));
            for minutes in [low, high] {
                let exact = i128::from(rate) * i128::from(minutes);
                match line_amount_cents(rate, minutes) {
                    Ok(cents) => prop_assert!((-29..=30).contains(&(i128::from(cents) * 60 - exact))),
                    Err(_) => prop_assert!(exact >= 60 * i128::from(i64::MAX) + 30),
                }
            }
            if let (Ok(low), Ok(high)) = (line_amount_cents(rate, low), line_amount_cents(rate, high)) {
                prop_assert!(low <= high);
            }
        }
    }
}

mod budget {
    use super::*;
    use horae_core::budget::{OVER_BUDGET_BAND, current_band, newly_crossed_bands, used_percent};

    proptest! {
        /// The progress percentage of a positive budget is always 0–100, is
        /// 100 once the budget is spent, and never falls as consumption rises.
        #[test]
        fn used_percent_is_bounded_and_monotonic(
            a in any::<i64>(),
            b in any::<i64>(),
            budget in prop_oneof![Just(1_i64), Just(i64::MAX), 1..=i64::MAX],
        ) {
            let (low, high) = (a.min(b), a.max(b));
            let (low_pct, high_pct) = (used_percent(low, budget).unwrap(), used_percent(high, budget).unwrap());
            prop_assert!(high_pct <= 100);
            prop_assert!(low_pct <= high_pct);
            if high >= budget {
                prop_assert_eq!(high_pct, 100);
            }
        }

        /// As consumption rises, storing `current_band` after each write
        /// announces every effective band (the configured warnings plus 100)
        /// exactly once, in order, up to the final band. Catches duplicate or
        /// skipped budget notifications.
        #[test]
        fn rising_consumption_announces_each_band_once(
            thresholds in prop::collection::vec(-10..=250_i32, 0..6),
            budget in 1..=100_000_i64,
            writes in prop::collection::vec(0..=40_000_i64, 1..24),
        ) {
            let (mut consumed, mut last, mut announced) = (0, 0, Vec::new());
            for added in writes {
                consumed += added;
                announced.extend(newly_crossed_bands(consumed, budget, &thresholds, last));
                last = current_band(consumed, budget, &thresholds);
            }
            let mut expected: Vec<i32> = thresholds
                .iter()
                .copied()
                .chain([OVER_BUDGET_BAND])
                .filter(|&band| 0 < band && band <= last)
                .collect();
            expected.sort_unstable();
            expected.dedup();
            prop_assert_eq!(&announced, &expected);
            prop_assert_eq!(consumed >= budget, announced.contains(&OVER_BUDGET_BAND));
        }
    }
}

mod week {
    use super::*;
    use horae_core::week::{iso_week_monday, week_bounds, week_start};

    fn date() -> impl Strategy<Value = NaiveDate> {
        let days = NaiveDate::MIN.num_days_from_ce()..=NaiveDate::MAX.num_days_from_ce();
        prop_oneof![
            Just(NaiveDate::MIN),
            Just(NaiveDate::MAX),
            days.prop_map(|day| NaiveDate::from_num_days_from_ce_opt(day).unwrap()),
        ]
    }

    proptest! {
        /// For every first-day setting the configured week starts on that
        /// weekday, contains the date and spans seven days; a week the
        /// calendar cannot represent is `None`, never a panic. The ISO Monday
        /// helper agrees with the Monday setting.
        #[test]
        fn weeks_start_on_the_setting_and_contain_the_date(date in date()) {
            prop_assert_eq!(iso_week_monday(date), week_start(date, Weekday::Mon));
            for first_day in WEEKDAYS {
                // By search: the latest `first_day` on or before the date, and
                // six days on, wherever both are representable.
                let days = |n| chrono::Days::new(n);
                let expected = (0..7)
                    .find_map(|k| date.checked_sub_days(days(k)).filter(|d| d.weekday() == first_day))
                    .and_then(|start| Some((start, start.checked_add_days(days(6))?)));
                prop_assert_eq!(week_bounds(date, first_day), expected);
                if let Some((start, end)) = expected {
                    prop_assert!(start <= date && date <= end);
                }
            }
        }
    }
}

mod project_activity {
    use super::*;
    use horae_core::project_activity::{ActivityRange, DailyActivity, weekly_activity};

    proptest! {
        /// Weekly buckets tile the range exactly (contiguous, on the first-day
        /// setting, clipped at both ends), each holds exactly the minutes of
        /// the days inside it, and the cumulative series is their running
        /// total, whatever the input order. Catches a day landing in the wrong
        /// week or being dropped at a range edge.
        #[test]
        fn weekly_buckets_tile_the_range_and_keep_every_minute(
            from in 0..36_500_u64,
            length in 0..120_u64,
            first_day in prop::sample::select(WEEKDAYS.to_vec()),
            (days, shuffled) in prop::collection::vec((0..140_u64, 0..=10_000_i64, 0..=10_000_i64), 0..40)
                .prop_flat_map(|days| (Just(days.clone()), Just(days).prop_shuffle())),
        ) {
            let base = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
            let from = base + Days::new(from);
            let range = ActivityRange::new(from, from + Days::new(length)).unwrap();
            let to_days = |list: &[(u64, i64, i64)]| -> Vec<DailyActivity> {
                list.iter()
                    .map(|&(offset, billable, non_billable)| DailyActivity {
                        // Some days fall either side of the range and must be ignored.
                        date: from + Days::new(offset) - Days::new(10),
                        billable_minutes: billable,
                        non_billable_minutes: non_billable,
                    })
                    .collect()
            };
            let days = to_days(&days);
            let weeks = weekly_activity(range, first_day, &days, 100).unwrap();
            prop_assert_eq!(&weekly_activity(range, first_day, &to_days(&shuffled), 100).unwrap(), &weeks);

            prop_assert_eq!(weeks.first().unwrap().from, range.from());
            prop_assert_eq!(weeks.last().unwrap().to, range.to());
            let mut cumulative = 0;
            for (index, week) in weeks.iter().enumerate() {
                prop_assert!(week.from <= week.to && (week.to - week.from).num_days() <= 6);
                if index > 0 {
                    prop_assert_eq!(week.from.weekday(), first_day);
                    prop_assert_eq!(week.from, weeks[index - 1].to.succ_opt().unwrap());
                }
                let inside = days.iter().filter(|d| (week.from..=week.to).contains(&d.date));
                let (billable, non_billable) = inside.fold((0, 0), |(b, n), d| {
                    (b + d.billable_minutes, n + d.non_billable_minutes)
                });
                prop_assert_eq!((week.billable_minutes, week.non_billable_minutes), (billable, non_billable));
                cumulative += billable + non_billable;
                prop_assert_eq!(week.cumulative_minutes, cumulative);
            }
        }
    }
}

mod state {
    use horae_core::state::can_transition;
    use horae_core::types::{EntryState, OrgRole};

    const STATES: [EntryState; 4] = [
        EntryState::Open,
        EntryState::Submitted,
        EntryState::Approved,
        EntryState::Invoiced,
    ];
    const ROLES: [OrgRole; 3] = [OrgRole::Admin, OrgRole::Manager, OrgRole::Member];

    /// Over every (from, to, role) triple: invoiced time is final, nothing
    /// transitions to itself, a member only moves time between open and
    /// submitted or invoices it (the invoice write grant, not the role, is
    /// the caller's check), approved time leaves only by a manager's reopen
    /// or an invoice, and submitted time cannot be invoiced without an
    /// approval decision.
    #[test]
    fn entry_state_machine_upholds_its_locks() {
        use EntryState::*;
        for from in STATES {
            for to in STATES {
                for role in ROLES {
                    if !can_transition(from, to, role) {
                        continue;
                    }
                    let step = format!("{from:?} -> {to:?} as {role:?}");
                    assert_ne!(from, Invoiced, "{step}");
                    assert_ne!(from, to, "{step}");
                    if role == OrgRole::Member {
                        assert!(
                            matches!(
                                (from, to),
                                (Open, Submitted) | (Submitted, Open) | (_, Invoiced)
                            ),
                            "{step}"
                        );
                    }
                    if from == Approved {
                        assert!(
                            to == Invoiced || (role.is_manager_or_above() && to == Open),
                            "{step}"
                        );
                    }
                    assert_ne!((from, to), (Submitted, Invoiced), "{step}");
                    // Managers and admins hold the same entry-state authority.
                    assert!(can_transition(from, to, OrgRole::Admin), "{step}");
                    assert!(can_transition(from, to, OrgRole::Manager), "{step}");
                }
            }
        }
    }

    /// The workflow is usable: from open, a member can submit and a manager
    /// can approve and invoice, and every non-final state can return to open.
    #[test]
    fn entry_state_machine_reaches_every_state() {
        use EntryState::*;
        assert!(can_transition(Open, Submitted, OrgRole::Member));
        assert!(can_transition(Submitted, Approved, OrgRole::Manager));
        assert!(can_transition(Approved, Invoiced, OrgRole::Manager));
        assert!(can_transition(Submitted, Open, OrgRole::Member));
        assert!(can_transition(Approved, Open, OrgRole::Manager));
    }
}
