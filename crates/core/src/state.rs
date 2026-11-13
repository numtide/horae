use crate::types::EntryState;

/// Returns true if time may move from `from` to `to`.
///
/// This is the state machine alone. Who may make the move is the caller's
/// check: submitting needs own time-write access, approving, rejecting and
/// reopening need approval access over the person, and invoicing needs invoice
/// access.
pub fn can_transition(from: EntryState, to: EntryState) -> bool {
    use EntryState::*;
    matches!(
        (from, to),
        // Submit open time; pull back or reject a submission.
        (Open, Submitted) | (Submitted, Open)
            // Approve a submission, or reopen an approved week.
            | (Submitted, Approved) | (Approved, Open)
            // Invoicing flips the covered entries to invoiced. Open time is
            // directly invoiceable (spec 001: no separate approval workflow in
            // v1); approved time is the canonical pre-invoice state. Submitted
            // time stays locked pending an approval decision.
            | (Open, Invoiced) | (Approved, Invoiced)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use EntryState::*;

    #[test]
    fn submit_and_pull_back() {
        assert!(can_transition(Open, Submitted));
        assert!(can_transition(Submitted, Open));
    }

    #[test]
    fn approve_and_reopen() {
        assert!(can_transition(Submitted, Approved));
        assert!(can_transition(Approved, Open));
        assert!(!can_transition(Open, Approved));
    }

    #[test]
    fn open_and_approved_time_is_invoiceable() {
        for from in [Open, Approved] {
            assert!(can_transition(from, Invoiced));
        }
        assert!(!can_transition(Submitted, Invoiced));
    }

    #[test]
    fn invoiced_is_final() {
        assert!(!can_transition(Invoiced, Open));
        assert!(!can_transition(Invoiced, Submitted));
    }
}
