//! fnva 数据目录路径集中管理 + 旧布局迁移。
//!
//! 布局(`~/.fnva`):
//! - `config.toml`           用户配置
//! - `state/`                运行时状态(程序生成,可删)
//!   - `current_envs.toml` / `history.toml`
//! - `cache/`                可重建缓存
//!   - `downloads/` / `maven_versions.json` / `java_versions.toml`
//! - `packages/<tool>/<name>/`  安装的工具(持久)

use std::path::PathBuf;

const FNVA_DIR: &str = ".fnva";

fn home() -> Result<PathBuf, String> {
    // FNVA_HOME 优先(主要用于测试隔离到临时目录);未设置时回落到用户主目录。
    if let Ok(custom) = std::env::var("FNVA_HOME") {
        if !custom.is_empty() {
            return Ok(PathBuf::from(custom));
        }
    }
    dirs::home_dir().ok_or_else(|| "Cannot get user home directory".to_string())
}

/// `~/.fnva`
pub fn fnva_dir() -> Result<PathBuf, String> {
    Ok(home()?.join(FNVA_DIR))
}

/// `~/.fnva/config.toml`
pub fn config_path() -> Result<PathBuf, String> {
    Ok(fnva_dir()?.join("config.toml"))
}

// --- state/ ---

/// `~/.fnva/state`
pub fn state_dir() -> Result<PathBuf, String> {
    Ok(fnva_dir()?.join("state"))
}

/// `~/.fnva/state/current_envs.toml`
pub fn current_envs_path() -> Result<PathBuf, String> {
    Ok(state_dir()?.join("current_envs.toml"))
}

/// `~/.fnva/state/history.toml`
pub fn history_path() -> Result<PathBuf, String> {
    Ok(state_dir()?.join("history.toml"))
}

// --- cache/ ---

/// `~/.fnva/cache`
pub fn cache_dir() -> Result<PathBuf, String> {
    Ok(fnva_dir()?.join("cache"))
}

/// `~/.fnva/cache/downloads`
pub fn downloads_dir() -> Result<PathBuf, String> {
    Ok(cache_dir()?.join("downloads"))
}

/// `~/.fnva/cache/maven_versions.json`
pub fn maven_versions_path() -> Result<PathBuf, String> {
    Ok(cache_dir()?.join("maven_versions.json"))
}

/// `~/.fnva/cache/java_versions.toml`
pub fn java_versions_path() -> Result<PathBuf, String> {
    Ok(cache_dir()?.join("java_versions.toml"))
}

// --- packages/ ---

/// `~/.fnva/packages`
pub fn packages_dir() -> Result<PathBuf, String> {
    Ok(fnva_dir()?.join("packages"))
}

/// `~/.fnva/packages/<tool>`
pub fn tool_packages_dir(tool: &str) -> Result<PathBuf, String> {
    Ok(packages_dir()?.join(tool))
}

/// 把旧的扁平布局迁移到分组布局。幂等,任何步骤失败都静默跳过。
///
/// 旧 → 新:
/// - `current_envs.toml` / `history.toml` → `state/` (legacy `session.toml` is merged into current_envs and removed)
/// - `maven_versions.json` → `cache/`
/// - `java-packages/` → `packages/java/`,`maven-packages/` → `packages/maven/`
/// - `current_env`(死代码遗留)→ 删除
/// - `config.toml` 里的安装路径同步更新
pub fn migrate_layout() {
    let Ok(base) = fnva_dir() else {
        return;
    };

    // state/cache/packages must be directories. If a previous bug ever left
    // one of them as a plain file, remove the file before creating the dir.
    ensure_dir(&base.join("state"));
    ensure_dir(&base.join("cache"));
    ensure_dir(&base.join("packages"));

    // session.toml is merged into current_envs.toml; remove any leftovers.
    let _ = std::fs::remove_file(base.join("session.toml"));
    if let Ok(state) = state_dir() {
        let _ = std::fs::remove_file(state.join("session.toml"));
    }
    move_item(&base.join("current_envs.toml"), &current_envs_path().ok());
    move_item(&base.join("history.toml"), &history_path().ok());
    move_item(
        &base.join("maven_versions.json"),
        &maven_versions_path().ok(),
    );

    move_item(&base.join("java-packages"), &tool_packages_dir("java").ok());
    move_item(
        &base.join("maven-packages"),
        &tool_packages_dir("maven").ok(),
    );

    // 删除死代码遗留的 current_env 单文件
    let _ = std::fs::remove_file(base.join("current_env"));

    update_config_paths();
    clean_aliyun_mirror();
    clean_downloading();
}

/// Remove the deprecated `aliyun` entry from `[[mirrors.java]]` blocks.
fn clean_aliyun_mirror() {
    let Ok(path) = config_path() else {
        return;
    };
    let Ok(content) = std::fs::read_to_string(&path) else {
        return;
    };
    let mut out = String::new();
    let mut rest = content.as_str();
    while let Some(pos) = rest.find("[[mirrors.java]]") {
        out.push_str(&rest[..pos]);
        let block_start = pos;
        let after = &rest[block_start..];
        let block_end = after[1..]
            .find("\n[")
            .map(|p| block_start + 1 + p)
            .unwrap_or(rest.len());
        let block = &rest[block_start..block_end];
        if !block.contains("name = \"aliyun\"") {
            out.push_str(block);
        }
        rest = &rest[block_end..];
    }
    out.push_str(rest);
    if out != content {
        let _ = std::fs::write(&path, out);
    }
}

/// Remove interrupted download leftovers (`*.downloading`).
fn clean_downloading() {
    let Ok(dir) = downloads_dir() else { return };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("downloading") {
            let _ = std::fs::remove_file(&path);
        }
    }
}

fn ensure_dir(p: &std::path::Path) {
    if p.exists() && !p.is_dir() {
        let _ = std::fs::remove_file(p);
    }
    let _ = std::fs::create_dir_all(p);
}

fn move_item(from: &std::path::Path, to: &Option<PathBuf>) {
    let Some(to) = to else { return };
    if !from.exists() || to.exists() {
        return;
    }
    if let Some(parent) = to.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::rename(from, to);
}

/// 更新 `config.toml` 里残留的旧安装路径。
fn update_config_paths() {
    let Ok(path) = config_path() else {
        return;
    };
    let Ok(content) = std::fs::read_to_string(&path) else {
        return;
    };
    let updated = content
        .replace(".fnva/java-packages/", ".fnva/packages/java/")
        .replace(".fnva/maven-packages/", ".fnva/packages/maven/");
    if updated != content {
        let _ = std::fs::write(&path, updated);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::FnvaHomeGuard;

    #[test]
    fn migrate_layout_moves_flat_files_to_grouped_dirs() {
        let tmp = tempfile::TempDir::new().unwrap();
        let _g = FnvaHomeGuard::new(tmp.path());
        let base = fnva_dir().unwrap();
        std::fs::create_dir_all(&base).unwrap();
        // 旧扁平布局:文件直接放在 ~/.fnva 根
        std::fs::write(base.join("current_envs.toml"), "test = 1\n").unwrap();
        std::fs::write(base.join("history.toml"), "[[history]]\n").unwrap();
        std::fs::write(base.join("maven_versions.json"), "[]").unwrap();

        migrate_layout();

        // 迁移到分组目录
        assert!(current_envs_path().unwrap().exists());
        assert!(history_path().unwrap().exists());
        assert!(maven_versions_path().unwrap().exists());
        // 旧位置文件已移走
        assert!(!base.join("current_envs.toml").exists());
        assert!(!base.join("history.toml").exists());
    }

    #[test]
    fn migrate_layout_is_idempotent() {
        let tmp = tempfile::TempDir::new().unwrap();
        let _g = FnvaHomeGuard::new(tmp.path());
        let base = fnva_dir().unwrap();
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(base.join("history.toml"), "[[history]]\nnew_env = \"x\"\n").unwrap();

        migrate_layout();
        let after_first = std::fs::read_to_string(history_path().unwrap()).unwrap();
        // 再跑:目标已存在,move_item 跳过,内容不变
        migrate_layout();
        let after_second = std::fs::read_to_string(history_path().unwrap()).unwrap();
        assert_eq!(after_first, after_second);
    }
}
