/// Integration tests against a real ERPNext instance.
///
/// Run with:
///   ERPNEXT_TEST_INSTANCE=1 \
///   ERPNEXT_URL=https://your.instance \
///   ERPNEXT_TOKEN=key:secret \
///   cargo test --test integration -- --ignored
use assert_cmd::Command;
use predicates::prelude::*;
use std::env;

fn should_run_integration() -> bool {
    env::var("ERPNEXT_TEST_INSTANCE").is_ok()
        && env::var("ERPNEXT_URL").is_ok()
        && env::var("ERPNEXT_TOKEN").is_ok()
}

fn erpnext_cmd() -> Command {
    let mut cmd = Command::cargo_bin("erpnext").unwrap();
    cmd.env("ERPNEXT_URL", env::var("ERPNEXT_URL").unwrap())
        .env("ERPNEXT_TOKEN", env::var("ERPNEXT_TOKEN").unwrap())
        .arg("--output")
        .arg("json");
    cmd
}

#[test]
#[ignore = "requires ERPNext instance (set ERPNEXT_TEST_INSTANCE=1)"]
fn test_whoami() {
    if !should_run_integration() {
        eprintln!("Skipping: set ERPNEXT_TEST_INSTANCE, ERPNEXT_URL, and ERPNEXT_TOKEN");
        return;
    }

    erpnext_cmd()
        .arg("whoami")
        .assert()
        .success()
        .stdout(predicate::str::contains("message"));
}

#[test]
#[ignore = "requires ERPNext instance (set ERPNEXT_TEST_INSTANCE=1)"]
fn test_doctype_list_user() {
    if !should_run_integration() {
        eprintln!("Skipping: set ERPNEXT_TEST_INSTANCE, ERPNEXT_URL, and ERPNEXT_TOKEN");
        return;
    }

    erpnext_cmd()
        .args([
            "doctype",
            "list",
            "User",
            "--fields",
            "name,enabled",
            "--limit-page-length",
            "1",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("data"));
}

#[test]
#[ignore = "requires ERPNext instance (set ERPNEXT_TEST_INSTANCE=1)"]
fn test_call_get_logged_user() {
    if !should_run_integration() {
        eprintln!("Skipping: set ERPNEXT_TEST_INSTANCE, ERPNEXT_URL, and ERPNEXT_TOKEN");
        return;
    }

    erpnext_cmd()
        .args(["call", "frappe.auth.get_logged_user"])
        .assert()
        .success()
        .stdout(predicate::str::contains("message"));
}
