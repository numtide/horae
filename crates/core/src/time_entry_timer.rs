//! When a stopped time entry may run again.

use chrono::NaiveDate;
use thiserror::Error;

use crate::types::EntryState;

/// Why an entry cannot be restarted. The messages are what the person sees.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum RestartRefusal {
    #[error("The timer is already running")]
    Running,
    #[error("Only open entries can be restarted")]
    NotOpen,
    #[error("Only today's entries can be restarted")]
    NotToday,
}

/// An entry may be restarted when it is stopped, open, and dated `today` in
/// its person's time zone; restarting it keeps its minutes and the next stop
/// adds the new elapsed time.
pub fn check_restart(
    is_running: bool,
    state: EntryState,
    spent_date: NaiveDate,
    today: NaiveDate,
) -> Result<(), RestartRefusal> {
    if is_running {
        Err(RestartRefusal::Running)
    } else if state != EntryState::Open {
        Err(RestartRefusal::NotOpen)
    } else if spent_date != today {
        Err(RestartRefusal::NotToday)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 18).unwrap()
    }

    #[test]
    fn a_running_entry_cannot_be_restarted() {
        assert_eq!(
            check_restart(true, EntryState::Open, today(), today()),
            Err(RestartRefusal::Running)
        );
    }

    #[test]
    fn a_locked_entry_cannot_be_restarted() {
        for state in [
            EntryState::Submitted,
            EntryState::Approved,
            EntryState::Invoiced,
        ] {
            assert_eq!(
                check_restart(false, state, today(), today()),
                Err(RestartRefusal::NotOpen)
            );
        }
    }

    #[test]
    fn an_entry_of_another_day_cannot_be_restarted() {
        let yesterday = today().pred_opt().unwrap();
        assert_eq!(
            check_restart(false, EntryState::Open, yesterday, today()),
            Err(RestartRefusal::NotToday)
        );
    }

    #[test]
    fn a_stopped_open_entry_of_today_can_be_restarted() {
        assert_eq!(
            check_restart(false, EntryState::Open, today(), today()),
            Ok(())
        );
    }
}
