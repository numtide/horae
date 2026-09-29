//! Exact reciprocal task/person totals in one caller-selected cost currency.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

/// Actual tracked minutes and optional internal costs, without billing rounding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkTotals {
    pub minutes: i64,
    pub billable_minutes: i64,
    /// Absent for incomplete or unauthorized costs, never a partial sum.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_cents: Option<i64>,
}

impl WorkTotals {
    /// No work has zero cost only when costs are visible to the caller.
    pub fn empty(costs_visible: bool) -> Self {
        Self {
            minutes: 0,
            billable_minutes: 0,
            cost_cents: costs_visible.then_some(0),
        }
    }

    fn add(&mut self, other: Self) -> Result<(), BreakdownError> {
        self.minutes = self
            .minutes
            .checked_add(other.minutes)
            .ok_or(BreakdownError::Overflow)?;
        self.billable_minutes = self
            .billable_minutes
            .checked_add(other.billable_minutes)
            .ok_or(BreakdownError::Overflow)?;
        self.cost_cents = self
            .cost_cents
            .zip(other.cost_cents)
            .map(|(left, right)| left.checked_add(right).ok_or(BreakdownError::Overflow))
            .transpose()?;
        Ok(())
    }
}

/// One pre-aggregated task/person intersection; costs are rounded per entry first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkCell {
    pub task_id: Uuid,
    pub user_id: Uuid,
    pub totals: WorkTotals,
}

/// Parent totals for both directions of the same task/person intersections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkSummary {
    pub total: WorkTotals,
    pub by_task: BTreeMap<Uuid, WorkTotals>,
    pub by_person: BTreeMap<Uuid, WorkTotals>,
}

/// Invalid input or a sum that cannot be represented exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum BreakdownError {
    #[error("Work totals exceed the supported integer range")]
    Overflow,
    #[error("Work totals contain invalid minutes or negative costs")]
    InvalidQuantity,
}

/// Fold the same intersections in both directions without rounding again.
pub fn summarize(cells: &[WorkCell], costs_visible: bool) -> Result<WorkSummary, BreakdownError> {
    let mut summary = WorkSummary {
        total: WorkTotals::empty(costs_visible),
        by_task: BTreeMap::new(),
        by_person: BTreeMap::new(),
    };
    for cell in cells {
        let row = cell.totals;
        if row.minutes < 0
            || row.billable_minutes < 0
            || row.billable_minutes > row.minutes
            || row.cost_cents.is_some_and(|cost| cost < 0)
        {
            return Err(BreakdownError::InvalidQuantity);
        }
        summary.total.add(row)?;
        summary
            .by_task
            .entry(cell.task_id)
            .or_insert_with(|| WorkTotals::empty(costs_visible))
            .add(row)?;
        summary
            .by_person
            .entry(cell.user_id)
            .or_insert_with(|| WorkTotals::empty(costs_visible))
            .add(row)?;
    }
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(task: u128, person: u128, minutes: i64, billable: i64, cost: Option<i64>) -> WorkCell {
        WorkCell {
            task_id: Uuid::from_u128(task),
            user_id: Uuid::from_u128(person),
            totals: WorkTotals {
                minutes,
                billable_minutes: billable,
                cost_cents: cost,
            },
        }
    }

    #[test]
    fn reciprocal_groups_reconcile_exact_minutes_and_per_entry_costs() {
        let cells = [
            cell(1, 3, 7, 7, Some(117)),
            cell(1, 4, 9, 0, Some(150)),
            cell(2, 3, 11, 11, Some(183)),
        ];
        let summary = summarize(&cells, true).unwrap();
        assert_eq!(
            summary.total,
            WorkTotals {
                minutes: 27,
                billable_minutes: 18,
                cost_cents: Some(450)
            }
        );
        for groups in [&summary.by_task, &summary.by_person] {
            assert_eq!(
                groups.values().map(|row| row.minutes).sum::<i64>(),
                summary.total.minutes
            );
            assert_eq!(
                groups
                    .values()
                    .map(|row| row.cost_cents.unwrap())
                    .sum::<i64>(),
                450
            );
        }
        assert_eq!(summary.by_task[&Uuid::from_u128(1)].minutes, 16);
        assert_eq!(summary.by_person[&Uuid::from_u128(3)].cost_cents, Some(300));
    }

    #[test]
    fn missing_costs_only_invalidate_contributing_groups_and_hidden_costs_stay_absent() {
        let cells = [cell(1, 3, 7, 7, Some(0)), cell(2, 4, 1, 0, None)];
        let summary = summarize(&cells, true).unwrap();
        assert_eq!(summary.total.cost_cents, None);
        assert_eq!(summary.by_task[&Uuid::from_u128(1)].cost_cents, Some(0));
        assert_eq!(summary.by_person[&Uuid::from_u128(4)].cost_cents, None);
        let hidden = summarize(&cells, false).unwrap();
        assert!(
            hidden
                .by_task
                .values()
                .chain(hidden.by_person.values())
                .all(|row| row.cost_cents.is_none())
        );
        assert_eq!(summarize(&[], true).unwrap().total, WorkTotals::empty(true));
        assert_eq!(summarize(&[], false).unwrap().total.cost_cents, None);
    }

    #[test]
    fn invalid_quantities_and_overflow_never_return_partial_totals() {
        for row in [
            cell(1, 1, -1, 0, None),
            cell(1, 1, 1, 2, None),
            cell(1, 1, 1, -1, None),
            cell(1, 1, 1, 1, Some(-1)),
        ] {
            assert_eq!(
                summarize(&[row], true),
                Err(BreakdownError::InvalidQuantity)
            );
        }
        assert_eq!(
            summarize(
                &[cell(1, 1, i64::MAX, 0, None), cell(2, 2, 1, 0, None)],
                true
            ),
            Err(BreakdownError::Overflow)
        );
        assert_eq!(
            summarize(
                &[cell(1, 1, 1, 0, Some(i64::MAX)), cell(2, 2, 1, 0, Some(1))],
                true
            ),
            Err(BreakdownError::Overflow)
        );
    }
}
