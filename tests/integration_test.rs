use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

fn relay_cmd(config_path: &str) -> Command {
    let mut cmd = Command::cargo_bin("relay").unwrap();
    cmd.env("RELAY_CONFIG", config_path);
    cmd
}

#[test]
fn test_full_workflow() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("config.yaml");
    let config_str = config_path.to_str().unwrap();

    // Build mock server first
    Command::new("cargo")
        .args(["build", "--bin", "mock-server"])
        .assert()
        .success();

    let mock_server_path = std::env::current_dir()
        .unwrap()
        .join("target/debug/mock-server");

    // Add mock server
    relay_cmd(config_str)
        .args([
            "add",
            "mock",
            "--transport",
            "stdio",
            "--cmd",
            mock_server_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    // List servers
    relay_cmd(config_str)
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("mock"));

    // Ping server
    relay_cmd(config_str)
        .args(["ping", "mock"])
        .assert()
        .success()
        .stdout(predicate::str::contains("mock-server"));

    // List tools
    relay_cmd(config_str)
        .args(["tools", "mock"])
        .assert()
        .success()
        .stdout(predicate::str::contains("echo"));

    // Describe tool
    relay_cmd(config_str)
        .args(["describe", "echo", "--server", "mock"])
        .assert()
        .success()
        .stdout(predicate::str::contains("message"));

    // Run tool with flags
    relay_cmd(config_str)
        .args([
            "run",
            "echo",
            "--server",
            "mock",
            "--message",
            "Hello World",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Echo: Hello World"));

    // Run tool with JSON input
    relay_cmd(config_str)
        .args([
            "run",
            "echo",
            "--server",
            "mock",
            "--input-json",
            r#"{"message": "JSON test"}"#,
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Echo: JSON test"));

    // Run with --format json output
    relay_cmd(config_str)
        .args([
            "--format",
            "json",
            "run",
            "echo",
            "--server",
            "mock",
            "--message",
            "Test",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""type": "text""#));
}
