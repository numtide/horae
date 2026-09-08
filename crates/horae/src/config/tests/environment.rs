use crate::config::{AppConfig, DEFAULT_OIDC_BUTTON_LABEL};

fn probe(mode: &str, environment: &[(&str, &str)]) {
    // A fresh process avoids mutating the environment of concurrent tests.
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "config::tests::environment::environment_probe",
            "--nocapture",
        ])
        .env_clear()
        .envs(environment.iter().copied())
        .env("HORAE_CONFIG_TEST", mode)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
}

#[test]
fn empty_environment_has_no_authentication_or_plugin_credentials() {
    probe("defaults", &[]);
}

#[test]
fn obsolete_authentication_variables_do_not_configure_the_server() {
    probe(
        "defaults",
        &[
            ("SESSION_SECRET", "unused-test-secret"),
            ("SECURE_COOKIES", "1"),
            ("OIDC_ISSUER", "https://id.example.test"),
            ("OIDC_CLIENT_ID", "test-client"),
            ("OIDC_CLIENT_SECRET", "test-secret"),
        ],
    );
}

#[test]
fn example_environment_can_configure_all_required_oidc_and_harvest_fields() {
    let environment: Vec<_> = include_str!("../../../../../.env.example")
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with('#'))
        .map(|line| {
            let (key, value) = line.split_once('=').expect("example uses KEY=value lines");
            let value = match key {
                "HORAE_OIDC_ISSUER" => "https://id.example.test",
                "HORAE_OIDC_CLIENT_ID" | "HORAE_HARVEST_CLIENT_ID" => "test-client",
                "HORAE_OIDC_CLIENT_SECRET" | "HORAE_HARVEST_CLIENT_SECRET" => "test-secret",
                "HORAE_OIDC_REDIRECT_URL" => "https://horae.example.test/auth/callback",
                "HORAE_HARVEST_REDIRECT_URL" => "https://horae.example.test/auth/harvest/callback",
                "HORAE_HARVEST_ENC_KEY" => {
                    "1111111111111111111111111111111111111111111111111111111111111111"
                }
                _ => value,
            };
            (key, value)
        })
        .collect();
    probe("example", &environment);
}

#[test]
fn secure_cookies_use_the_documented_prefixed_variable() {
    for value in ["1", "true", "TRUE"] {
        probe("secure", &[("HORAE_SECURE_COOKIES", value)]);
    }
    for value in ["0", "false", ""] {
        probe("defaults", &[("HORAE_SECURE_COOKIES", value)]);
    }
}

#[test]
fn partial_oidc_configuration_remains_disabled_without_a_callback() {
    probe(
        "defaults",
        &[
            ("HORAE_OIDC_ISSUER", "https://id.example.test"),
            ("HORAE_OIDC_CLIENT_ID", "test-client"),
            ("HORAE_OIDC_CLIENT_SECRET", "test-secret"),
        ],
    );
}

#[test]
fn environment_probe() {
    let Ok(mode) = std::env::var("HORAE_CONFIG_TEST") else {
        return;
    };
    let config = AppConfig::from_env().unwrap();
    match mode.as_str() {
        "defaults" | "secure" => {
            assert!(!config.dev_login);
            assert_eq!(config.secure_cookies, mode == "secure");
            assert!(config.oidc.is_none());
            assert!(config.harvest.is_none());
            assert!(config.plugin_database_url.is_none());
        }
        "example" => {
            assert!(config.dev_login);
            assert!(!config.secure_cookies);
            let oidc = config
                .oidc
                .as_ref()
                .expect("example supplies all four OIDC settings");
            assert_eq!(oidc.issuer, "https://id.example.test");
            assert_eq!(oidc.client_id, "test-client");
            assert_eq!(oidc.client_secret, "test-secret");
            assert_eq!(
                oidc.redirect_url,
                "https://horae.example.test/auth/callback"
            );
            assert!(oidc.additional_audiences.is_empty());
            assert_eq!(oidc.button_label, DEFAULT_OIDC_BUTTON_LABEL);
            let harvest = config
                .harvest
                .as_ref()
                .expect("example supplies all four Harvest settings");
            assert_eq!(harvest.client_id, "test-client");
            assert_eq!(harvest.client_secret, "test-secret");
            assert_eq!(
                harvest.redirect_url,
                "https://horae.example.test/auth/harvest/callback"
            );
            assert_eq!(harvest.encryption_key_hex.len(), 64);
        }
        _ => panic!("unknown configuration test mode"),
    }
    // The config must not advertise an unused cookie-signing key. All values in
    // this process come from the test fixture, never the developer's environment.
    assert!(
        serde_json::to_value(config)
            .unwrap()
            .get("session_secret")
            .is_none()
    );
}
