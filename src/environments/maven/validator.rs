use std::path::Path;

/// mvn 可执行文件名(平台相关)
fn mvn_bin_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "mvn.cmd"
    } else {
        "mvn"
    }
}

/// 验证 Maven HOME 路径是否有效(存在 `bin/mvn`)
pub fn validate_maven_home(maven_home: &str) -> bool {
    let p = Path::new(maven_home);
    if !p.exists() {
        return false;
    }
    p.join("bin").exists() && p.join("bin").join(mvn_bin_name()).exists()
}

/// 给定解压根目录,定位实际 Maven home。
///
/// Maven 包经 `tar --strip-components=1` 解压后,`bin/mvn` 直接位于根目录,
/// 无需像 Java 那样查找 `Contents/Home` 子目录。
pub fn locate_maven_home(install_dir: &Path) -> Result<String, String> {
    if install_dir.join("bin").join(mvn_bin_name()).exists() {
        return Ok(install_dir.to_string_lossy().to_string());
    }
    Err(format!(
        "Maven binary not found under {}",
        install_dir.display()
    ))
}
