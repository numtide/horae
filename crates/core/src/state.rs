use crate::types::{EntryState, OrgRole};

/// Returns true if the transition from `from` to `to` is allowed for `role`.
pub fn can_transition(from: EntryState, to: EntryState, role: OrgRole) -> bool {
    use EntryState::*;
    use OrgRole::*;
    match (from, to) {
        // Any member can submit open entries
        (Open, Submitted) => true,
        // Members can reopen (pull back) their own submission
        (Submitted, Open) => true,
        // Managers and admins can approve
        (Submitted, Approved) => matches!(role, Manager | Admin),
        // Managers and admins can reject (reopen from submitted)
        (Approved, Open) => matches!(role, Manager | Admin),
        // Managers and admins generate invoices, which flips the covered
        // entries to invoiced. Open time is directly invoiceable (spec 001:
        // no separate approval workflow in v1); approved time is the
        // canonical pre-invoice state. Submitted time stays locked pending
        // an approval decision.
        (Open, Invoiced) | (Approved, Invoiced) => matches!(role, Manager | Admin),
        // No other transitions allowed
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use EntryState::*;
    use OrgRole::*;

    #[test]
    fn member_submit_and_reopen() {
        assert!(can_transition(Open, Submitted, Member));
        assert!(can_transition(Submitted, Open, Member));
    }

    #[test]
    fn only_manager_can_approve() {
        assert!(!can_transition(Submitted, Approved, Member));
        assert!(can_transition(Submitted, Approved, Manager));
        assert!(can_transition(Submitted, Approved, Admin));
    }

    #[test]
    fn managers_invoice_open_and_approved_time() {
        for from in [Open, Approved] {
            assert!(!can_transition(from, Invoiced, Member));
            assert!(can_transition(from, Invoiced, Manager));
            assert!(can_transition(from, Invoiced, Admin));
        }
        assert!(!can_transition(Submitted, Invoiced, Admin));
    }

    #[test]
    fn invoiced_is_final() {
        assert!(!can_transition(Invoiced, Open, Admin));
        assert!(!can_transition(Invoiced, Submitted, Admin));
    }
}
