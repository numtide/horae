//! The shared, server-rendered `/auth/login` page.
//!
//! `/auth/login` lives outside the Dioxus SPA (see `route.rs`), so it cannot use
//! the app's utility CSS — the markup and palette here are self-contained and
//! mirror the "Horae Login" design. Only the call-to-action differs: a one-click
//! "Sign in as Admin" in dev mode, or a single "Continue with SSO" hand-off to
//! the OIDC provider in production. Horae has no passwords, no multi-provider
//! picker, and no signup form (accounts are admin-created, or created at first
//! sign-in when the operator allows the email's domain), so the design's
//! email/password and provider-list controls are intentionally absent — every
//! control on this page maps to a real action.

use dioxus::prelude::{asset, manganis};
use serde::{Deserialize, Serialize};

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

impl LoginNotice {
    fn message(&self) -> String {
        match self {
            Self::NoAccount { email: Some(email) } => format!(
                "No Horae account exists for {}. Ask an administrator to add you.",
                escape_html(email)
            ),
            Self::NoAccount { email: None } => {
                "No Horae account matches this sign-in. Ask an administrator to add you.".into()
            }
            Self::Deactivated => {
                "Your Horae account is deactivated. Ask an administrator to restore access.".into()
            }
            Self::DomainNotAllowed { email } => format!(
                "No Horae account exists for {}, and addresses at {} can't create one. \
                 Ask an administrator to add you.",
                escape_html(email),
                escape_html(email.rsplit_once('@').map_or("", |(_, domain)| domain)),
            ),
            Self::EmailUnconfirmed { email } => format!(
                "No Horae account exists for {}, and your identity provider hasn't confirmed \
                 this address, so Horae can't create one. Ask an administrator to add you.",
                escape_html(email)
            ),
        }
    }
}

/// Which sign-in action the page offers.
pub enum LoginVariant {
    /// `DEV_LOGIN=1`: one-click "Sign in as Admin" (`POST /auth/dev-login`).
    Dev,
    /// Production: a single SSO hand-off to the OIDC provider
    /// (`GET /auth/oidc/start`). The button text is operator-configurable
    /// (`HORAE_OIDC_BUTTON_LABEL`).
    Oidc { cta_label: String },
}

/// Render the full login page for `variant`, with `notice` above the action.
pub fn render(variant: LoginVariant, notice: Option<&LoginNotice>) -> String {
    let notice = notice.map_or_else(String::new, |notice| {
        format!(r#"<p class="notice" role="alert">{}</p>"#, notice.message())
    });
    let action = match variant {
        LoginVariant::Dev => r#"<form class="action" method="POST" action="/auth/dev-login">
          <button type="submit" class="cta">Sign in as Admin</button>
        </form>
        <div class="dev-badge">Dev mode</div>"#
            .to_string(),
        // The label comes from operator config, so escape it before it lands in
        // the page's markup.
        LoginVariant::Oidc { cta_label } => format!(
            r#"<a class="cta" href="/auth/oidc/start">{}</a>"#,
            escape_html(&cta_label)
        ),
    };
    // The same icons the app shell links (app.rs), through the same hashed asset URLs.
    let icons = format!(
        r#"<link rel="icon" href="/favicon.ico" sizes="32x32">
  <link rel="icon" type="image/svg+xml" href="{}">
  <link rel="apple-touch-icon" href="{}">"#,
        asset!("/assets/favicon.svg"),
        asset!("/assets/apple-touch-icon.png"),
    );
    PAGE.replace("<!--ICONS-->", &icons)
        .replace("<!--ACTION-->", &format!("{notice}{action}"))
}

/// Minimal HTML-text escaping for interpolating operator-controlled config into
/// the page. Covers the characters that can break out of element text or an
/// attribute value.
fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

static PAGE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Sign in — Horae</title>
  <!--ICONS-->
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family=Newsreader:ital,opsz,wght@0,6..72,400;0,6..72,500;0,6..72,600;1,6..72,500&family=Instrument+Sans:wght@400;500;600;700&family=IBM+Plex+Mono:wght@400;500&display=swap" rel="stylesheet">
  <style>
    /* This page is served outside the SPA (raw Axum), so it cannot load the
       fingerprinted horae.css. These tokens mirror that file's `:root` palette
       (the comment names the source token) and are defined once here so the
       colors below are not repeated hex-by-hex. */
    :root {
      --ink: #100F0C;          /* horae.css --color-bg */
      --panel: #0d1a16;        /* brand-panel ground */
      --pine: #1F5C4D;         /* horae.css --color-pine */
      --pine-ground: #164034;
      --pine-line: #0F3D33;
      --gold: #D99A3C;         /* horae.css --color-accent */
      --gold-tick: #E7C079;
      --mint: #4FB79A;         /* horae.css --color-primary */
      --mint-hover: #6ecab0;
      --text: #EFEAE0;         /* horae.css --color-text */
      --heading: #F6F2E9;
      --muted: #A29C8D;        /* horae.css --color-text-secondary */
      --sage: #8faa9f;
      --border: #322E26;       /* horae.css --color-border */
    }
    *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      min-height: 100vh; display: flex;
      background: var(--ink); color: var(--text);
      font-family: 'Instrument Sans', system-ui, sans-serif;
    }
    a { color: var(--mint); text-decoration: none; }
    a:hover { color: var(--mint-hover); }

    /* LEFT — the sign-in panel */
    .left {
      flex: 1; min-width: 0; display: flex; flex-direction: column;
      align-items: center; justify-content: center; padding: 48px 32px;
    }
    .panel { width: 100%; max-width: 400px; }

    .wordmark { display: flex; align-items: center; gap: 12px; margin-bottom: 40px; }
    .mark {
      position: relative; width: 40px; height: 40px; flex: 0 0 40px;
      border-radius: 10px; background: var(--pine); overflow: hidden;
    }
    .mark-hand {
      position: absolute; left: 50%; top: 6px; width: 1.5px; height: 7px;
      background: var(--gold-tick); transform: translateX(-50%);
    }
    .mark-sun {
      position: absolute; left: 50%; bottom: 11px; width: 20px; height: 20px;
      border-radius: 50%; background: var(--gold); transform: translateX(-50%);
    }
    .mark-ground { position: absolute; left: 0; right: 0; bottom: 0; height: 12px; background: var(--pine-ground); }
    .mark-horizon { position: absolute; left: 0; right: 0; bottom: 12px; height: 1px; background: var(--pine-line); }
    .brand { display: flex; align-items: baseline; }
    .brand-name {
      font-family: 'Newsreader', Georgia, serif; font-size: 28px; font-weight: 600;
      letter-spacing: -0.01em; color: var(--heading);
    }
    .brand-dot {
      width: 5px; height: 5px; border-radius: 50%; background: var(--gold);
      margin-left: 4px; align-self: flex-end; margin-bottom: 6px;
    }

    .title {
      font-family: 'Newsreader', Georgia, serif; font-size: 34px; font-weight: 600;
      letter-spacing: -0.01em; color: var(--heading); margin-bottom: 8px;
    }
    .subtitle { font-size: 15px; color: var(--muted); margin-bottom: 32px; }

    .action { margin: 0; }
    button.cta { font-family: inherit; border: none; cursor: pointer; }
    .cta {
      display: block; width: 100%; text-align: center; padding: 13px 16px;
      background: var(--pine); color: var(--heading); border-radius: 9px;
      font-size: 15px; font-weight: 600; text-decoration: none;
      box-shadow: 0 8px 24px -10px rgba(31, 92, 77, 0.6);
      transition: filter 0.14s ease;
    }
    .cta:hover { filter: brightness(1.1); color: var(--heading); }

    .notice {
      margin-bottom: 20px; padding: 12px 14px; border-radius: 9px;
      font-size: 14px; line-height: 1.5; color: var(--text);
      background: rgba(217, 154, 60, 0.12); border: 1px solid rgba(217, 154, 60, 0.35);
      overflow-wrap: anywhere;
    }

    .dev-badge {
      display: inline-block; margin-top: 20px; padding: 4px 10px;
      border-radius: 9999px; font-size: 11px; font-weight: 600; letter-spacing: 0.05em;
      text-transform: uppercase; background: rgba(79, 183, 154, 0.14);
      color: var(--mint); border: 1px solid rgba(79, 183, 154, 0.3);
    }

    /* RIGHT — the brand panel */
    .right {
      flex: 0 0 46%; position: relative; overflow: hidden;
      background: var(--panel); border-left: 1px solid var(--border);
      display: flex; flex-direction: column; justify-content: center; padding: 64px;
    }
    .glow {
      position: absolute; top: -80px; right: -80px; width: 320px; height: 320px;
      border-radius: 50%;
      background: radial-gradient(circle, rgba(217, 154, 60, 0.18), transparent 70%);
    }
    .brand-copy { position: relative; }
    .eyebrow {
      font-family: 'IBM Plex Mono', monospace; font-size: 12px; letter-spacing: 0.2em;
      text-transform: uppercase; color: var(--mint); margin-bottom: 20px;
    }
    .headline {
      font-family: 'Newsreader', Georgia, serif; font-size: 38px; line-height: 1.25;
      color: var(--heading); max-width: 420px;
    }
    .tagline {
      font-family: 'Newsreader', Georgia, serif; font-style: italic; font-size: 19px;
      line-height: 1.5; color: var(--sage); margin-top: 18px; max-width: 400px;
    }

    /* On narrow viewports the brand panel would crowd the form — drop it. */
    @media (max-width: 820px) { .right { display: none; } }
  </style>
</head>
<body>
  <div class="left">
    <div class="panel">
      <div class="wordmark">
        <div class="mark">
          <div class="mark-hand"></div>
          <div class="mark-sun"></div>
          <div class="mark-ground"></div>
          <div class="mark-horizon"></div>
        </div>
        <div class="brand"><span class="brand-name">Horae</span><span class="brand-dot"></span></div>
      </div>

      <h1 class="title">Welcome back</h1>
      <p class="subtitle">Sign in to your Horae workspace.</p>

      <!--ACTION-->
    </div>
  </div>

  <div class="right">
    <div class="glow"></div>
    <div class="brand-copy">
      <div class="eyebrow">Open-source time tracking</div>
      <div class="headline">Time, turned into paper.</div>
      <div class="tagline">Track hours, review, and invoice — one calm, editorial workspace for the whole team.</div>
    </div>
  </div>
</body>
</html>"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn oidc() -> LoginVariant {
        LoginVariant::Oidc {
            cta_label: "Continue with SSO".into(),
        }
    }

    #[test]
    fn page_without_a_notice_has_no_alert() {
        assert!(!render(oidc(), None).contains(r#"role="alert""#));
    }

    #[test]
    fn no_account_notice_names_the_persons_email() {
        let html = render(
            oidc(),
            Some(&LoginNotice::NoAccount {
                email: Some("ada@example.com".into()),
            }),
        );
        assert!(html.contains("No Horae account exists for ada@example.com."));
    }

    #[test]
    fn no_account_notice_without_email_names_no_address() {
        let html = render(oidc(), Some(&LoginNotice::NoAccount { email: None }));
        assert!(html.contains("No Horae account matches this sign-in."));
    }

    #[test]
    fn deactivated_notice_says_the_account_is_deactivated() {
        let html = render(oidc(), Some(&LoginNotice::Deactivated));
        assert!(html.contains("Your Horae account is deactivated."));
    }

    #[test]
    fn domain_notice_names_the_emails_domain() {
        let html = render(
            oidc(),
            Some(&LoginNotice::DomainNotAllowed {
                email: "ada@other.example".into(),
            }),
        );
        assert!(html.contains("addresses at other.example can't create one"));
    }

    #[test]
    fn unconfirmed_notice_says_the_provider_did_not_confirm() {
        let html = render(
            oidc(),
            Some(&LoginNotice::EmailUnconfirmed {
                email: "ada@example.com".into(),
            }),
        );
        assert!(html.contains("your identity provider hasn't confirmed this address"));
    }

    #[test]
    fn provider_sent_email_is_html_escaped_in_the_notice() {
        let html = render(
            oidc(),
            Some(&LoginNotice::DomainNotAllowed {
                email: "<b>x</b>@<i>y</i>".into(),
            }),
        );
        assert!(html.contains("&lt;b&gt;x&lt;/b&gt;@&lt;i&gt;y&lt;/i&gt;"));
        assert!(!html.contains("<b>x</b>") && !html.contains("<i>y</i>"));
    }

    #[test]
    fn dev_variant_offers_the_admin_bypass_and_no_fake_controls() {
        let html = render(LoginVariant::Dev, None);
        assert!(html.contains(r#"action="/auth/dev-login""#));
        assert!(html.contains("Sign in as Admin"));
        assert!(html.contains("Dev mode"));
        // The design's decorative controls must not appear — every control is real.
        assert!(!html.contains(r#"type="password""#));
        assert!(!html.contains("Continue with Google"));
    }

    #[test]
    fn page_links_the_favicons() {
        let html = render(LoginVariant::Dev, None);
        assert!(html.contains(r#"<link rel="icon" href="/favicon.ico" sizes="32x32">"#));
        assert!(html.contains(r#"<link rel="icon" type="image/svg+xml" href="/"#));
        assert!(html.contains(r#"<link rel="apple-touch-icon" href="/"#));
        assert!(!html.contains("<!--ICONS-->"));
    }

    #[test]
    fn oidc_variant_uses_the_configured_label_and_hands_off_to_start() {
        let html = render(
            LoginVariant::Oidc {
                cta_label: "Continue with Okta".into(),
            },
            None,
        );
        assert!(html.contains(r#"href="/auth/oidc/start""#));
        assert!(html.contains("Continue with Okta"));
        assert!(!html.contains(r#"action="/auth/dev-login""#));
    }

    #[test]
    fn configured_label_is_html_escaped() {
        let html = render(
            LoginVariant::Oidc {
                cta_label: r#"<b>x</b>&"#.into(),
            },
            None,
        );
        assert!(html.contains("&lt;b&gt;x&lt;/b&gt;&amp;"));
        assert!(!html.contains("<b>x</b>"));
    }
}
