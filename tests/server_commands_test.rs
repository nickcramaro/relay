use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

fn relay_cmd(config_path: &str) -> Command {
    let mut cmd = Command::cargo_bin("relay").unwrap();
    cmd.env("RELAY_CONFIG", config_path);
    cmd
}

#[test]
fn test_add_and_list_server() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.yaml");
    let config_str = config_path.to_str().unwrap();

    // Add a server
    relay_cmd(config_str)
        .args([
            "add",
            "test-server",
            "--transport",
            "http",
            "--url",
            "http://localhost:3000",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added server"));

    // List servers
    relay_cmd(config_str)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("test-server"));

    // Remove server
    relay_cmd(config_str)
        .args(["remove", "test-server"])
        .assert()
        .success();

    // Verify removed
    relay_cmd(config_str).arg("list").assert().success().stdout(
        predicate::str::contains("No servers").or(predicate::str::contains("test-server").not()),
    );
}
