//! Maven 扫描候选路径:平台标准 + SDKMAN/手动 + `~/.fnva/packages/maven` + config/env/命令行 extra。

use crate::infrastructure::config::Config;

/// 合并所有来源的候选扫描路径。`extra` 通常来自命令行 `--path`。
pub fn common_paths(extra: &[String]) -> Vec<String> {
    let mut paths = Vec::new();

    if cfg!(target_os = "windows") {
        paths.extend_from_slice(&[
            r"C:\Program Files\Apache\maven".into(),
            r"C:\Program Files\Apache Software Foundation\maven".into(),
        ]);
    } else if cfg!(target_os = "macos") {
        paths.extend_from_slice(&[
            "/opt/homebrew/Cellar/maven".into(),
            "/usr/local/Cellar/maven".into(),
            "/Library/Maven".into(),
        ]);
    } else {
        paths.extend_from_slice(&[
            "/usr/share/maven".into(),
            "/opt/maven".into(),
            "/opt/apache-maven".into(),
        ]);
    }

    // SDKMAN / 手动安装(跨平台,都在用户主目录)
    if let Some(home) = dirs::home_dir() {
        paths.push(
            home.join(".sdkman")
                .join("candidates")
                .join("maven")
                .to_string_lossy()
                .into_owned(),
        );
        paths.push(home.join(".maven").to_string_lossy().into_owned());
        paths.push(
            home.join(".fnva")
                .join("packages")
                .join("maven")
                .to_string_lossy()
                .into_owned(),
        );
    }

    // 持久自定义(config)
    if let Ok(config) = Config::load() {
        for p in &config.custom_maven_scan_paths {
            if !p.trim().is_empty() {
                paths.push(p.trim().into());
            }
        }
    }

    // 临时自定义(env)
    if let Ok(env_paths) = std::env::var("FNVA_MAVEN_SCAN_PATHS") {
        for p in env_paths.split(';') {
            if !p.trim().is_empty() {
                paths.push(p.trim().into());
            }
        }
    }

    // 命令行 extra
    for p in extra {
        if !p.trim().is_empty() {
            paths.push(p.trim().into());
        }
    }

    paths
}
