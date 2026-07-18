//! `fnva doctor` —— 环境自检,定位新用户最常见的安装 / 集成问题。
//!
//! 逐项检查配置可读、数据目录可写、shell 检测、shell 集成是否就位、
//! fnva 是否在 PATH,以及(可选)镜像连通性。每项打印 ✓/✗ 并给修复建议,
//! 最后汇总;任一失败则整体返回失败(由调用方转成非零退出码)。

use crate::cli::print;
use crate::infrastructure::config::Config;
use crate::infrastructure::paths;
use crate::infrastructure::shell::platform::detect_shell;
use crate::infrastructure::shell::ShellType;
use std::path::PathBuf;

/// 检查结果计数。
#[derive(Default)]
struct Outcome {
    passed: u32,
    failed: u32,
    skipped: u32,
}

impl Outcome {
    fn pass(&mut self) {
        self.passed += 1;
    }
    fn fail(&mut self) {
        self.failed += 1;
    }
    fn skip(&mut self) {
        self.skipped += 1;
    }
}

/// 运行全部自检。返回 `true` 表示全部通过(或仅有 skipped)。
pub async fn run_doctor(network: bool) -> Result<bool, String> {
    let mut out = Outcome::default();

    println!("{}\n", print::bold("fnva doctor — environment self-check"));

    check_config(&mut out);
    check_data_dir(&mut out);
    check_shell(&mut out);
    check_path(&mut out);
    check_network(network, &mut out).await;

    println!();
    let ok = out.failed == 0;
    if ok {
        print::success(&format!(
            "{} passed, {} skipped — all checks OK",
            out.passed, out.skipped
        ));
    } else {
        print::failure(
            &format!(
                "{} passed, {} failed, {} skipped",
                out.passed, out.failed, out.skipped
            ),
            Some("fnva may still run, but the failing checks above likely cause issues."),
        );
    }
    Ok(ok)
}

fn check_config(out: &mut Outcome) {
    let Some(path) = paths::config_path().ok() else {
        print::failure(
            "Cannot resolve config path",
            Some("Is your home directory accessible?"),
        );
        out.fail();
        return;
    };
    match Config::load() {
        Ok(_) => {
            print::success(&format!("Config readable: {}", path.display()));
            out.pass();
        }
        Err(e) => {
            print::failure(&format!("Config unreadable: {}", path.display()), Some(&e));
            out.fail();
        }
    }
}

fn check_data_dir(out: &mut Outcome) {
    let Some(dir) = paths::fnva_dir().ok() else {
        print::failure(
            "Cannot resolve data directory",
            Some("Is your home directory accessible?"),
        );
        out.fail();
        return;
    };
    // migrate_layout 会幂等地补全 state/cache/packages 子目录。
    paths::migrate_layout();
    let probe = dir.join(".doctor_probe");
    let writable = std::fs::write(&probe, b"probe").is_ok() && std::fs::remove_file(&probe).is_ok();
    if writable {
        print::success(&format!("Data directory writable: {}", dir.display()));
        out.pass();
    } else {
        print::failure(
            &format!("Data directory not writable: {}", dir.display()),
            Some("Check ownership/permissions on your ~/.fnva directory."),
        );
        out.fail();
    }
}

fn check_shell(out: &mut Outcome) {
    let shell = detect_shell();
    print::success(&format!("Shell detected: {shell:?}"));
    out.pass();

    let candidates = shell_rc_candidates(&shell);
    if candidates.is_empty() {
        print::warn(&format!(
            "No rc profile known for {shell:?} — skipping integration check"
        ));
        out.skip();
        return;
    }
    for rc in &candidates {
        if let Ok(content) = std::fs::read_to_string(rc) {
            if content.contains("fnva") {
                print::success(&format!("Shell integration found: {}", rc.display()));
                out.pass();
                return;
            }
        }
    }
    let primary = candidates[0].display();
    print::failure(
        &format!(
            "Shell integration missing (checked {} candidate(s))",
            candidates.len()
        ),
        Some(&format!("Add to {primary}: eval \"$(fnva env)\"")),
    );
    out.fail();
}

/// 返回当前 shell 可能加载的 rc / profile 候选路径(存在多个时逐个检查)。
fn shell_rc_candidates(shell: &ShellType) -> Vec<PathBuf> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };
    match shell {
        ShellType::Bash => vec![home.join(".bashrc"), home.join(".bash_profile")],
        ShellType::Zsh => vec![home.join(".zshrc"), home.join(".zprofile")],
        ShellType::Fish => vec![home.join(".config").join("fish").join("config.fish")],
        ShellType::PowerShell => {
            if cfg!(target_os = "windows") {
                vec![
                    home.join("Documents")
                        .join("PowerShell")
                        .join("Microsoft.PowerShell_profile.ps1"),
                    home.join("Documents")
                        .join("WindowsPowerShell")
                        .join("Microsoft.PowerShell_profile.ps1"),
                ]
            } else {
                vec![home
                    .join(".config")
                    .join("powershell")
                    .join("Microsoft.PowerShell_profile.ps1")]
            }
        }
        ShellType::Cmd | ShellType::Unknown => Vec::new(),
    }
}

fn check_path(out: &mut Outcome) {
    match which::which("fnva") {
        Ok(p) => {
            print::success(&format!("fnva in PATH: {}", p.display()));
            out.pass();
        }
        Err(_) => {
            print::failure(
                "fnva not found in PATH",
                Some("Re-run the installer, or add ~/.fnva/bin to PATH manually."),
            );
            out.fail();
        }
    }
}

async fn check_network(do_check: bool, out: &mut Outcome) {
    if !do_check {
        print::warn("Mirror reachability: skipped (pass --network to check)");
        out.skip();
        return;
    }
    let Some(config) = Config::load().ok() else {
        print::warn("Mirror reachability: skipped (config unreadable)");
        out.skip();
        return;
    };
    let Some(url) = config
        .mirrors
        .maven
        .iter()
        .chain(config.mirrors.java.iter())
        .find(|m| m.enabled)
        .map(|m| m.base_url.clone())
        .filter(|u| !u.is_empty())
    else {
        print::warn("Mirror reachability: no enabled mirror with a base_url configured");
        out.skip();
        return;
    };
    print::action(&format!("Checking mirror: {url}"));
    let Ok(client) = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
    else {
        print::failure(
            "Cannot build HTTP client",
            Some("Unknown TLS/backend error."),
        );
        out.fail();
        return;
    };
    match client.get(&url).send().await {
        Ok(resp) => {
            print::success(&format!(
                "Mirror reachable (HTTP {})",
                resp.status().as_u16()
            ));
            out.pass();
        }
        Err(e) => {
            print::failure(&format!("Mirror unreachable: {url}"), Some(&e.to_string()));
            out.fail();
        }
    }
}
