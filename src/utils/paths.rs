use std::path::{Path, PathBuf};

/// 路径工具
pub struct PathUtils;

impl PathUtils {
    /// 规范化路径分隔符
    pub fn normalize_separators(path: &str) -> String {
        if cfg!(target_os = "windows") {
            path.replace('/', "\\")
        } else {
            path.replace('\\', "/")
        }
    }

    /// 获取路径的目录部分
    pub fn parent(path: &str) -> Option<String> {
        Path::new(path).parent().and_then(|p| p.to_str()).map(|s| s.to_string())
    }

    /// 获取路径的文件名部分
    pub fn filename(path: &str) -> Option<String> {
        Path::new(path).file_name().and_then(|s| s.to_str()).map(|s| s.to_string())
    }

    /// 获取文件扩展名
    pub fn extension(path: &str) -> Option<String> {
        Path::new(path).extension().and_then(|s| s.to_str()).map(|s| s.to_string())
    }

    /// 获取不含扩展名的文件名
    pub fn filename_without_extension(path: &str) -> Option<String> {
        Path::new(path)
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
    }

    /// 连接路径
    pub fn join(base: &str, path: &str) -> String {
        let base_path = Path::new(base);
        base_path.join(path).to_string_lossy().to_string()
    }

    /// 连接多个路径
    pub fn join_multiple(base: &str, paths: &[&str]) -> String {
        let mut result = Path::new(base).to_path_buf();
        for path in paths {
            result = result.join(path);
        }
        result.to_string_lossy().to_string()
    }

    /// 获取相对路径
    pub fn relative_from(from: &str, to: &str) -> Option<PathBuf> {
        let from_path = Path::new(from).canonicalize().ok()?;
        let to_path = Path::new(to).canonicalize().ok()?;
        pathdiff::diff_paths(from_path, to_path)
    }

    /// 转换为绝对路径
    pub fn absolute(path: &str) -> String {
        let path = Path::new(path);
        if path.is_absolute() {
            path.to_string_lossy().to_string()
        } else if let Ok(current_dir) = std::env::current_dir() {
            current_dir.join(path).to_string_lossy().to_string()
        } else {
            path.to_string_lossy().to_string()
        }
    }

    /// 检查路径是否存在
    pub fn exists(path: &str) -> bool {
        Path::new(path).exists()
    }

    /// 检查路径是否是文件
    pub fn is_file(path: &str) -> bool {
        Path::new(path).is_file()
    }

    /// 检查路径是否是目录
    pub fn is_directory(path: &str) -> bool {
        Path::new(path).is_dir()
    }

    /// 获取路径组件
    pub fn components(path: &str) -> Vec<String> {
        Path::new(path)
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect()
    }

    /// 获取路径的最后一部分
    pub fn last_component(path: &str) -> Option<String> {
        Self::components(path).last().cloned()
    }

    /// 移除路径的最后一个组件
    pub fn pop_component(path: &str) -> String {
        Path::new(path).parent().unwrap_or_else(|| Path::new(".")).to_string_lossy().to_string()
    }

    /// 清理路径（移除 . 和 ..）
    pub fn clean(path: &str) -> String {
        Path::new(path).components().collect::<PathBuf>().to_string_lossy().to_string()
    }

    /// 检查路径是否是子路径
    pub fn is_sub_path(parent: &str, child: &str) -> bool {
        let parent_path = Path::new(parent);
        let child_path = Path::new(child);

        child_path.starts_with(parent_path)
    }

    /// 获取通用前缀
    pub fn common_prefix(path1: &str, path2: &str) -> String {
        let components1 = Self::components(path1);
        let components2 = Self::components(path2);

        let mut common = Vec::new();
        for (c1, c2) in components1.iter().zip(components2.iter()) {
            if c1 == c2 {
                common.push(c1.as_str());
            } else {
                break;
            }
        }

        if common.is_empty() {
            String::new()
        } else {
            common.iter().collect::<std::path::PathBuf>().to_string_lossy().to_string()
        }
    }

    /// 路径转换为适合当前操作系统的格式
    pub fn to_native(path: &str) -> String {
        let path = Path::new(path);
        path.to_string_lossy().to_string()
    }

    /// 确保路径使用正确的大小写（Windows）
    #[cfg(target_os = "windows")]
    pub fn correct_case(path: &str) -> String {
        if let Ok(path_buf) = Path::new(path).canonicalize() {
            path_buf.to_string_lossy().to_string()
        } else {
            path.to_string()
        }
    }

    /// 在非 Windows 系统上，直接返回路径
    #[cfg(not(target_os = "windows"))]
    pub fn correct_case(path: &str) -> String {
        path.to_string()
    }

    /// 获取路径的文件大小（如果是文件）
    pub fn size(path: &str) -> Option<u64> {
        let path = Path::new(path);
        if path.is_file() {
            path.metadata().ok()?.len().into()
        } else {
            None
        }
    }

    /// 计算目录大小（递归）
    pub fn dir_size(path: &str) -> Result<u64, std::io::Error> {
        let path = Path::new(path);
        if !path.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotADirectory,
                "Path is not a directory"
            ));
        }

        let mut total_size = 0u64;
        Self::calculate_dir_size_recursive(path, &mut total_size)?;
        Ok(total_size)
    }

    /// 递归计算目录大小
    fn calculate_dir_size_recursive(dir: &Path, total_size: &mut u64) -> Result<(), std::io::Error> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                *total_size += entry.metadata()?.len();
            } else if path.is_dir() {
                Self::calculate_dir_size_recursive(&path, total_size)?;
            }
        }
        Ok(())
    }

    /// 格式化文件大小为人类可读格式
    pub fn format_size(size: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = size as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", size as u64, UNITS[unit_index])
        } else {
            format!("{:.1} {}", size, UNITS[unit_index])
        }
    }

    /// 创建目录和所有父目录
    pub fn create_dir_all(path: &str) -> Result<(), std::io::Error> {
        std::fs::create_dir_all(path)
    }

    /// 删除空目录
    pub fn remove_dir(path: &str) -> Result<(), std::io::Error> {
        std::fs::remove_dir(path)
    }

    /// 删除目录及其内容
    pub fn remove_dir_all(path: &str) -> Result<(), std::io::Error> {
        std::fs::remove_dir_all(path)
    }

    /// 复制文件
    pub fn copy_file(src: &str, dst: &str) -> Result<(), std::io::Error> {
        let src_path = Path::new(src);
        let dst_path = Path::new(dst);

        // 确保目标目录存在
        if let Some(parent) = dst_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::copy(src_path, dst_path)?;
        Ok(())
    }

    /// 移动文件
    pub fn move_file(src: &str, dst: &str) -> Result<(), std::io::Error> {
        let src_path = Path::new(src);
        let dst_path = Path::new(dst);

        // 确保目标目录存在
        if let Some(parent) = dst_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::rename(src_path, dst_path)?;
        Ok(())
    }

    /// 生成临时文件路径
    pub fn temp_file() -> String {
        let temp_dir = std::env::temp_dir();
        let filename = format!("fnva_temp_{}", uuid::Uuid::new_v4());
        temp_dir.join(filename).to_string_lossy().to_string()
    }

    /// 生成临时目录路径
    pub fn temp_dir() -> String {
        let temp_dir = std::env::temp_dir();
        let dirname = format!("fnva_temp_{}", uuid::Uuid::new_v4());
        temp_dir.join(dirname).to_string_lossy().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_separators() {
        if cfg!(target_os = "windows") {
            assert_eq!(PathUtils::normalize_separators("path/to/file"), "path\\to\\file");
            assert_eq!(PathUtils::normalize_separators("path\\to\\file"), "path\\to\\file");
        } else {
            assert_eq!(PathUtils::normalize_separators("path\\to\\file"), "path/to/file");
            assert_eq!(PathUtils::normalize_separators("path/to/file"), "path/to/file");
        }
    }

    #[test]
    fn test_join() {
        if cfg!(target_os = "windows") {
            // Windows下Path::join会保留Unix路径分隔符作为文件名的一部分
            assert_eq!(PathUtils::join("/base", "sub/file"), "/base\\sub/file");
            // 当基础路径以/结尾时，行为不同
            assert_eq!(PathUtils::join("/base/", "file"), "/base/file");
        } else {
            assert_eq!(PathUtils::join("/base", "sub/file"), "/base/sub/file");
            assert_eq!(PathUtils::join("/base/", "file"), "/base/file");
        }
    }

    #[test]
    fn test_components() {
        if cfg!(target_os = "windows") {
            let components = PathUtils::components(r"C:\path\to\file.txt");
            assert_eq!(components, vec!["C:", "\\", "path", "to", "file.txt"]);
        } else {
            let components = PathUtils::components("/path/to/file.txt");
            assert_eq!(components, vec!["/", "path", "to", "file.txt"]);
        }
    }

    #[test]
    fn test_filename_without_extension() {
        assert_eq!(PathUtils::filename_without_extension("file.txt"), Some("file".to_string()));
        assert_eq!(PathUtils::filename_without_extension("path/to/file.ext"), Some("file".to_string()));
        assert_eq!(PathUtils::filename_without_extension("noext"), Some("noext".to_string()));
    }

    #[test]
    fn test_format_size() {
        assert_eq!(PathUtils::format_size(512), "512 B");
        assert_eq!(PathUtils::format_size(1024), "1.0 KB");
        assert_eq!(PathUtils::format_size(1536), "1.5 KB");
        assert_eq!(PathUtils::format_size(1048576), "1.0 MB");
        assert_eq!(PathUtils::format_size(1073741824), "1.0 GB");
    }

    #[test]
    fn test_common_prefix() {
        if cfg!(target_os = "windows") {
            assert_eq!(PathUtils::common_prefix(r"C:\path\to\file1", r"C:\path\to\file2"), r"C:\path\to");
            // Windows下两个不同的路径可能有共同的根路径
            let result = PathUtils::common_prefix("/different/path", "/another/path");
            // 预期结果可能是"\\"（Windows下的根路径）
            assert!(result.is_empty() || result == "\\" || result == "/");
        } else {
            assert_eq!(PathUtils::common_prefix("/path/to/file1", "/path/to/file2"), "/path/to");
            // 对于没有共同路径的情况，清理结果以避免平台差异
            let result = PathUtils::common_prefix("/different/path", "/another/path");
            assert!(result.is_empty() || result == "/");
        }
    }

    #[test]
    fn test_temp_file() {
        let temp_path = PathUtils::temp_file();
        assert!(temp_path.contains("fnva_temp_"));
        assert!(temp_path.len() > 20);
    }
}