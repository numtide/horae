//! `horae user link-oidc`: the local operator binds an OIDC issuer and subject to
//! an existing account (constitution, Principle IV exception).
//!
//! The operator takes the issuer and subject from the provider's administration
//! records or a validated ID token, never from an email address. A binding is
//! never replaced: the only change to an existing one is recording the issuer on
//! a legacy binding that already holds the same subject.

use openidconnect::url::{Host, Url};
use sqlx::PgPool;
use uuid::Uuid;

/// The unique index that keeps one subject on at most one account.
const SUBJECT_CONSTRAINT: &str = "users_oidc_subject_key";

/// What a successful link did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkOutcome {
    /// The unbound account now holds the issuer and subject.
    Linked,
    /// The account already held exactly this issuer and subject.
    Unchanged,
    /// The account's legacy binding with this subject now records the issuer.
    IssuerRecorded,
}

/// Why a link was refused. A refusal writes nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejection {
    InvalidSubject,
    InvalidIssuer,
    IssuerMismatch,
    /// `HORAE_OIDC_ISSUER` is set but not UTF-8, so the issuer cannot be checked.
    UnreadableConfiguredIssuer,
    UserNotFound,
    Inactive,
    SubjectInUse,
    AlreadyLinked,
}

#[derive(Debug)]
pub enum LinkError {
    Rejected(Rejection),
    Database(sqlx::Error),
}

impl From<sqlx::Error> for LinkError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

impl From<Rejection> for LinkError {
    fn from(rejection: Rejection) -> Self {
        Self::Rejected(rejection)
    }
}

impl LinkOutcome {
    /// The operator-facing result. It never names the issuer or subject.
    pub fn message(self, user_id: Uuid) -> String {
        match self {
            Self::Linked => format!("Linked user {user_id} to the OIDC identity."),
            Self::Unchanged => {
                format!("User {user_id} is already linked to this OIDC identity; nothing changed.")
            }
            Self::IssuerRecorded => {
                format!("Recorded the issuer on the existing OIDC binding of user {user_id}.")
            }
        }
    }
}

impl Rejection {
    /// The operator-facing reason. It never names the issuer or subject.
    pub fn message(self, user_id: Uuid) -> String {
        match self {
            Self::InvalidSubject => {
                "The subject must be 1–255 bytes with no leading or trailing whitespace.".into()
            }
            Self::InvalidIssuer => "The issuer must be an https URL (or http on a loopback host) \
                                    with no query or fragment."
                .into(),
            Self::IssuerMismatch => "The issuer does not match HORAE_OIDC_ISSUER exactly.".into(),
            Self::UnreadableConfiguredIssuer => {
                "HORAE_OIDC_ISSUER is not valid UTF-8; the issuer cannot be checked.".into()
            }
            Self::UserNotFound => format!("No user with ID {user_id}."),
            Self::Inactive => {
                format!("User {user_id} is deactivated; reactivate the account before linking it.")
            }
            Self::SubjectInUse => "Another user is already linked to this subject.".into(),
            Self::AlreadyLinked => format!(
                "User {user_id} is already linked to a different OIDC identity. \
                 Horae does not replace bindings."
            ),
        }
    }
}

/// Check the operator's input before any database access. Values are compared,
/// never trimmed or normalized: they must match the provider's token claims
/// byte for byte. `configured_issuer` is a non-empty `HORAE_OIDC_ISSUER`.
pub fn validate(
    issuer: &str,
    subject: &str,
    configured_issuer: Option<&str>,
) -> Result<(), Rejection> {
    validate_subject(subject)?;
    validate_issuer(issuer)?;
    match configured_issuer {
        Some(configured) if configured != issuer => Err(Rejection::IssuerMismatch),
        _ => Ok(()),
    }
}

/// OIDC Core caps `sub` at 255 ASCII characters; counting UTF-8 bytes keeps that
/// limit for ASCII and stays strict for anything else. Surrounding whitespace is
/// almost certainly a paste error, so it is refused rather than silently trimmed.
fn validate_subject(subject: &str) -> Result<(), Rejection> {
    if subject.is_empty() || subject.len() > 255 || subject.trim() != subject {
        return Err(Rejection::InvalidSubject);
    }
    Ok(())
}

/// Parsed only to check its shape; the caller keeps the original string, since
/// parsing normalizes (e.g. adds a trailing slash to a bare origin).
fn validate_issuer(issuer: &str) -> Result<(), Rejection> {
    // The parser silently drops tabs and newlines, reads `\` as `/`, and fills
    // in a missing or extra `/` before the host. A token's `iss` would never be
    // spelled like that, so such input is refused rather than repaired.
    let authority = issuer.split_once(':').map(|(_, rest)| rest);
    let well_formed = authority
        .and_then(|rest| rest.strip_prefix("//"))
        .is_some_and(|host| !host.starts_with('/'));
    if !well_formed || issuer.contains(|c: char| c.is_ascii_control() || c == '\\') {
        return Err(Rejection::InvalidIssuer);
    }
    let url = Url::parse(issuer).map_err(|_| Rejection::InvalidIssuer)?;
    let loopback = match url.host() {
        Some(Host::Domain(domain)) => domain == "localhost",
        Some(Host::Ipv4(ip)) => ip.is_loopback(),
        Some(Host::Ipv6(ip)) => ip.is_loopback(),
        None => false,
    };
    let secure = url.scheme() == "https" || (url.scheme() == "http" && loopback);
    // `Url::parse` strips surrounding whitespace, so compare against the input.
    if !secure || url.query().is_some() || url.fragment().is_some() || issuer.trim() != issuer {
        return Err(Rejection::InvalidIssuer);
    }
    Ok(())
}

/// The account `link` would change.
#[derive(Debug, Clone)]
struct Target {
    active: bool,
    issuer: Option<String>,
    subject: Option<String>,
}

/// The binding rules, with `holder` the account that currently holds `subject`
/// (possibly `user_id` itself).
fn decide(
    user_id: Uuid,
    target: Option<&Target>,
    holder: Option<Uuid>,
    issuer: &str,
    subject: &str,
) -> Result<LinkOutcome, Rejection> {
    let target = target.ok_or(Rejection::UserNotFound)?;
    if !target.active {
        return Err(Rejection::Inactive);
    }
    match (target.subject.as_deref(), target.issuer.as_deref()) {
        (Some(bound), Some(bound_issuer)) if bound == subject && bound_issuer == issuer => {
            Ok(LinkOutcome::Unchanged)
        }
        (Some(bound), None) if bound == subject => Ok(LinkOutcome::IssuerRecorded),
        (Some(_), _) => Err(Rejection::AlreadyLinked),
        (None, _) if holder.is_some_and(|holder| holder != user_id) => Err(Rejection::SubjectInUse),
        (None, _) => Ok(LinkOutcome::Linked),
    }
}

/// Bind `issuer` and `subject` to `user_id` in one transaction that holds the
/// account's row lock, so sign-in, deactivation and other links are ordered
/// against it. The unique subject index decides between different accounts.
pub async fn link(
    db: &PgPool,
    user_id: Uuid,
    issuer: &str,
    subject: &str,
) -> Result<LinkOutcome, LinkError> {
    let mut tx = db.begin().await?;
    let target = sqlx::query_as!(
        Target,
        "SELECT active, oidc_issuer AS issuer, oidc_subject AS subject
         FROM users WHERE id = $1 FOR UPDATE",
        user_id,
    )
    .fetch_optional(&mut *tx)
    .await?;
    let holder = sqlx::query_scalar!("SELECT id FROM users WHERE oidc_subject = $1", subject)
        .fetch_optional(&mut *tx)
        .await?;

    let outcome = decide(user_id, target.as_ref(), holder, issuer, subject)?;
    if outcome != LinkOutcome::Unchanged {
        let written = sqlx::query!(
            "UPDATE users SET oidc_issuer = $2, oidc_subject = $3 WHERE id = $1",
            user_id,
            issuer,
            subject,
        )
        .execute(&mut *tx)
        .await;
        match written {
            // Another account took the subject after it was read above.
            Err(sqlx::Error::Database(error))
                if error.code().as_deref() == Some("23505")
                    && error.constraint() == Some(SUBJECT_CONSTRAINT) =>
            {
                return Err(Rejection::SubjectInUse.into());
            }
            written => {
                written?;
            }
        }
    }
    tx.commit().await?;
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ISSUER: &str = "https://id.example.com";

    #[test]
    fn subject_is_kept_exactly_and_rejected_rather_than_trimmed() {
        for ok in [
            "a",
            "Case-Sensitive",
            "inner space",
            "ünïcödé",
            &"x".repeat(255),
        ] {
            assert_eq!(validate_subject(ok), Ok(()), "{ok:?}");
        }
        for bad in [
            "",
            " lead",
            "trail ",
            "\tlead",
            "trail\n",
            "\u{a0}nbsp",
            "nbsp\u{a0}",
        ] {
            assert_eq!(
                validate_subject(bad),
                Err(Rejection::InvalidSubject),
                "{bad:?}"
            );
        }
        assert_eq!(
            validate_subject(&"x".repeat(256)),
            Err(Rejection::InvalidSubject)
        );
    }

    #[test]
    fn subject_length_counts_utf8_bytes() {
        // 'é' is two bytes: 127 of them fit in 255 bytes, 128 do not.
        assert_eq!(validate_subject(&"é".repeat(127)), Ok(()));
        assert_eq!(
            validate_subject(&"é".repeat(128)),
            Err(Rejection::InvalidSubject)
        );
    }

    #[test]
    fn issuer_must_be_https_or_loopback_http_without_query_or_fragment() {
        for ok in [
            ISSUER,
            "https://id.example.com/",
            "https://id.example.com/realms/Main",
            "http://localhost:5556/dex",
            "http://127.0.0.1:5556/dex",
            "http://[::1]:5556/dex",
        ] {
            assert_eq!(validate_issuer(ok), Ok(()), "{ok}");
        }
        for bad in [
            "",
            "id.example.com",
            "/relative",
            "http://id.example.com",
            "ftp://id.example.com",
            "https://id.example.com?tenant=a",
            "https://id.example.com/?",
            "https://id.example.com#frag",
            " https://id.example.com",
            "https://id.example.com ",
        ] {
            assert_eq!(
                validate_issuer(bad),
                Err(Rejection::InvalidIssuer),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn issuer_rejects_what_the_url_parser_would_silently_repair() {
        // A parser drops inner tabs and newlines, treats a backslash like a
        // slash and fills in a missing `//`. The token's `iss` would never be
        // spelled that way, so the binding could never match.
        for bad in [
            "https://id.exa\tmple.com",
            "https://id.example.com/\r",
            "https://id.example.com/a\nb",
            "https://id.example.com\\realm",
            "https:\\\\id.example.com",
            "https:id.example.com",
            "https:/id.example.com",
            "https:///id.example.com",
            "https://id.example.com/\u{7f}",
        ] {
            assert_eq!(
                validate_issuer(bad),
                Err(Rejection::InvalidIssuer),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn issuer_spelling_is_accepted_as_given() {
        // Validation never rewrites: case and percent-encoding are the provider's.
        for ok in [
            "HTTPS://ID.Example.COM",
            "https://id.example.com/Realms/Main%2FSub",
            "Http://LocalHost:5556/dex",
        ] {
            assert_eq!(validate(ok, "s", Some(ok)), Ok(()), "{ok}");
        }
    }

    #[test]
    fn configured_issuer_must_match_exactly() {
        assert_eq!(validate(ISSUER, "s", Some(ISSUER)), Ok(()));
        assert_eq!(validate(ISSUER, "s", None), Ok(()));
        // A trailing slash is a different issuer string.
        assert_eq!(
            validate(ISSUER, "s", Some("https://id.example.com/")),
            Err(Rejection::IssuerMismatch)
        );
        // Input errors are reported before the configuration comparison.
        assert_eq!(
            validate(ISSUER, "", Some("https://other")),
            Err(Rejection::InvalidSubject)
        );
    }

    fn target(active: bool, issuer: Option<&str>, subject: Option<&str>) -> Target {
        Target {
            active,
            issuer: issuer.map(Into::into),
            subject: subject.map(Into::into),
        }
    }

    #[test]
    fn decide_follows_the_transition_table() {
        let u = Uuid::now_v7();
        let other = Uuid::now_v7();
        let unbound = target(true, None, None);
        let legacy = target(true, None, Some("s"));
        let bound = target(true, Some(ISSUER), Some("s"));
        let cases = [
            (None, None, Err(Rejection::UserNotFound)),
            (
                Some(target(false, None, None)),
                None,
                Err(Rejection::Inactive),
            ),
            (
                Some(target(false, Some(ISSUER), Some("s"))),
                Some(u),
                Err(Rejection::Inactive),
            ),
            (Some(bound.clone()), Some(u), Ok(LinkOutcome::Unchanged)),
            (
                Some(legacy.clone()),
                Some(u),
                Ok(LinkOutcome::IssuerRecorded),
            ),
            (Some(unbound.clone()), None, Ok(LinkOutcome::Linked)),
            (Some(unbound), Some(other), Err(Rejection::SubjectInUse)),
            (
                Some(target(true, Some("https://other.example"), Some("s"))),
                Some(u),
                Err(Rejection::AlreadyLinked),
            ),
            (
                Some(target(true, Some(ISSUER), Some("t"))),
                None,
                Err(Rejection::AlreadyLinked),
            ),
            (
                Some(target(true, None, Some("t"))),
                Some(other),
                Err(Rejection::AlreadyLinked),
            ),
            (
                Some(target(true, Some(ISSUER), Some("S"))),
                None,
                Err(Rejection::AlreadyLinked),
            ),
        ];
        for (row, holder, expected) in cases {
            assert_eq!(
                decide(u, row.as_ref(), holder, ISSUER, "s"),
                expected,
                "{row:?} {holder:?}"
            );
        }
    }
}
