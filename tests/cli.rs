use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn help_succeeds() {
    let mut command = Command::cargo_bin("arxiv2md").expect("binary exists");
    command.arg("--help").assert().success();
}

#[test]
fn invalid_id_fails() {
    let mut command = Command::cargo_bin("arxiv2md").expect("binary exists");
    command
        .arg("not-an-arxiv-id")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid arXiv identifier or URL"));
}
