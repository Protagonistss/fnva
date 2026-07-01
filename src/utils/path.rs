use std::path::Path;

/// 标准化路径格式，处理反斜杠和大小写问题
pub fn normalize_path(path: &str) -> String {
    let p = Path::new(path);
    match p.canonicalize() {
        Ok(canonical_path) => canonical_path.to_string_lossy().to_string(),
        Err(_) => p.to_string_lossy().replace('\\', "/").to_lowercase(),
    }
}
