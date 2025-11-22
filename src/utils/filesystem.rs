use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// 文件系统工具
pub struct FileSystemUtils;

impl FileSystemUtils {
    /// 安全地创建目录
    pub fn create_dir_all(path: &Path) -> Result<(), io::Error> {
        if !path.exists() {
            fs::create_dir_all(path)?;
        }
        Ok(())
    }

    /// 安全地删除文件
    pub fn remove_file(path: &Path) -> Result<(), io::Error> {
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    /// 安全地删除目录及其内容
    pub fn remove_dir_all(path: &Path) -> Result<(), io::Error> {
        if path.exists() {
            fs::remove_dir_all(path)?;
        }
        Ok(())
    }

    /// 安全地复制文件
    pub fn copy_file(src: &Path, dst: &Path) -> Result<(), io::Error> {
        // 确保目标目录存在
        if let Some(parent) = dst.parent() {
            Self::create_dir_all(parent)?;
        }

        fs::copy(src, dst)?;
        Ok(())
    }

    /// 读取文件内容，如果文件不存在则返回 None
    pub fn read_to_string_optional(path: &Path) -> Result<Option<String>, io::Error> {
        if path.exists() {
            let content = fs::read_to_string(path)?;
            Ok(Some(content))
        } else {
            Ok(None)
        }
    }

    /// 写入文件，创建目录如果不存在
    pub fn write_to_string(path: &Path, content: &str) -> Result<(), io::Error> {
        // 确保目录存在
        if let Some(parent) = path.parent() {
            Self::create_dir_all(parent)?;
        }

        fs::write(path, content)?;
        Ok(())
    }

    /// 追加内容到文件
    pub fn append_to_string(path: &Path, content: &str) -> Result<(), io::Error> {
        // 确保目录存在
        if let Some(parent) = path.parent() {
            Self::create_dir_all(parent)?;
        }

        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?
            .write_all(content.as_bytes())?;
        Ok(())
    }

    /// 检查文件是否可读
    pub fn is_readable(path: &Path) -> bool {
        path.exists() && path.is_file()
    }

    /// 检查文件是否可写
    pub fn is_writable(path: &Path) -> bool {
        if path.exists() {
            path.is_file()
        } else {
            // 检查父目录是否可写
            path.parent()
                .map(|p| p.is_dir() && Self::test_writable_dir(p))
                .unwrap_or(false)
        }
    }

    /// 测试目录是否可写
    fn test_writable_dir(dir: &Path) -> bool {
        let test_file = dir.join(".fnva_write_test");
        match fs::write(&test_file, "test") {
            Ok(_) => {
                let _ = fs::remove_file(&test_file);
                true
            }
            Err(_) => false,
        }
    }

    /// 获取文件大小
    pub fn file_size(path: &Path) -> Result<u64, io::Error> {
        Ok(fs::metadata(path)?.len())
    }

    /// 检查路径是否是绝对路径
    pub fn is_absolute_path(path: &str) -> bool {
        Path::new(path).is_absolute()
    }

    /// 规范化路径
    pub fn normalize_path(path: &str) -> PathBuf {
        let path = Path::new(path);
        if let Ok(canonical) = path.canonicalize() {
            canonical
        } else {
            path.to_path_buf()
        }
    }

    /// 获取相对路径
    pub fn relative_path(from: &Path, to: &Path) -> Option<PathBuf> {
        pathdiff::diff_paths(from, to)
    }

    /// 创建临时文件
    pub fn create_temp_file() -> Result<PathBuf, io::Error> {
        use std::env;

        let temp_dir = env::temp_dir();
        let temp_file = temp_dir.join(format!("fnva_temp_{}", uuid::Uuid::new_v4()));

        // 创建空文件
        fs::File::create(&temp_file)?;

        Ok(temp_file)
    }

    /// 创建临时目录
    pub fn create_temp_dir() -> Result<PathBuf, io::Error> {
        use std::env;

        let temp_dir = env::temp_dir();
        let temp_path = temp_dir.join(format!("fnva_temp_{}", uuid::Uuid::new_v4()));

        fs::create_dir_all(&temp_path)?;

        Ok(temp_path)
    }

    /// 查找文件，支持递归搜索
    pub fn find_file(
        start_dir: &Path,
        filename: &str,
        max_depth: usize,
    ) -> Result<Option<PathBuf>, io::Error> {
        Self::find_file_recursive(start_dir, filename, 0, max_depth)
    }

    /// 递归查找文件
    fn find_file_recursive(
        dir: &Path,
        filename: &str,
        current_depth: usize,
        max_depth: usize,
    ) -> Result<Option<PathBuf>, io::Error> {
        if current_depth > max_depth {
            return Ok(None);
        }

        // 检查当前目录
        let file_path = dir.join(filename);
        if file_path.exists() {
            return Ok(Some(file_path));
        }

        // 递归搜索子目录
        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    if let Some(found) =
                        Self::find_file_recursive(&path, filename, current_depth + 1, max_depth)?
                    {
                        return Ok(Some(found));
                    }
                }
            }
        }

        Ok(None)
    }

    /// 获取目录中的所有文件（递归）
    pub fn get_all_files(dir: &Path) -> Result<Vec<PathBuf>, io::Error> {
        let mut files = Vec::new();
        Self::collect_files_recursive(dir, &mut files)?;
        Ok(files)
    }

    /// 递归收集文件
    fn collect_files_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), io::Error> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                files.push(path);
            } else if path.is_dir() {
                Self::collect_files_recursive(&path, files)?;
            }
        }
        Ok(())
    }

    /// 备份文件
    pub fn backup_file(file_path: &Path) -> Result<PathBuf, io::Error> {
        if !file_path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("File not found: {:?}", file_path),
            ));
        }

        let backup_path = Self::generate_backup_path(file_path)?;
        Self::copy_file(file_path, &backup_path)?;

        Ok(backup_path)
    }

    /// 生成备份路径
    fn generate_backup_path(file_path: &Path) -> Result<PathBuf, io::Error> {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let file_stem = file_path
            .file_stem()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid file name"))?;

        let extension = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        let backup_name = if extension.is_empty() {
            format!("{}_backup_{}", file_stem.to_string_lossy(), timestamp)
        } else {
            format!(
                "{}_backup_{}.{}",
                file_stem.to_string_lossy(),
                timestamp,
                extension
            )
        };

        Ok(file_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(backup_name))
    }

    /// 清理临时文件
    pub fn cleanup_temp_files() -> Result<usize, io::Error> {
        use std::env;

        let temp_dir = env::temp_dir();
        let mut cleaned = 0;

        for entry in fs::read_dir(&temp_dir)? {
            let entry = entry?;
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            if file_name_str.starts_with("fnva_temp_") {
                let path = entry.path();
                if path.is_file() {
                    fs::remove_file(&path)?;
                } else {
                    fs::remove_dir_all(&path)?;
                }
                cleaned += 1;
            }
        }

        Ok(cleaned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_is_absolute_path() {
        if cfg!(target_os = "windows") {
            assert!(FileSystemUtils::is_absolute_path("C:\\Program Files\\Java"));
        } else {
            assert!(FileSystemUtils::is_absolute_path("/usr/bin/java"));
        }
        assert!(!FileSystemUtils::is_absolute_path("relative/path"));
        assert!(!FileSystemUtils::is_absolute_path("./relative"));
    }

    #[test]
    fn test_create_temp_file() {
        let temp_file = FileSystemUtils::create_temp_file().unwrap();
        assert!(temp_file.exists());
        assert!(temp_file.is_file());

        // 清理
        let _ = fs::remove_file(&temp_file);
    }

    #[test]
    fn test_backup_file() {
        // 创建测试文件
        let test_file = Path::new("test_backup.txt");
        let content = "test content";
        fs::write(test_file, content).unwrap();

        // 备份文件
        let backup_path = FileSystemUtils::backup_file(test_file).unwrap();
        assert!(backup_path.exists());
        assert!(backup_path.is_file());

        // 验证备份内容
        let backup_content = fs::read_to_string(&backup_path).unwrap();
        assert_eq!(backup_content, content);

        // 清理
        let _ = fs::remove_file(test_file);
        let _ = fs::remove_file(&backup_path);
    }
}
