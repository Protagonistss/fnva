//! CLI 端到端测试:通过 assert_cmd 跑 fnva 二进制,覆盖命令分发 / 参数 / 退出码 / 输出。
//!
//! 每个测试把 `FNVA_HOME` 指向独立临时目录(子进程级 env),互不干扰,可并行——
//! 不像 lib 测试那样需要全局锁。

use assert_cmd::Command;
use predicates::prelude::*;

fn fnva_cmd() -> Command {
    Command::cargo_bin("fnva").expect("fnva binary should be built")
}

#[test]
fn version_flag_prints_version() {
    fnva_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn java_list_on_empty_config_succeeds() {
    let tmp = tempfile::TempDir::new().unwrap();
    fnva_cmd()
        .env("FNVA_HOME", tmp.path())
        .args(["java", "list"])
        .assert()
        .success();
}

#[test]
fn java_use_nonexistent_fails_with_not_found() {
    let tmp = tempfile::TempDir::new().unwrap();
    fnva_cmd()
        .env("FNVA_HOME", tmp.path())
        .args(["java", "use", "ghost"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found").or(predicate::str::contains("Not found")));
}

#[test]
fn history_json_emits_json_payload() {
    let tmp = tempfile::TempDir::new().unwrap();
    fnva_cmd()
        .env("FNVA_HOME", tmp.path())
        .args(["history", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"history\""));
}

#[test]
fn maven_default_unset_warns_on_stderr() {
    let tmp = tempfile::TempDir::new().unwrap();
    fnva_cmd()
        .env("FNVA_HOME", tmp.path())
        .args(["maven", "default"])
        .assert()
        .success()
        .stderr(predicate::str::contains("No default"));
}

#[test]
fn cc_list_runs_and_shows_default_env() {
    let tmp = tempfile::TempDir::new().unwrap();
    // Config::new 自带一个 anthropic-cc 默认环境,cc list 应能列出它。
    fnva_cmd()
        .env("FNVA_HOME", tmp.path())
        .args(["cc", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("anthropic-cc"));
}

#[test]
fn doctor_runs_and_reports_checks() {
    let tmp = tempfile::TempDir::new().unwrap();
    fnva_cmd()
        .env("FNVA_HOME", tmp.path())
        .arg("doctor")
        .assert()
        .stdout(predicate::str::contains("fnva doctor"));
}
