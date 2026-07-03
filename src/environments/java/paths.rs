//! Java 扫描候选路径:平台标准 + `~/.fnva/packages/java` + config 自定义 + env + 命令行 extra。

use crate::infrastructure::config::Config;

/// 合并所有来源的候选扫描路径。`extra` 通常来自命令行 `--path`。
pub fn common_paths(extra: &[String]) -> Vec<String> {
    let mut paths = Vec::new();

    if cfg!(target_os = "windows") {
        paths.extend_from_slice(&[
            r"C:\Program Files\Java".into(),
            r"C:\Program Files (x86)\Java".into(),
            r"C:\Program Files\Eclipse Adoptium".into(),
            r"C:\Program Files\Amazon Corretto".into(),
            r"C:\Program Files\Microsoft\jdk".into(),
            r"C:\Program Files\Zulu".into(),
        ]);
    } else if cfg!(target_os = "macos") {
        paths.extend_from_slice(&[
            "/Library/Java/JavaVirtualMachines".into(),
            "/System/Library/Java/JavaVirtualMachines".into(),
            "/usr/local/java".into(),
            "/opt/homebrew/Caskroom".into(),
        ]);
    } else {
        paths.extend_from_slice(&[
            "/usr/lib/jvm".into(),
            "/usr/lib/jvm/java".into(),
            "/usr/local/java".into(),
            "/opt/java".into(),
            "/usr/java".into(),
        ]);
    }

    if let Some(home) = dirs::home_dir() {
        paths.push(
            home.join(".fnva")
                .join("packages")
                .join("java")
                .to_string_lossy()
                .into_owned(),
        );
    }

    // 持久自定义(config)
    if let Ok(config) = Config::load() {
        for p in &config.custom_java_scan_paths {
            if !p.trim().is_empty() {
                paths.push(p.trim().into());
            }
        }
    }

    // 临时自定义(env;兼容旧的 FNVA_SCAN_PATHS)
    for var in ["FNVA_JAVA_SCAN_PATHS", "FNVA_SCAN_PATHS"] {
        if let Ok(env_paths) = std::env::var(var) {
            for p in env_paths.split(';') {
                if !p.trim().is_empty() {
                    paths.push(p.trim().into());
                }
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
