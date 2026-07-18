use crate::error::{safe_path_to_str, AppError};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::Path;

pub fn create_progress_bar() -> Result<ProgressBar, AppError> {
    let pb = ProgressBar::new(0);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {bytes:>8}/{total_bytes:<8} ({eta})")
            .map_err(|e| AppError::Internal {
                message: format!("Failed to create progress bar style: {e}")
            })?
            .progress_chars("━╸ ")
    );
    Ok(pb)
}

pub fn extract_zip(zip_path: &Path, dest_dir: &Path) -> Result<(), String> {
    let file = fs::File::open(zip_path).map_err(|e| format!("Failed to open ZIP file: {e}"))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Failed to read ZIP file: {e}"))?;
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read ZIP entry: {e}"))?;
        // enclosed_name 对含 `..` / 绝对路径等不安全条目返回 None,跳过以防 zip-slip。
        let Some(rel) = file.enclosed_name() else {
            continue;
        };
        let outpath = dest_dir.join(rel);
        if file.is_dir() {
            fs::create_dir_all(&outpath).map_err(|e| format!("Failed to create directory: {e}"))?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p)
                        .map_err(|e| format!("Failed to create parent directory: {e}"))?;
                }
            }
            let mut outfile =
                fs::File::create(&outpath).map_err(|e| format!("Failed to create file: {e}"))?;
            std::io::copy(&mut file, &mut outfile)
                .map_err(|e| format!("Failed to write file: {e}"))?;
        }
    }
    Ok(())
}

pub fn extract_tar_gz(tar_path: &Path, dest_dir: &Path) -> Result<(), String> {
    let tar_path_str =
        safe_path_to_str(tar_path).map_err(|e| format!("Path conversion failed: {e}"))?;
    let dest_dir_str =
        safe_path_to_str(dest_dir).map_err(|e| format!("Dest path conversion failed: {e}"))?;

    let output = std::process::Command::new("tar")
        .args([
            "-xzf",
            tar_path_str,
            "-C",
            dest_dir_str,
            "--strip-components=1",
        ])
        .output()
        .map_err(|e| format!("Failed to execute extract command: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Extraction failed: {stderr}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    /// 构造一个 zip 文件,内含 `entries`(名字 → 内容)。
    fn build_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let file = fs::File::create(path).unwrap();
        let mut zip = ZipWriter::new(file);
        let opts = SimpleFileOptions::default();
        for (name, data) in entries {
            zip.start_file(*name, opts).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
    }

    #[test]
    fn extract_zip_unpacks_files_and_nested_dirs() {
        let tmp = tempfile::TempDir::new().unwrap();
        let zip_path = tmp.path().join("a.zip");
        build_zip(&zip_path, &[("a.txt", b"hello"), ("sub/b.txt", b"world")]);

        let dest = tmp.path().join("out");
        fs::create_dir_all(&dest).unwrap();

        extract_zip(&zip_path, &dest).unwrap();

        assert_eq!(fs::read_to_string(dest.join("a.txt")).unwrap(), "hello");
        assert_eq!(
            fs::read_to_string(dest.join("sub").join("b.txt")).unwrap(),
            "world"
        );
    }

    #[test]
    fn extract_zip_rejects_path_traversal_entries() {
        // zip-slip:恶意条目含 `..`,`enclosed_name()` 返回 None 被跳过,
        // 绝不能写到 dest 目录之外。
        let tmp = tempfile::TempDir::new().unwrap();
        let zip_path = tmp.path().join("evil.zip");
        build_zip(
            &zip_path,
            &[("../escape.txt", b"pwned"), ("ok.txt", b"safe")],
        );

        let dest = tmp.path().join("out");
        fs::create_dir_all(&dest).unwrap();

        extract_zip(&zip_path, &dest).unwrap();

        // 正常条目照常解压
        assert_eq!(fs::read_to_string(dest.join("ok.txt")).unwrap(), "safe");
        // 恶意条目未逃逸到 dest 的父目录
        assert!(!tmp.path().join("escape.txt").exists());
    }
}
