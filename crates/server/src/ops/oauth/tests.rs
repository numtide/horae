//! The credential generator every Horae credential shares.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;

use super::*;

#[test]
fn each_kind_has_its_own_prefix_and_32_random_bytes() {
    for prefix in [
        ACCESS_TOKEN_PREFIX,
        REFRESH_TOKEN_PREFIX,
        CODE_PREFIX,
        CLIENT_SECRET_PREFIX,
    ] {
        let secret = Secret::generate(prefix);
        let encoded = secret.expose().strip_prefix(prefix).unwrap();
        assert_eq!(URL_SAFE_NO_PAD.decode(encoded).unwrap().len(), 32);
        assert_ne!(secret.expose(), Secret::generate(prefix).expose());
        assert_eq!(secret.digest(), digest(secret.expose()));
    }
}

#[test]
fn debug_shows_only_the_kind_and_display_prefix() {
    let secret = Secret::generate(REFRESH_TOKEN_PREFIX);
    let debug = format!("{secret:?}");
    assert_eq!(
        debug,
        format!("{REFRESH_TOKEN_PREFIX}{}…", secret.display_prefix())
    );
    assert!(!debug.contains(&secret.expose()[REFRESH_TOKEN_PREFIX.len() + 8..]));
}

#[test]
fn client_ids_are_random_and_prefixed() {
    let id = generate_client_id();
    let encoded = id.strip_prefix(CLIENT_ID_PREFIX).unwrap();
    assert_eq!(URL_SAFE_NO_PAD.decode(encoded).unwrap().len(), 16);
    assert_ne!(id, generate_client_id());
}
