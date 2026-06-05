use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn erpnext_cmd() -> Command {
    Command::cargo_bin("erpnext").unwrap()
}

fn with_config_file(contents: &str) -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("config.toml");
    fs::write(&config_path, contents).unwrap();
    (dir, config_path)
}

#[test]
fn help_shows_usage() {
    erpnext_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "CLI tool for interacting with ERPNext",
        ));
}

#[test]
fn config_show_displays_saved_values() {
    let (_dir, config_path) = with_config_file(
        r#"
url = "https://erp.example.com"
auth_type = "token"
api_key = "abc123"
api_secret = "secret456"
timeout_secs = 30
"#,
    );

    erpnext_cmd()
        .env("ERPNEXT_CONFIG_FILE", &config_path)
        .args(["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("https://erp.example.com"))
        .stdout(predicate::str::contains("token"))
        .stdout(predicate::str::contains("abc1"))
        .stdout(predicate::str::contains("(masked)"));
}

#[test]
fn config_set_url_persists_to_file() {
    let (_dir, config_path) = with_config_file("");

    erpnext_cmd()
        .env("ERPNEXT_CONFIG_FILE", &config_path)
        .args(["config", "set-url", "https://saved.example.com"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "URL set to: https://saved.example.com",
        ));

    let saved = fs::read_to_string(config_path).unwrap();
    assert!(saved.contains("https://saved.example.com"));
}

#[test]
fn whoami_fails_without_credentials() {
    let (_dir, config_path) = with_config_file(
        r#"
url = "https://erp.example.com"
auth_type = "token"
"#,
    );

    erpnext_cmd()
        .env("ERPNEXT_CONFIG_FILE", &config_path)
        .arg("whoami")
        .assert()
        .failure()
        .stderr(predicate::str::contains("configuration is incomplete"));
}

#[test]
fn doctype_list_accepts_spaced_doctype_name() {
    let server = httpmock::MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(httpmock::Method::GET)
            .path("/api/resource/Sales%20Invoice")
            .query_param("fields", "name")
            .header("Authorization", "token test-key:test-secret");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"data": [{"name": "SINV-0001"}]}"#);
    });

    let (_dir, config_path) = with_config_file(&format!(
        r#"
url = "{}"
auth_type = "token"
api_key = "test-key"
api_secret = "test-secret"
timeout_secs = 10
"#,
        server.url("")
    ));

    erpnext_cmd()
        .env("ERPNEXT_CONFIG_FILE", &config_path)
        .args(["doctype", "list", "Sales Invoice", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("SINV-0001"));

    mock.assert();
}
