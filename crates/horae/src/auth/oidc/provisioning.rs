//! Maps a verified OIDC identity onto the Horae user that may receive a
//! session: by subject, then by verified email on a first login, or by
//! creating an active member when the operator lists the email's domain.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::Identity;
use crate::config::AutoProvision;
use crate::models::user::{Activation, NewPerson};
use crate::ops::people::User;
use crate::ops::{self, people::Existing};
use horae_core::permissions::catalog::BuiltInProfile;

/// Why the last OIDC sign-in was refused, shown once above the call to action.
/// Each variant names only the person's own email from their validated token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoginNotice {
    /// No account matches, and none may be created.
    NoAccount { email: Option<String> },
    /// The person's account is deactivated.
    Deactivated,
    /// Provisioning is on, but not for this email's domain.
    DomainNotAllowed { email: String },
    /// Provisioning is on for the domain, but the provider did not confirm the
    /// email.
    EmailUnconfirmed { email: String },
}

/// A candidate user row for the resolution decision.
struct Candidate {
    id: Uuid,
    active: bool,
}

/// Whether an identity that matches no account may create one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Eligibility {
    /// Provisioning is off, or the token has no usable email.
    Off,
    /// The email's domain is not on the operator's list.
    DomainNotAllowed,
    /// The domain is listed, but the email does not count as verified.
    Unconfirmed,
    Allowed,
}

/// Pure eligibility check. `true` verifies; an absent claim verifies only when
/// the operator trusts it; `false` never does.
fn eligibility(
    policy: &AutoProvision,
    email: Option<&str>,
    email_verified: Option<bool>,
) -> Eligibility {
    if policy.domains.is_empty() {
        return Eligibility::Off;
    }
    let Some((_, domain)) = email.and_then(|e| e.rsplit_once('@')) else {
        return Eligibility::Off;
    };
    if domain.is_empty() {
        return Eligibility::Off;
    }
    if !policy
        .domains
        .iter()
        .any(|d| d.eq_ignore_ascii_case(domain))
    {
        return Eligibility::DomainNotAllowed;
    }
    match email_verified {
        Some(true) => Eligibility::Allowed,
        None if policy.trust_unverified_email => Eligibility::Allowed,
        _ => Eligibility::Unconfirmed,
    }
}

/// What a callback should do with the verified identity.
#[derive(Debug, PartialEq, Eq)]
enum Resolution {
    /// Log in this user; `link_subject` = write the OIDC subject onto the row
    /// (true on a first, email-matched login).
    Login { id: Uuid, link_subject: bool },
    /// No account matches and the identity may create one.
    Provision,
    /// No Horae account matches and none may be created (FR-002).
    DenyNoUser,
    /// The matched account is deactivated — deny sign-in (FR-002).
    DenyInactive,
    /// No account matches, and provisioning is off for this email's domain.
    DenyDomainNotAllowed,
    /// No account matches, and the provider did not confirm the email.
    DenyEmailUnconfirmed,
}

/// Pure resolution: match by subject first, then (only for a verified email) by
/// email. Deactivated matches are denied; an unmatched identity is provisioned
/// only when `eligibility` allows it.
fn resolve_identity(
    by_subject: Option<Candidate>,
    email_verified: bool,
    by_email: Option<Candidate>,
    eligibility: Eligibility,
) -> Resolution {
    if let Some(u) = by_subject {
        return if u.active {
            Resolution::Login {
                id: u.id,
                link_subject: false,
            }
        } else {
            Resolution::DenyInactive
        };
    }
    if email_verified && let Some(u) = by_email {
        return if u.active {
            Resolution::Login {
                id: u.id,
                link_subject: true,
            }
        } else {
            Resolution::DenyInactive
        };
    }
    match eligibility {
        Eligibility::Allowed => Resolution::Provision,
        Eligibility::DomainNotAllowed => Resolution::DenyDomainNotAllowed,
        Eligibility::Unconfirmed => Resolution::DenyEmailUnconfirmed,
        Eligibility::Off => Resolution::DenyNoUser,
    }
}

/// An account that may receive a session; `created` when this sign-in made it.
#[derive(Debug)]
pub(crate) struct SignIn {
    pub(crate) user: User,
    pub(crate) created: bool,
}

/// Resolve a verified identity to the account that may receive a session, or
/// to the notice explaining why there is none.
pub(crate) async fn resolve_user(
    db: &sqlx::PgPool,
    identity: &Identity,
    policy: &AutoProvision,
) -> anyhow::Result<Result<SignIn, LoginNotice>> {
    // The trust switch is for provisioning only: binding an existing account by
    // email always needs an explicit `true` (FR-010).
    let email_verified = identity.email_verified == Some(true);
    // Where an account holds the email or subject but cannot be signed in to,
    // "no account exists for <email>" would be false. The generic notice says
    // nothing about the address.
    let unmatched = || LoginNotice::NoAccount { email: None };
    let by_subject = fetch_by_identity(db, identity).await?;
    // Only an as-yet-unlinked account may be claimed by a verified email — a
    // first-login bootstrap. This prevents a different subject with a matching
    // email from silently rebinding an already-linked account.
    let by_email = match (email_verified, &identity.email) {
        (true, Some(email)) => fetch_unlinked_by_email(db, email).await?,
        _ => None,
    };

    let resolution = resolve_identity(
        by_subject.as_ref().map(candidate),
        email_verified,
        by_email.as_ref().map(candidate),
        eligibility(policy, identity.email.as_deref(), identity.email_verified),
    );

    let (user, link_subject) = match resolution {
        Resolution::Login { id, link_subject } => {
            // The matched user is whichever lookup produced `id`.
            let user = by_subject
                .into_iter()
                .chain(by_email)
                .find(|u| u.id == id)
                .ok_or_else(|| anyhow::anyhow!("resolved user vanished"))?;
            (user, link_subject)
        }
        Resolution::Provision => {
            let email = identity
                .email
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("provisioning needs an email"))?;
            if let Some(user) = provision(db, identity, email).await? {
                tracing::info!(user_id = %user.id, "OIDC sign-in created an account");
                return Ok(Ok(SignIn {
                    user,
                    created: true,
                }));
            }
            // Nothing was created: a concurrent first sign-in for this identity
            // won, or the email or subject is held by another account.
            return Ok(match fetch_by_identity(db, identity).await? {
                Some(user) if user.active => Ok(SignIn {
                    user,
                    created: false,
                }),
                Some(_) => Err(LoginNotice::Deactivated),
                None => {
                    tracing::warn!("OIDC login denied: email or subject held by another account");
                    Err(unmatched())
                }
            });
        }
        Resolution::DenyNoUser => {
            tracing::warn!("OIDC login denied: no account for subject/email");
            return Ok(Err(LoginNotice::NoAccount {
                email: identity.email.clone(),
            }));
        }
        Resolution::DenyInactive => {
            tracing::warn!("OIDC login denied: account is deactivated");
            return Ok(Err(LoginNotice::Deactivated));
        }
        Resolution::DenyDomainNotAllowed => {
            tracing::warn!("OIDC login denied: no account, and the email's domain is not listed");
            return Ok(Err(LoginNotice::DomainNotAllowed {
                email: identity.email.clone().unwrap_or_default(),
            }));
        }
        Resolution::DenyEmailUnconfirmed => {
            tracing::warn!("OIDC login denied: no account, and the email is not verified");
            return Ok(Err(LoginNotice::EmailUnconfirmed {
                email: identity.email.clone().unwrap_or_default(),
            }));
        }
    };

    if link_subject {
        let linked = sqlx::query_as!(
            User,
            r#"UPDATE users SET oidc_subject = $1, oidc_issuer = $3
               WHERE id = $2 AND oidc_subject IS NULL AND active
               RETURNING id, org_id, email, name, oidc_subject,
                         cost_rate_cents, billable_rate_cents, active,
                         created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
            identity.subject,
            user.id,
            identity.issuer,
        )
        .fetch_optional(db)
        .await?;
        if let Some(user) = linked {
            return Ok(Ok(SignIn {
                user,
                created: false,
            }));
        }
        // A concurrent callback may have linked the row. Only the same identity
        // winning that same active account is allowed to receive a session.
        return Ok(fetch_by_identity(db, identity)
            .await?
            .filter(|current| current.id == user.id && current.active)
            .map(|user| SignIn {
                user,
                created: false,
            })
            .ok_or_else(unmatched));
    }

    Ok(Ok(SignIn {
        user,
        created: false,
    }))
}

/// Create an active member account bound to the identity. Creates nothing when
/// any account holds the email (in any letter case or state), when another
/// account holds the subject, or when there is not exactly one organization.
/// Every user-creating path holds the email lock from
/// [`crate::db::lock_new_user_email`], and the unique subject constraint makes
/// concurrent first sign-ins for one identity create a single row.
async fn provision(
    db: &sqlx::PgPool,
    identity: &Identity,
    email: &str,
) -> anyhow::Result<Option<User>> {
    let organizations = sqlx::query_scalar!("SELECT id FROM organizations")
        .fetch_all(db)
        .await?;
    let [org_id] = organizations[..] else {
        return Ok(None);
    };
    let mut write = ops::begin(db, ops::Caller::system(org_id)).await?;
    let result = add_member(&mut write.tx, &write.actor, identity, email).await;
    // The sign-in reports `UserCreated` itself once the session exists.
    match write.commit(result, &ops::NoEvents).await {
        Ok(user) => Ok(user),
        // A concurrent sign-in won the email or the subject.
        Err(ops::OpError::Conflict(_)) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// Adds the member through `ops::people::add`, which locks the organization,
/// then the email, and binds the identity. `None` when an account holds the
/// email.
async fn add_member(
    tx: &mut ops::Tx,
    actor: &ops::Actor,
    identity: &Identity,
    email: &str,
) -> Result<ops::Done<Option<User>>, ops::OpError> {
    let person = NewPerson {
        email: email.to_owned(),
        name: account_name(identity.name.as_deref(), email),
    };
    let added = ops::people::add(
        tx,
        actor,
        &[person],
        BuiltInProfile::Member,
        Activation::Active,
        Existing::Skip,
    )
    .await?;
    let Some(person) = added.value.added.first() else {
        return Ok(ops::Done::new(None));
    };
    let user = sqlx::query_as!(
        User,
        r#"UPDATE users SET oidc_subject = $2, oidc_issuer = $3
           WHERE id = $1
           RETURNING id, org_id, email, name, oidc_subject,
                     cost_rate_cents, billable_rate_cents, active,
                     created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
        person.id,
        identity.subject,
        identity.issuer,
    )
    .fetch_one(&mut **tx)
    .await
    .map_err(|error| {
        match error
            .as_database_error()
            .and_then(|error| error.constraint())
        {
            Some("users_oidc_subject_key") => ops::OpError::conflict("Subject already bound"),
            _ => ops::OpError::from(error),
        }
    })?;
    Ok(ops::Done::new(Some(user)))
}

/// The token's display name, or the email's local part when it has none.
fn account_name(name: Option<&str>, email: &str) -> String {
    name.map(str::trim)
        .filter(|name| !name.is_empty())
        .or_else(|| {
            email
                .rsplit_once('@')
                .map(|(local, _)| local)
                .filter(|local| !local.is_empty())
        })
        .unwrap_or(email)
        .to_string()
}

/// Reports the sign-in to plugins once the session exists: `UserCreated` when
/// this sign-in made the account, then `UserLoggedIn`.
pub(crate) async fn announce_sign_in(
    state: &crate::state::AppState,
    sign_in: &SignIn,
) -> anyhow::Result<()> {
    let SignIn { user, created } = sign_in;
    let mut connection = state.db.acquire().await?;
    let payload = crate::plugin::event::UserPayload {
        method: Some("oidc".into()),
        ..crate::plugin::event::user_payload(
            &mut connection,
            user.org_id,
            user.id,
            &user.email,
            &user.name,
        )
        .await?
    };
    drop(connection);
    let occurred_at = chrono::Utc::now();
    if *created {
        state
            .plugins
            .dispatch(crate::plugin::AppEvent::UserCreated {
                occurred_at,
                org_id: user.org_id,
                user: payload.clone(),
            });
    }
    state
        .plugins
        .dispatch(crate::plugin::AppEvent::UserLoggedIn {
            occurred_at,
            org_id: user.org_id,
            user: payload,
        });
    Ok(())
}

fn candidate(u: &User) -> Candidate {
    Candidate {
        id: u.id,
        active: u.active,
    }
}

/// The user already linked to this issuer and subject, if any. A subject is
/// only stable within its issuer, and a legacy binding with no recorded issuer
/// matches nothing until an operator records it (`horae user link-oidc`).
async fn fetch_by_identity(db: &sqlx::PgPool, identity: &Identity) -> anyhow::Result<Option<User>> {
    let user = sqlx::query_as!(
        User,
        r#"SELECT id, org_id, email, name, oidc_subject,
                  cost_rate_cents, billable_rate_cents, active,
                  created_at as "created_at: chrono::DateTime<chrono::Utc>"
           FROM users WHERE oidc_issuer = $1 AND oidc_subject = $2"#,
        identity.issuer,
        identity.subject,
    )
    .fetch_optional(db)
    .await?;
    Ok(user)
}

/// The user with this email that has **not** yet been linked to any OIDC
/// subject — the only account a verified email is allowed to claim (FR-002).
async fn fetch_unlinked_by_email(db: &sqlx::PgPool, email: &str) -> anyhow::Result<Option<User>> {
    let user = sqlx::query_as!(
        User,
        r#"SELECT id, org_id, email, name, oidc_subject,
                  cost_rate_cents, billable_rate_cents, active,
                  created_at as "created_at: chrono::DateTime<chrono::Utc>"
           FROM users WHERE email = $1 AND oidc_subject IS NULL"#,
        email,
    )
    .fetch_optional(db)
    .await?;
    Ok(user)
}

#[cfg(test)]
mod tests;
