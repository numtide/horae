use super::*;

mod api_reads;
mod budgets;
mod managers;
mod people;

#[test]
fn lock_order_bounds_sorts_and_drops_repeats() {
    let (low, high) = (Uuid::from_u128(1), Uuid::from_u128(2));
    assert_eq!(lock_order(&[high, low, high]).unwrap(), [low, high]);
    assert_eq!(lock_order(&[low; 100]).unwrap(), [low]);
    for ids in [vec![], vec![low; 101]] {
        assert_eq!(
            lock_order(&ids),
            Err(OpError::invalid(
                "project_ids",
                "Select between 1 and 100 projects"
            ))
        );
    }
}
