use crate::types::RoundDir;

/// Minutes used for billing: a frozen value wins over today's rounding rule.
/// Raw worked minutes remain separate, for elapsed-time and cost reporting.
pub fn effective_minutes(minutes: u32, frozen: Option<u32>, inc: u32, dir: RoundDir) -> u32 {
    frozen.unwrap_or_else(|| round(minutes, inc, dir))
}

/// Round `minutes` to the nearest multiple of `inc`.
///
/// - `inc == 0` → identity (no rounding)
/// - `RoundDir::Up` → ceiling to next multiple
/// - `RoundDir::Down` → floor to previous multiple
/// - `RoundDir::Nearest` → nearest multiple (ties round up)
///
/// A multiple above `u32::MAX` saturates to `u32::MAX`. Stored minutes are
/// `i32` and increments `i16`, so stored values never reach it.
pub fn round(minutes: u32, inc: u32, dir: RoundDir) -> u32 {
    if inc == 0 {
        return minutes;
    }
    // Widened so the multiple above `minutes` cannot wrap.
    let (minutes, inc) = (u64::from(minutes), u64::from(inc));
    let lower = minutes - minutes % inc;
    let rounded = match dir {
        RoundDir::Up if lower == minutes => minutes,
        RoundDir::Up => lower + inc,
        RoundDir::Down => lower,
        RoundDir::Nearest if 2 * (minutes - lower) < inc => lower,
        RoundDir::Nearest => lower + inc,
    };
    u32::try_from(rounded).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effective_minutes_respect_frozen_history_including_zero() {
        for dir in [RoundDir::Up, RoundDir::Down, RoundDir::Nearest] {
            assert_eq!(effective_minutes(8, Some(10), 15, dir), 10);
            assert_eq!(effective_minutes(8, Some(0), 15, dir), 0);
            assert_eq!(effective_minutes(8, None, 15, dir), round(8, 15, dir));
        }
    }

    #[test]
    fn identity_when_inc_zero() {
        for dir in [RoundDir::Up, RoundDir::Down, RoundDir::Nearest] {
            assert_eq!(round(73, 0, dir), 73);
        }
    }

    #[test]
    fn result_is_multiple_of_inc() {
        for m in 0..=120u32 {
            let r = round(m, 15, RoundDir::Nearest);
            assert_eq!(r % 15, 0, "round({m}, 15, Nearest) = {r}");
        }
    }

    #[test]
    fn nearest_ties_round_up() {
        // 7.5 min → ties between 0 and 15 → rounds up to 15
        assert_eq!(round(8, 15, RoundDir::Nearest), 15);
        assert_eq!(round(7, 15, RoundDir::Nearest), 0);
    }

    #[test]
    fn up_and_down() {
        assert_eq!(round(61, 15, RoundDir::Up), 75);
        assert_eq!(round(61, 15, RoundDir::Down), 60);
        assert_eq!(round(60, 15, RoundDir::Up), 60); // already multiple
    }
}
