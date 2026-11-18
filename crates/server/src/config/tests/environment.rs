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
fn job_attempt_limit_uses_the_documented_environment_variable() {
    for value in ["1", "3", "100"] {
        probe("job-policy", &[("HORAE_JOB_MAX_ATTEMPTS", value)]);
    }
}

#[test]
fn invalid_job_attempt_limits_reject_startup() {
    for value in ["", "0", "-1", "101", "2147483648", "three"] {
        probe("invalid-job-policy", &[("HORAE_JOB_MAX_ATTEMPTS", value)]);
    }
}

#[test]
fn mail_environment_requires_a_complete_valid_configuration() {
    probe(
        "invalid-mail",
        &[("HORAE_SENDMAIL_PATH", "/no/such/sendmail")],
    );
    probe("invalid-mail", &[("HORAE_MAIL_FROM", "a@example.test")]);
    probe(
        "invalid-mail",
        &[
            ("HORAE_SENDMAIL_PATH", "sendmail"),
            ("HORAE_MAIL_FROM", "a@example.test"),
        ],
    );
    let executable = std::env::current_exe().unwrap();
    probe(
        "mail",
        &[
            ("HORAE_SENDMAIL_PATH", executable.to_str().unwrap()),
            ("HORAE_MAIL_FROM", "alerts@example.test"),
        ],
    );
}

const KEY_11: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const KEY_22: &str = "2222222222222222222222222222222222222222222222222222222222222222";

fn key_file(contents: &str) -> tempfile::NamedTempFile {
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), contents).unwrap();
    file
}

#[test]
fn the_cursor_key_file_wins_over_the_key_value() {
    // `openssl rand -hex 32 > file` leaves a trailing newline.
    let file = key_file(&format!("{KEY_11}\n"));
    probe(
        "api-key-11",
        &[
            ("HORAE_API_CURSOR_KEY_FILE", file.path().to_str().unwrap()),
            ("HORAE_API_CURSOR_KEY", KEY_22),
        ],
    );
}

#[test]
fn the_cursor_key_value_is_used_without_a_file() {
    probe("api-key-22", &[("HORAE_API_CURSOR_KEY", KEY_22)]);
}

#[test]
fn a_bad_cursor_key_file_refuses_to_start() {
    let malformed = key_file("not a key");
    let short = key_file(&KEY_11[..62]);
    let directory = tempfile::tempdir().unwrap();
    for path in [
        "/no/such/horae-api-cursor-key",
        directory.path().to_str().unwrap(),
        malformed.path().to_str().unwrap(),
        short.path().to_str().unwrap(),
    ] {
        probe(
            "invalid-api",
            &[
                ("HORAE_API_CURSOR_KEY_FILE", path),
                ("HORAE_API_CURSOR_KEY", KEY_22),
                ("HORAE_CONFIG_TEST_VARIABLE", "HORAE_API_CURSOR_KEY_FILE"),
            ],
        );
    }
}

#[test]
fn a_bad_cursor_key_value_refuses_to_start() {
    for value in [
        "xyz",
        &KEY_11[..63],
        &format!("{KEY_11}00"),
        &"é".repeat(32),
    ] {
        probe(
            "invalid-api",
            &[
                ("HORAE_API_CURSOR_KEY", value),
                ("HORAE_CONFIG_TEST_VARIABLE", "HORAE_API_CURSOR_KEY"),
            ],
        );
    }
}

#[test]
fn the_rate_limit_uses_the_documented_variable() {
    probe("api-rate-5-60", &[("HORAE_API_RATE_LIMIT", "5/60")]);
}

#[test]
fn a_bad_rate_limit_refuses_to_start() {
    for value in [
        "", "100", "0/15", "100/0", "a/15", "100/b", "-1/15", "100/15/2",
    ] {
        probe(
            "invalid-api",
            &[
                ("HORAE_API_RATE_LIMIT", value),
                ("HORAE_CONFIG_TEST_VARIABLE", "HORAE_API_RATE_LIMIT"),
            ],
        );
    }
}

#[test]
fn environment_probe() {
    let Ok(mode) = std::env::var("HORAE_CONFIG_TEST") else {
        return;
    };
    let result = AppConfig::from_env();
    if mode == "invalid-api" {
        let variable = std::env::var("HORAE_CONFIG_TEST_VARIABLE").unwrap();
        let error = format!("{:#}", result.unwrap_err());
        assert!(error.contains(&variable), "{error}");
        assert!(!error.contains(KEY_22), "the error repeats a key: {error}");
        return;
    }
    if mode == "invalid-mail" {
        assert!(result.unwrap_err().to_string().contains("HORAE_"));
        return;
    }
    if mode == "invalid-job-policy" {
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("HORAE_JOB_MAX_ATTEMPTS")
        );
        return;
    }
    let config = result.unwrap();
    match mode.as_str() {
        "defaults" | "secure" => {
            assert_eq!(
                serde_json::to_value(&config).unwrap()["job_policy"]["max_attempts"],
                5
            );
            assert!(!config.dev_login);
            assert_eq!(config.secure_cookies, mode == "secure");
            assert!(config.oidc.is_none());
            assert!(config.harvest.client.is_none());
            assert!(config.harvest.redirect_url.is_none());
            assert!(config.harvest.encryption_key_hex.is_none());
            assert!(config.plugin_database_url.is_none());
            assert!(config.mail.is_none());
            assert!(config.api.cursor_key.is_random());
            assert_eq!(config.api.rate_limit, Default::default());
            assert_eq!(config.api.rate_limit.requests, 100);
            assert_eq!(config.api.rate_limit.window.as_secs(), 15);
        }
        "api-key-11" | "api-key-22" => {
            let byte = if mode == "api-key-11" { 0x11 } else { 0x22 };
            assert_eq!(config.api.cursor_key.bytes(), &[byte; 32]);
            assert!(!config.api.cursor_key.is_random());
            let debug = format!("{:?}", config.api);
            assert!(!debug.contains(&format!("{byte}, {byte}")), "{debug}");
        }
        "api-rate-5-60" => {
            assert_eq!(config.api.rate_limit.requests, 5);
            assert_eq!(config.api.rate_limit.window.as_secs(), 60);
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
            let harvest = &config.harvest;
            let client = harvest
                .client
                .as_ref()
                .expect("example supplies the Harvest client");
            assert_eq!(client.client_id, "test-client");
            assert_eq!(client.client_secret, "test-secret");
            assert_eq!(
                harvest.redirect_url.as_deref(),
                Some("https://horae.example.test/auth/harvest/callback")
            );
            assert_eq!(
                harvest.encryption_key_hex.as_ref().map(String::len),
                Some(64)
            );
        }
        "job-policy" => {
            let requested: i32 = std::env::var("HORAE_JOB_MAX_ATTEMPTS")
                .unwrap()
                .parse()
                .unwrap();
            assert_eq!(config.job_policy.max_attempts, requested);
        }
        "mail" => {
            let mail = config.mail.as_ref().unwrap();
            assert_eq!(mail.sender, "alerts@example.test");
            assert!(mail.executable.is_absolute());
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
