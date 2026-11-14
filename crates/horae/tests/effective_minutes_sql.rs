#![cfg(feature = "server")]
//! The SQL `effective_minutes` and
//! `horae_core::rounding::effective_minutes` must agree on every stored input:
//! reports, budgets and approvals bill through the SQL twin, the Harvest API
//! through the Rust one, so a disagreement bills the same time differently.

use horae_core::rounding::effective_minutes;
use horae_core::types::RoundDir;
use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::TestRunner;
use serial_test::serial;
use sqlx::PgPool;

/// Harvest's limit on a person's day ("Daily hours cannot exceed 240"), so no
/// stored entry holds more.
const DAY_CAP: i32 = 14_400;

type Case = (i32, Option<i32>, i16, RoundDir);

/// Stored minutes: non-negative and at most one capped day.
fn minutes() -> impl Strategy<Value = i32> {
    prop_oneof![
        Just(0),
        Just(1),
        Just(DAY_CAP),
        DAY_CAP - 100..=DAY_CAP,
        0..=1_440,
        0..=DAY_CAP,
    ]
}

/// `organizations.round_minutes` is a non-negative smallint.
fn increment() -> impl Strategy<Value = i16> {
    prop_oneof![
        Just(0),
        Just(1),
        Just(2),
        Just(15),
        Just(i16::MAX),
        0..=60_i16,
        0..=i16::MAX,
    ]
}

fn direction() -> impl Strategy<Value = RoundDir> {
    prop_oneof![
        Just(RoundDir::Up),
        Just(RoundDir::Down),
        Just(RoundDir::Nearest)
    ]
}

/// Ordinary stored tuples, plus exact Nearest ties (`k × inc + inc / 2`, even
/// `inc`) that stay within the day cap.
fn stored_case() -> impl Strategy<Value = Case> {
    let tie = (1..=DAY_CAP / 2)
        .prop_flat_map(|half| (0..=(DAY_CAP - half) / (2 * half), Just(half)))
        .prop_map(|(k, half)| {
            let inc = 2 * half;
            (k * inc + half, None, inc as i16, RoundDir::Nearest)
        });
    prop_oneof![
        3 => (minutes(), prop::option::of(minutes()), increment(), direction()),
        1 => tie,
    ]
}

/// Any non-negative `i32`, weighted toward the top, where rounding up leaves
/// the `integer` range.
fn unbounded_case() -> impl Strategy<Value = Case> {
    let minutes = prop_oneof![Just(i32::MAX), i32::MAX - 100_000..=i32::MAX, 0..=i32::MAX];
    (minutes, Just(None), increment(), direction())
}

async fn sql_effective_minutes(
    pool: &PgPool,
    (minutes, frozen, inc, dir): Case,
) -> Result<i32, String> {
    sqlx::query_scalar!(
        r#"SELECT effective_minutes($1, $2, $3, $4) AS "minutes!""#,
        minutes,
        frozen,
        inc,
        dir as RoundDir,
    )
    .fetch_one(pool)
    .await
    .map_err(|err| err.to_string())
}

fn rust_effective_minutes((minutes, frozen, inc, dir): Case) -> u32 {
    effective_minutes(
        minutes as u32,
        frozen.map(|value| value as u32),
        inc as u32,
        dir,
    )
}

/// Runs `cases` deterministic draws and collects each case where `agrees`
/// rejects the SQL and Rust results; there is no shrinking, so each mismatch
/// is printed in full.
async fn mismatches(
    pool: &PgPool,
    strategy: impl Strategy<Value = Case>,
    cases: usize,
    agrees: impl Fn(&Result<i32, String>, u32) -> bool,
) -> Vec<String> {
    let mut runner = TestRunner::deterministic();
    let mut found = Vec::new();
    for _ in 0..cases {
        let case = strategy.new_tree(&mut runner).unwrap().current();
        let sql = sql_effective_minutes(pool, case).await;
        let rust = rust_effective_minutes(case);
        if !agrees(&sql, rust) {
            found.push(format!("{case:?}: sql={sql:?} rust={rust}"));
        }
    }
    found
}

#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn sql_effective_minutes_matches_the_domain_rounding(pool: PgPool) {
    let found = mismatches(&pool, stored_case(), 4_096, |sql, rust| {
        i32::try_from(rust).is_ok_and(|rust| *sql == Ok(rust))
    })
    .await;
    assert!(
        found.is_empty(),
        "{} mismatches, first: {:#?}",
        found.len(),
        &found[..found.len().min(10)]
    );
}

/// Above the day cap the twins part where the result leaves `integer`: SQL
/// raises "integer out of range" exactly where Rust's `u32` exceeds
/// `i32::MAX`, and they agree everywhere else.
#[sqlx::test(migrations = "./migrations")]
#[serial]
async fn sql_effective_minutes_errors_where_the_result_leaves_integer(pool: PgPool) {
    // The smallest such input, so the property below is never vacuous.
    let edge = (i32::MAX, None, 2, RoundDir::Up);
    assert_eq!(rust_effective_minutes(edge), 1 << 31);
    assert!(sql_effective_minutes(&pool, edge).await.is_err());

    let found = mismatches(
        &pool,
        unbounded_case(),
        2_048,
        |sql, rust| match i32::try_from(rust) {
            Ok(rust) => *sql == Ok(rust),
            Err(_) => sql
                .as_ref()
                .is_err_and(|err| err.contains("integer out of range")),
        },
    )
    .await;
    assert!(
        found.is_empty(),
        "{} mismatches, first: {:#?}",
        found.len(),
        &found[..found.len().min(10)]
    );
}
