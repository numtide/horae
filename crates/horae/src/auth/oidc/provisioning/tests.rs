use super::*;

mod link;
mod linking;
mod provisioning;

fn active(id: Uuid) -> Candidate {
    Candidate { id, active: true }
}
fn inactive(id: Uuid) -> Candidate {
    Candidate { id, active: false }
}
fn policy(domains: &[&str], trust_unverified_email: bool) -> AutoProvision {
    AutoProvision {
        domains: domains.iter().map(|d| d.to_string()).collect(),
        trust_unverified_email,
    }
}

#[test]
fn subject_match_logs_in_without_relinking() {
    let id = Uuid::now_v7();
    assert_eq!(
        resolve_identity(Some(active(id)), true, None, Eligibility::Off),
        Resolution::Login {
            id,
            link_subject: false
        }
    );
}

#[test]
fn first_login_links_subject_via_verified_email() {
    let id = Uuid::now_v7();
    assert_eq!(
        resolve_identity(None, true, Some(active(id)), Eligibility::Off),
        Resolution::Login {
            id,
            link_subject: true
        }
    );
}

#[test]
fn unverified_email_is_not_matched() {
    let id = Uuid::now_v7();
    // Even with an email-matched candidate, an unverified email must not link.
    assert_eq!(
        resolve_identity(None, false, Some(active(id)), Eligibility::Off),
        Resolution::DenyNoUser
    );
}

#[test]
fn no_account_is_denied_when_provisioning_is_off() {
    assert_eq!(
        resolve_identity(None, true, None, Eligibility::Off),
        Resolution::DenyNoUser
    );
}

#[test]
fn no_account_is_provisioned_when_eligible() {
    assert_eq!(
        resolve_identity(None, true, None, Eligibility::Allowed),
        Resolution::Provision
    );
}

#[test]
fn ineligible_identities_get_their_reason() {
    assert_eq!(
        resolve_identity(None, true, None, Eligibility::DomainNotAllowed),
        Resolution::DenyDomainNotAllowed
    );
    assert_eq!(
        resolve_identity(None, false, None, Eligibility::Unconfirmed),
        Resolution::DenyEmailUnconfirmed
    );
}

#[test]
fn a_matched_account_is_never_provisioned_over() {
    let id = Uuid::now_v7();
    assert_eq!(
        resolve_identity(Some(inactive(id)), true, None, Eligibility::Allowed),
        Resolution::DenyInactive
    );
    assert_eq!(
        resolve_identity(None, true, Some(inactive(id)), Eligibility::Allowed),
        Resolution::DenyInactive
    );
}

#[test]
fn deactivated_subject_match_is_denied() {
    let id = Uuid::now_v7();
    assert_eq!(
        resolve_identity(Some(inactive(id)), true, None, Eligibility::Off),
        Resolution::DenyInactive
    );
}

#[test]
fn deactivated_email_match_is_denied() {
    let id = Uuid::now_v7();
    assert_eq!(
        resolve_identity(None, true, Some(inactive(id)), Eligibility::Off),
        Resolution::DenyInactive
    );
}

#[test]
fn provisioning_is_off_without_domains() {
    assert_eq!(
        eligibility(&policy(&[], true), Some("ada@example.com"), Some(true)),
        Eligibility::Off
    );
}

#[test]
fn listed_domain_with_verified_email_is_allowed() {
    assert_eq!(
        eligibility(
            &policy(&["example.com"], false),
            Some("Ada@Example.COM"),
            Some(true)
        ),
        Eligibility::Allowed
    );
}

#[test]
fn domain_is_the_part_after_the_last_at() {
    let policy = policy(&["example.com"], false);
    assert_eq!(
        eligibility(&policy, Some("a@b@example.com"), Some(true)),
        Eligibility::Allowed
    );
    assert_eq!(
        eligibility(&policy, Some("ada@example.com@evil.test"), Some(true)),
        Eligibility::DomainNotAllowed
    );
}

#[test]
fn subdomains_and_other_domains_are_not_allowed() {
    let policy = policy(&["example.com"], false);
    for email in [
        "ada@sub.example.com",
        "ada@example.org",
        "ada@notexample.com",
    ] {
        assert_eq!(
            eligibility(&policy, Some(email), Some(true)),
            Eligibility::DomainNotAllowed,
            "{email}"
        );
    }
}

#[test]
fn missing_or_domainless_email_cannot_provision() {
    let policy = policy(&["example.com"], true);
    for email in [None, Some("ada"), Some("ada@")] {
        assert_eq!(eligibility(&policy, email, Some(true)), Eligibility::Off);
    }
}

#[test]
fn email_verified_false_is_never_trusted() {
    for trust in [false, true] {
        assert_eq!(
            eligibility(
                &policy(&["example.com"], trust),
                Some("ada@example.com"),
                Some(false)
            ),
            Eligibility::Unconfirmed
        );
    }
}

#[test]
fn absent_email_verified_is_trusted_only_when_configured() {
    let email = Some("ada@example.com");
    assert_eq!(
        eligibility(&policy(&["example.com"], false), email, None),
        Eligibility::Unconfirmed
    );
    assert_eq!(
        eligibility(&policy(&["example.com"], true), email, None),
        Eligibility::Allowed
    );
}

#[test]
fn account_name_prefers_the_trimmed_name_claim() {
    assert_eq!(
        account_name(Some("  Ada Lovelace "), "ada@example.com"),
        "Ada Lovelace"
    );
}

#[test]
fn account_name_falls_back_to_the_email_local_part() {
    for name in [None, Some(""), Some("   ")] {
        assert_eq!(account_name(name, "ada@example.com"), "ada");
    }
    assert_eq!(account_name(None, "@example.com"), "@example.com");
}

#[test]
fn subject_match_takes_precedence_over_email() {
    let subject_id = Uuid::now_v7();
    let email_id = Uuid::now_v7();
    assert_eq!(
        resolve_identity(
            Some(active(subject_id)),
            true,
            Some(active(email_id)),
            Eligibility::Allowed
        ),
        Resolution::Login {
            id: subject_id,
            link_subject: false
        }
    );
}
