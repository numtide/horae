use super::{ClientKind, RedirectUri, RedirectUriError, Scope, ScopeSet, UnknownScope, pkce};
use crate::permissions::catalog::Permission;

// ── Scopes ───────────────────────────────────────────────────────────────────

/// The proofs, the properties and the parser reach scopes only through
/// `Scope::ALL`, so a variant left out of it would pass them all unchecked.
/// A new variant stops this match compiling until it is added here and there.
#[test]
fn all_lists_every_scope() {
    for scope in [Scope::TimeRead, Scope::TimeWrite, Scope::CatalogRead] {
        match scope {
            Scope::TimeRead | Scope::TimeWrite | Scope::CatalogRead => {}
        }
        assert!(Scope::ALL.contains(&scope), "{scope:?}");
    }
}

#[test]
fn scope_names_round_trip() {
    for scope in Scope::ALL {
        assert_eq!(Scope::parse(scope.as_str()), Some(scope));
    }
    assert_eq!(Scope::parse("Time:Read"), None);
}

#[test]
fn scope_list_parses_space_separated_names_once_each() {
    let set = ScopeSet::parse_list("catalog:read time:read  catalog:read").unwrap();
    assert_eq!(set, ScopeSet::of(&[Scope::TimeRead, Scope::CatalogRead]));
    assert_eq!(set.display(), "time:read catalog:read");
}

#[test]
fn empty_scope_list_is_the_empty_set() {
    assert_eq!(ScopeSet::parse_list(""), Ok(ScopeSet::EMPTY));
    assert!(ScopeSet::EMPTY.is_empty());
    assert_eq!(ScopeSet::EMPTY.display(), "");
}

#[test]
fn unknown_scope_is_refused_by_name() {
    assert_eq!(
        ScopeSet::parse_list("time:read admin"),
        Err(UnknownScope("admin".into()))
    );
    assert_eq!(
        ScopeSet::parse_each(["time:read", "time:read:all"]),
        Err(UnknownScope("time:read:all".into()))
    );
}

#[test]
fn write_scope_normalizes_to_include_read() {
    let write = ScopeSet::of(&[Scope::TimeWrite]);
    assert_eq!(
        write.normalize(),
        ScopeSet::of(&[Scope::TimeRead, Scope::TimeWrite])
    );
    let catalog = ScopeSet::of(&[Scope::CatalogRead]);
    assert_eq!(catalog.normalize(), catalog);
}

#[test]
fn subset_compares_scope_sets() {
    let read = ScopeSet::of(&[Scope::TimeRead]);
    let both = ScopeSet::of(&[Scope::TimeRead, Scope::CatalogRead]);
    assert!(read.is_subset(both));
    assert!(!both.is_subset(read));
    assert!(ScopeSet::EMPTY.is_subset(read));
}

/// Each scope's ceiling is exactly research R-6's table: an allow-list, so a
/// permission added to the catalog stays outside every scope until someone
/// puts it in this table.
#[test]
fn each_ceiling_is_the_research_table() {
    use Permission::*;
    let time_read = [TimeReadOwn, TimeReadManaged, TimeReadAll];
    let time_write = [
        TimeReadOwn,
        TimeWriteOwn,
        TimeReadManaged,
        TimeWriteManaged,
        TimeReadAll,
        TimeWriteAll,
    ];
    let catalog_read = [
        ProjectReadManaged,
        ProjectReadAll,
        ClientReadAll,
        TaskReadAll,
        PeopleReadManaged,
        PeopleReadAll,
    ];
    for (scope, expected) in [
        (Scope::TimeRead, &time_read[..]),
        (Scope::TimeWrite, &time_write[..]),
        (Scope::CatalogRead, &catalog_read[..]),
    ] {
        let mut ceiling: Vec<_> = ScopeSet::of(&[scope]).ceiling().iter().collect();
        let mut expected = expected.to_vec();
        ceiling.sort();
        expected.sort();
        assert_eq!(ceiling, expected, "{scope:?}");
    }
    let union: Vec<_> = time_write.iter().chain(&catalog_read).copied().collect();
    for permission in ScopeSet::of(&Scope::ALL).ceiling().iter() {
        assert!(union.contains(&permission), "{permission:?}");
    }
}

#[test]
fn empty_scope_set_lets_nothing_through() {
    assert_eq!(ScopeSet::EMPTY.ceiling().iter().count(), 0);
}

// ── Redirect URIs ────────────────────────────────────────────────────────────

fn parse(uri: &str, kind: ClientKind) -> Result<RedirectUri, RedirectUriError> {
    RedirectUri::parse(uri, kind)
}

#[test]
fn accepted_redirect_uris() {
    use ClientKind::*;
    for (uri, kind) in [
        ("https://app.example.com/callback", Public),
        ("https://app.example.com/callback?x=1", Confidential),
        ("http://127.0.0.1/callback", Public),
        ("http://127.0.0.1:8123/callback", Public),
        ("http://127.0.0.1/", Public),
        ("org.example:/callback", Public),
        ("http://[::1]/callback", Public),
        ("http://[::1]:8123/callback", Public),
        ("com.example.app:/oauth/callback", Public),
        ("dev.horae.android:/oauth/callback", Public),
    ] {
        assert_eq!(
            parse(uri, kind).map(|ok| ok.as_str().to_owned()),
            Ok(uri.to_owned()),
            "{uri}"
        );
    }
}

#[test]
fn refused_redirect_uris() {
    use ClientKind::*;
    use RedirectUriError::*;
    for (uri, kind, error) in [
        ("https://app.example.com/callback#done", Public, Fragment),
        (
            "https://user:pass@app.example.com/callback",
            Public,
            UserInfo,
        ),
        ("https://user@app.example.com/callback", Public, UserInfo),
        ("https://*.example.com/callback", Public, Wildcard),
        ("https://app.example.com/*", Public, Wildcard),
        ("http://localhost:8080/callback", Public, Localhost),
        ("https://localhost/callback", Public, Localhost),
        ("http://app.example.com/callback", Public, HttpNotLoopback),
        ("http://10.0.0.1/callback", Public, HttpNotLoopback),
        ("http://127.0.0.2/callback", Public, HttpNotLoopback),
        ("https://localhost./callback", Public, Localhost),
        ("https://sub.localhost/callback", Public, Localhost),
        ("https://App.LocalHost./callback", Confidential, Localhost),
        ("http://127.0.0.1:0/callback", Public, PortZero),
        ("http://[::1]:0/callback", Public, PortZero),
        ("https://app.example.com:0/callback", Public, PortZero),
        ("myapp:/callback", Public, Scheme),
        ("a.:/callback", Public, Scheme),
        (".a:/callback", Public, Invalid),
        ("a..b:/callback", Public, Scheme),
        (" https://example.com/cb", Public, Whitespace),
        ("https://exa\tmple.com/cb", Public, Whitespace),
        ("com.example.app:/cb\n", Public, Whitespace),
        ("https://example.com/cb ", Public, Whitespace),
        ("https://example.com/c\u{7}b", Public, Whitespace),
        (
            "https:/example.com/cb",
            Public,
            NotCanonical("https://example.com/cb".into()),
        ),
        (
            "https:example.com/cb",
            Public,
            NotCanonical("https://example.com/cb".into()),
        ),
        (
            "http://127.1/callback",
            Public,
            NotCanonical("http://127.0.0.1/callback".into()),
        ),
        (
            "http://127.0.0.1",
            Public,
            NotCanonical("http://127.0.0.1/".into()),
        ),
        (
            "https://APP.example.com/callback",
            Public,
            NotCanonical("https://app.example.com/callback".into()),
        ),
        ("javascript:alert(1)", Public, Scheme),
        ("ftp://app.example.com/", Public, Scheme),
        ("/callback", Public, Invalid),
        ("", Public, Invalid),
        ("http://127.0.0.1:8123/callback", Confidential, PublicOnly),
        ("http://[::1]/callback", Confidential, PublicOnly),
        ("com.example.app:/oauth/callback", Confidential, PublicOnly),
    ] {
        assert_eq!(parse(uri, kind), Err(error), "{uri}");
    }
}

#[test]
fn https_uri_matches_only_itself() {
    let registered = parse("https://app.example.com/callback", ClientKind::Public).unwrap();
    assert!(registered.matches("https://app.example.com/callback"));
    for requested in [
        "https://app.example.com/callback/",
        "https://app.example.com/callback?x=1",
        "https://app.example.com/Callback",
        "https://APP.example.com/callback",
        "https://app.example.com:443/callback",
        "https://app.example.com/call",
        "https://app.example.com/callbackx",
        "http://app.example.com/callback",
    ] {
        assert!(!registered.matches(requested), "{requested}");
    }
}

#[test]
fn loopback_uri_matches_any_port() {
    for (uri, address) in [
        ("http://127.0.0.1/callback", "127.0.0.1"),
        ("http://127.0.0.1:8000/callback", "127.0.0.1"),
        ("http://[::1]/callback", "[::1]"),
    ] {
        let registered = parse(uri, ClientKind::Public).unwrap();
        for requested in [
            format!("http://{address}/callback"),
            format!("http://{address}:1/callback"),
            format!("http://{address}:51234/callback"),
        ] {
            assert!(registered.matches(&requested), "{uri} ~ {requested}");
        }
        for requested in [
            format!("http://{address}:51234/Callback"),
            format!("http://{address}:51234/callback?x=1"),
            format!("http://{address}:51234/callback/"),
            format!("http://{address}:51234@evil.example/callback"),
            format!("http://{address}.evil.example/callback"),
            format!("http://{address}:123456/callback"),
            format!("http://{address}:65536/callback"),
            format!("http://{address}:0/callback"),
            format!("http://{address}:00080/callback"),
            format!("http://{address}:/callback"),
            format!("http://{address}:8080/call\tback"),
            format!(" http://{address}:8080/callback"),
            format!("https://{address}/callback"),
            "http://localhost/callback".to_owned(),
        ] {
            assert!(!registered.matches(&requested), "{uri} !~ {requested}");
        }
    }
}

#[test]
fn loopback_addresses_do_not_match_each_other() {
    let registered = parse("http://127.0.0.1/callback", ClientKind::Public).unwrap();
    assert!(!registered.matches("http://[::1]/callback"));
}

#[test]
fn app_scheme_uri_matches_only_itself() {
    let registered = parse("com.example.app:/oauth/callback", ClientKind::Public).unwrap();
    assert!(registered.matches("com.example.app:/oauth/callback"));
    assert!(!registered.matches("com.example.app:/oauth/callback2"));
    assert!(!registered.matches("com.example.App:/oauth/callback"));
    assert!(!registered.matches("com.example.app:/oauth/callback?x=1"));
}

// ── PKCE ─────────────────────────────────────────────────────────────────────

/// RFC 7636 Appendix B.
const RFC_VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
const RFC_CHALLENGE: &str = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";

#[test]
fn pkce_accepts_the_rfc_example() {
    assert!(pkce::verify(RFC_VERIFIER, RFC_CHALLENGE));
}

#[test]
fn pkce_refuses_another_verifier() {
    let other = format!("{}x", &RFC_VERIFIER[..42]);
    assert!(!pkce::verify(&other, RFC_CHALLENGE));
}

/// `verify` knows only S256; refusing `code_challenge_method=plain` is the
/// authorization endpoint's check (PR C, T023).
#[test]
fn verifier_is_not_its_own_challenge() {
    assert!(!pkce::verify(RFC_VERIFIER, RFC_VERIFIER));
}

#[test]
fn pkce_bounds_the_verifier_length() {
    use base64::Engine;
    use sha2::Digest;
    let challenge_of = |verifier: &str| {
        base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(sha2::Sha256::digest(verifier.as_bytes()))
    };
    for (length, accepted) in [(42, false), (43, true), (128, true), (129, false)] {
        let verifier = "a".repeat(length);
        assert_eq!(
            pkce::verify(&verifier, &challenge_of(&verifier)),
            accepted,
            "{length}"
        );
    }
}

#[test]
fn pkce_refuses_characters_outside_the_unreserved_set() {
    use base64::Engine;
    use sha2::Digest;
    for bad in ['+', '/', '=', ' ', '%', 'é'] {
        let verifier = format!("{}{bad}", "a".repeat(50));
        let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(sha2::Sha256::digest(verifier.as_bytes()));
        assert!(!pkce::verify(&verifier, &challenge), "{bad:?}");
    }
    let unreserved = format!("{}-._~", "aZ09".repeat(10));
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(sha2::Sha256::digest(unreserved.as_bytes()));
    assert!(pkce::verify(&unreserved, &challenge));
}
