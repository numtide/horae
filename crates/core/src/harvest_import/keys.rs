//! Natural-key normalization and composite key builders (FR-012).
//!
//! When no Harvest provenance mapping exists yet (a first import) or the source
//! carries no Harvest ids at all (the CSV adapter), records are matched against
//! existing Horae rows by a **composite natural key**. All string components are
//! normalized first — trimmed of surrounding whitespace and case-folded — so
//! incidental casing or spacing differences do not look like a different record.
//!
//! These builders produce a single canonical string per entity so callers can
//! compare or index on it directly. Distinct entities never collide because each
//! component is length-prefixed via a delimiter that cannot appear in a
//! normalized value boundary the same way (see [`compose`]).

use chrono::NaiveDate;

/// Trim surrounding whitespace and case-fold a natural-key component so that
/// `"  Acme Corp "` and `"acme corp"` compare equal.
pub fn normalize(value: &str) -> String {
    value.trim().to_lowercase()
}

/// Join normalized components into one canonical key string. Each component is
/// separated by a unit-separator byte (`\u{1f}`), which normalization never
/// introduces, so `("a", "bc")` and `("ab", "c")` cannot collide.
fn compose(parts: &[&str]) -> String {
    parts
        .iter()
        .map(|p| normalize(p))
        .collect::<Vec<_>>()
        .join("\u{1f}")
}

/// Client natural key: normalized name (data-model.md).
pub fn client_key(name: &str) -> String {
    normalize(name)
}

/// Project natural key: the project code when present, otherwise the pairing of
/// its client key and normalized project name (data-model.md).
pub fn project_key(client_name: &str, project_name: &str, project_code: Option<&str>) -> String {
    match project_code.map(str::trim).filter(|c| !c.is_empty()) {
        Some(code) => format!("code\u{1f}{}", normalize(code)),
        None => compose(&["name", client_name, project_name]),
    }
}

/// Task natural key: normalized name within the org-level catalog (data-model.md).
pub fn task_key(name: &str) -> String {
    normalize(name)
}

/// Time-entry natural key: the combination of user, project, task, spent date,
/// duration in minutes, and notes — so two genuinely distinct entries on the
/// same day are both kept while an exact re-import of one is recognized as the
/// same record (FR-012).
pub fn time_entry_key(
    user_email: &str,
    project_key: &str,
    task_key: &str,
    spent_date: NaiveDate,
    minutes: i64,
    notes: Option<&str>,
) -> String {
    compose(&[
        user_email,
        project_key,
        task_key,
        &spent_date.to_string(),
        &minutes.to_string(),
        notes.unwrap_or(""),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_trims_and_case_folds() {
        assert_eq!(normalize("  Acme Corp "), "acme corp");
        assert_eq!(normalize("ACME"), "acme");
        assert_eq!(normalize("acme"), "acme");
    }

    #[test]
    fn client_key_equal_across_case_and_whitespace() {
        assert_eq!(client_key("  Acme Corp "), client_key("acme corp"));
        assert_ne!(client_key("Acme"), client_key("Beta"));
    }

    #[test]
    fn project_key_prefers_code_when_present() {
        // With a code, the client/name pairing is irrelevant.
        assert_eq!(
            project_key("Acme", "Website", Some("WEB-1")),
            project_key("Other", "Different", Some(" web-1 "))
        );
    }

    #[test]
    fn project_key_falls_back_to_client_and_name() {
        assert_eq!(
            project_key("Acme", "Website", None),
            project_key("acme", "website", Some("  ")) // blank code → fallback
        );
        // Same project name under different clients is distinct.
        assert_ne!(
            project_key("Acme", "Website", None),
            project_key("Beta", "Website", None)
        );
    }

    #[test]
    fn task_key_shared_across_projects() {
        assert_eq!(task_key("Design"), task_key(" design "));
    }

    #[test]
    fn time_entry_key_distinguishes_and_matches() {
        let date = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let a = time_entry_key("u@x.com", "proj", "task", date, 90, Some("standup"));
        let b = time_entry_key("U@X.com", "proj", "task", date, 90, Some("standup"));
        // Case-folded user email matches.
        assert_eq!(a, b);
        // A different duration is a distinct entry.
        let c = time_entry_key("u@x.com", "proj", "task", date, 120, Some("standup"));
        assert_ne!(a, c);
        // A different note is a distinct entry.
        let d = time_entry_key("u@x.com", "proj", "task", date, 90, Some("review"));
        assert_ne!(a, d);
    }

    #[test]
    fn composite_components_do_not_collide() {
        // ("a","bc") vs ("ab","c") must differ thanks to the separator.
        assert_ne!(compose(&["a", "bc"]), compose(&["ab", "c"]));
    }
}
