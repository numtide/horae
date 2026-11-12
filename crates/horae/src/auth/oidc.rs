//! Production OIDC authentication (authorization-code flow with PKCE).
//!
//! `GET /auth/login` serves the sign-in landing page; its button hands off to
//! `GET /auth/oidc/start`, which redirects to the provider. `GET /auth/callback`
//! completes the exchange, verifies the ID token, and maps the verified identity
//! onto a Horae user. Accounts are created by admins (FR-002), so a first OIDC
//! login *links* the subject to a user matched by verified email. Only when the
//! operator lists the email's domain does a first login that matches no account
//! create one (an active member). Deactivated users are denied at sign-in, and a
//! sign-in refused because of the account leaves a one-time notice that
//! `/auth/login` shows.

pub mod link;

use axum::extract::Query;
use axum::response::{Html, IntoResponse, Redirect};

use crate::auth::page::{LoginNotice, LoginVariant, render};
use openidconnect::core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata};
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
};
use serde::{Deserialize, Serialize};
use tower_sessions::Session;
use uuid::Uuid;

use crate::auth::session::set_session_user_id;
use crate::config::{AutoProvision, OidcConfig};
use crate::models::User;
use horae_core::types::OrgRole;

/// Session key holding the in-flight authorization request's CSRF, PKCE, and
/// nonce secrets between `/auth/login` and `/auth/callback`.
const OIDC_FLOW_KEY: &str = "oidc_flow";

/// Session key holding the [`LoginNotice`] of a refused sign-in until
/// `/auth/login` shows it. Kept server-side so the email never enters a URL.
const LOGIN_NOTICE_KEY: &str = "login_notice";

/// The one-time secrets that tie a callback back to the login that started it.
#[derive(Serialize, Deserialize)]
struct OidcFlow {
    csrf: String,
    pkce_verifier: String,
    nonce: String,
}

#[derive(Deserialize)]
pub struct CallbackParams {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

/// `GET /auth/login` for OIDC deployments: serve the sign-in landing page. The
/// page's button links to `/auth/oidc/start`, which begins the flow. A notice
/// left by a refused sign-in is shown once.
pub async fn login_page(session: Session) -> impl IntoResponse {
    let label = crate::state::global_state()
        .await
        .oidc
        .as_ref()
        .map(|c| c.button_label.clone())
        .unwrap_or_else(|| crate::config::DEFAULT_OIDC_BUTTON_LABEL.into());
    let notice = session
        .remove::<LoginNotice>(LOGIN_NOTICE_KEY)
        .await
        .ok()
        .flatten();
    Html(render(
        LoginVariant::Oidc { cta_label: label },
        notice.as_ref(),
    ))
}

/// `GET /auth/oidc/start`: begin the authorization-code flow. Stashes the
/// CSRF/PKCE/nonce secrets in the session and redirects to the provider. Falls
/// back to `/auth/login` on any setup error.
pub async fn start(session: Session) -> axum::response::Response {
    let state = crate::state::global_state().await;
    let Some(cfg) = state.oidc.clone() else {
        // Neither dev-login nor OIDC configured. Return a plain error rather than
        // redirecting back to `/auth/login`, which would loop.
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Authentication is not configured.",
        )
            .into_response();
    };

    let build = tokio::task::spawn_blocking(move || build_authorization(&cfg)).await;
    let auth = match build {
        Ok(Ok(a)) => a,
        Ok(Err(e)) => {
            tracing::error!("OIDC login setup failed: {e}");
            return Redirect::to("/auth/login").into_response();
        }
        Err(e) => {
            tracing::error!("OIDC login task panicked: {e}");
            return Redirect::to("/auth/login").into_response();
        }
    };

    let flow = OidcFlow {
        csrf: auth.csrf,
        pkce_verifier: auth.pkce_verifier,
        nonce: auth.nonce,
    };
    if let Err(e) = session.insert(OIDC_FLOW_KEY, flow).await {
        tracing::error!("failed to store OIDC flow in session: {e}");
        return Redirect::to("/auth/login").into_response();
    }

    Redirect::to(&auth.url).into_response()
}

/// `GET /auth/callback`: validate state, exchange the code (PKCE), verify the
/// ID token and nonce, then resolve the identity onto a Horae user.
pub async fn callback(session: Session, Query(params): Query<CallbackParams>) -> impl IntoResponse {
    let state = crate::state::global_state().await;
    let Some(cfg) = state.oidc.clone() else {
        return Redirect::to("/auth/login");
    };

    // The flow secrets are single-use: consume them regardless of outcome.
    let flow: Option<OidcFlow> = session.get(OIDC_FLOW_KEY).await.ok().flatten();
    let _ = session.remove::<OidcFlow>(OIDC_FLOW_KEY).await;
    let Some(flow) = flow else {
        tracing::warn!("OIDC callback with no in-flight flow");
        return Redirect::to("/auth/login");
    };

    if params.error.is_some() {
        tracing::warn!("OIDC provider returned error: {:?}", params.error);
        return Redirect::to("/auth/login");
    }

    // CSRF: the returned `state` must equal the value we generated at login.
    let (Some(returned_state), Some(code)) = (params.state, params.code) else {
        tracing::warn!("OIDC callback missing state or code");
        return Redirect::to("/auth/login");
    };
    if returned_state != flow.csrf {
        tracing::warn!("OIDC callback state mismatch (possible CSRF)");
        return Redirect::to("/auth/login");
    }

    let policy = cfg.auto_provision.clone();
    // Exchange the code and verify the ID token off the async runtime (blocking).
    let identity = tokio::task::spawn_blocking(move || {
        exchange_and_verify(&cfg, code, &flow.pkce_verifier, &flow.nonce)
    })
    .await;
    let identity = match identity {
        Ok(Ok(id)) => id,
        Ok(Err(e)) => {
            tracing::warn!("OIDC token exchange/verification failed: {e}");
            return Redirect::to("/auth/login");
        }
        Err(e) => {
            tracing::error!("OIDC callback task panicked: {e}");
            return Redirect::to("/auth/login");
        }
    };

    match resolve_and_login(state, &session, &identity, &policy).await {
        Ok(true) => Redirect::to("/"),
        Ok(false) => Redirect::to("/auth/login"),
        Err(e) => {
            tracing::error!("OIDC user resolution failed: {e}");
            Redirect::to("/auth/login")
        }
    }
}

/// The verified identity claims we act on.
struct Identity {
    /// The token's `iss`, spelled exactly as the provider sent it.
    issuer: String,
    subject: String,
    email: Option<String>,
    /// `None` when the token has no `email_verified` claim, which some
    /// providers (tsidp) never send.
    email_verified: Option<bool>,
    /// The token's `name` claim, if any.
    name: Option<String>,
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
struct SignIn {
    user: User,
    created: bool,
}

/// Resolve a verified identity to the account that may receive a session, or
/// to the notice explaining why there is none.
async fn resolve_user(
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
                         org_role as "org_role: OrgRole",
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
    let mut tx = db.begin().await?;
    sqlx::query!("SET TRANSACTION ISOLATION LEVEL READ COMMITTED")
        .execute(&mut *tx)
        .await?;
    // Lock the organization row before the email, the order every
    // user-creating path follows, so none of them can deadlock another.
    let organizations = sqlx::query_scalar!("SELECT id FROM organizations FOR KEY SHARE")
        .fetch_all(&mut *tx)
        .await?;
    let [org_id] = organizations[..] else {
        return Ok(None);
    };
    if crate::db::lock_new_user_email(&mut tx, email).await? {
        return Ok(None);
    }
    let user = sqlx::query_as!(
        User,
        r#"INSERT INTO users (id, org_id, email, name, org_role, oidc_subject, oidc_issuer)
           VALUES ($1, $7, $2, $3, $4, $5, $6)
           ON CONFLICT DO NOTHING
           RETURNING id, org_id, email, name, oidc_subject,
                     org_role as "org_role: OrgRole",
                     cost_rate_cents, billable_rate_cents, active,
                     created_at as "created_at: chrono::DateTime<chrono::Utc>""#,
        Uuid::now_v7(),
        email,
        account_name(identity.name.as_deref(), email),
        OrgRole::Member as OrgRole,
        identity.subject,
        identity.issuer,
        org_id,
    )
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(user) = &user {
        crate::server_fns::provisioning::provision(
            &mut tx,
            org_id,
            user.id,
            horae_core::permissions::catalog::BuiltInProfile::Member,
        )
        .await?;
    }
    tx.commit().await?;
    Ok(user)
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

/// Resolve the identity before creating a session or dispatching events. A
/// refusal stores its notice for `/auth/login`.
async fn resolve_and_login(
    state: &'static crate::state::AppState,
    session: &Session,
    identity: &Identity,
    policy: &AutoProvision,
) -> anyhow::Result<bool> {
    let SignIn { user, created } = match resolve_user(&state.db, identity, policy).await? {
        Ok(sign_in) => sign_in,
        Err(notice) => {
            session.insert(LOGIN_NOTICE_KEY, notice).await?;
            return Ok(false);
        }
    };

    // Rotate the session id on the privilege change to defeat session fixation:
    // any pre-auth id an attacker may have fixed is discarded before we mark the
    // session authenticated.
    session.cycle_id().await?;
    set_session_user_id(session, user.id).await?;

    let mut connection = state.db.acquire().await?;
    let payload = crate::plugin::event::UserPayload {
        method: Some("oidc".into()),
        ..crate::server_fns::user_payload(
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
    if created {
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

    Ok(true)
}

fn candidate(u: &crate::models::User) -> Candidate {
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
                  org_role as "org_role: OrgRole",
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
                  org_role as "org_role: OrgRole",
                  cost_rate_cents, billable_rate_cents, active,
                  created_at as "created_at: chrono::DateTime<chrono::Utc>"
           FROM users WHERE email = $1 AND oidc_subject IS NULL"#,
        email,
    )
    .fetch_optional(db)
    .await?;
    Ok(user)
}

/// The authorization request to send the user to.
struct Authorization {
    url: String,
    csrf: String,
    pkce_verifier: String,
    nonce: String,
}

/// Discover the provider and build the authorization URL + one-time secrets.
/// Blocking (network discovery) — call from `spawn_blocking`.
fn build_authorization(cfg: &OidcConfig) -> anyhow::Result<Authorization> {
    let client = discover_client(cfg)?;
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (url, csrf, nonce) = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    Ok(Authorization {
        url: url.to_string(),
        csrf: csrf.secret().clone(),
        pkce_verifier: pkce_verifier.secret().clone(),
        nonce: nonce.secret().clone(),
    })
}

/// Exchange the authorization code and verify the ID token against `nonce`.
/// Blocking (network token request) — call from `spawn_blocking`.
fn exchange_and_verify(
    cfg: &OidcConfig,
    code: String,
    pkce_verifier: &str,
    nonce: &str,
) -> anyhow::Result<Identity> {
    let client = discover_client(cfg)?;

    let token = client
        .exchange_code(AuthorizationCode::new(code))
        .set_pkce_verifier(PkceCodeVerifier::new(pkce_verifier.to_string()))
        .request(http_client_with_timeout)?;

    let id_token = token
        .id_token()
        .ok_or_else(|| anyhow::anyhow!("provider returned no ID token"))?;

    // `client_id` is always required by the base verifier; this only widens which
    // *additional* audiences are tolerated (e.g. Zitadel's project ID). With an
    // empty list the closure always returns false — i.e. strict, client_id only.
    let additional = cfg.additional_audiences.clone();
    let verifier = client
        .id_token_verifier()
        .set_other_audience_verifier_fn(move |aud| audience_is_trusted(&additional, aud.as_str()));

    let claims = id_token.claims(&verifier, &Nonce::new(nonce.to_string()))?;

    Ok(Identity {
        // `as_str` keeps the original spelling; `url().as_str()` would normalize it.
        issuer: claims.issuer().as_str().to_string(),
        subject: claims.subject().as_str().to_string(),
        email: claims.email().map(|e| e.as_str().to_string()),
        email_verified: claims.email_verified(),
        name: claims
            .name()
            .and_then(|name| name.get(None))
            .map(|name| name.as_str().to_string()),
    })
}

/// Whether an extra ID-token audience (one that is not `client_id`) is on the
/// operator-configured trust list.
fn audience_is_trusted(additional: &[String], aud: &str) -> bool {
    additional.iter().any(|a| a == aud)
}

/// Wall-clock bound on each OIDC HTTP call (discovery, token exchange), so a
/// slow or hung provider cannot stall a login or pin a `spawn_blocking` thread.
const OIDC_HTTP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// A drop-in for `openidconnect::ureq::http_client` that bounds the request with
/// [`OIDC_HTTP_TIMEOUT`]. The upstream client uses the global `ureq::get/post`
/// with no timeout; this routes through an agent that has one.
fn http_client_with_timeout(
    request: openidconnect::HttpRequest,
) -> Result<openidconnect::HttpResponse, openidconnect::ureq::Error> {
    use openidconnect::http::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
    use openidconnect::http::{Method, StatusCode};
    use openidconnect::ureq::Error;

    let agent = ureq::AgentBuilder::new().timeout(OIDC_HTTP_TIMEOUT).build();

    let is_post = request.method == Method::POST;
    let mut req = if is_post {
        agent.post(request.url.as_str())
    } else {
        agent.get(request.url.as_str())
    };
    for (name, value) in &request.headers {
        req = req.set(
            name.as_str(),
            value.to_str().map_err(|_| {
                Error::Other(format!(
                    "invalid {name} header value {:?}",
                    value.as_bytes()
                ))
            })?,
        );
    }

    let response = if is_post {
        req.send_bytes(&request.body)
    } else {
        req.call()
    }
    .map_err(Box::new)?;

    Ok(openidconnect::HttpResponse {
        status_code: StatusCode::from_u16(response.status())
            .map_err(|err| Error::Http(err.into()))?,
        headers: [(
            CONTENT_TYPE,
            HeaderValue::from_str(response.content_type())
                .map_err(|err| Error::Http(err.into()))?,
        )]
        .into_iter()
        .collect::<HeaderMap>(),
        body: response.into_string()?.as_bytes().into(),
    })
}

/// Discover provider metadata and build a client. Blocking (network).
fn discover_client(cfg: &OidcConfig) -> anyhow::Result<CoreClient> {
    let issuer = IssuerUrl::new(cfg.issuer.clone())?;
    let metadata = CoreProviderMetadata::discover(&issuer, http_client_with_timeout)?;
    let client = CoreClient::from_provider_metadata(
        metadata,
        ClientId::new(cfg.client_id.clone()),
        Some(ClientSecret::new(cfg.client_secret.clone())),
    )
    .set_redirect_uri(RedirectUrl::new(cfg.redirect_url.clone())?);
    Ok(client)
}

#[cfg(test)]
mod tests {
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
    fn extra_audiences_are_trusted_only_when_listed() {
        let allow = vec!["proj-123".to_string()];
        // Zitadel's extra project-ID audience is accepted when configured.
        assert!(audience_is_trusted(&allow, "proj-123"));
        // Anything else is rejected.
        assert!(!audience_is_trusted(&allow, "proj-999"));
        // With nothing configured, no extra audience is trusted (strict default).
        assert!(!audience_is_trusted(&[], "proj-123"));
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
}
