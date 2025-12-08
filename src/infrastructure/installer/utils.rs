use crate::error::{safe_path_to_str, AppError};
use crate::infrastructure::remote::UnifiedJavaVersion;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::Path;

pub fn create_progress_bar() -> Result<ProgressBar, AppError> {
    let pb = ProgressBar::new(0);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta}) {percent}%")
            .map_err(|e| AppError::Internal {
                message: format!("创建进度条样式失败: {e}")
            })?
            .progress_chars("#>-")
    );
    Ok(pb)
}

pub fn extract_zip(zip_path: &Path, dest_dir: &Path) -> Result<(), String> {
    let file = fs::File::open(zip_path).map_err(|e| format!("打开 ZIP 文件失败: {e}"))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("读取 ZIP 文件失败: {e}"))?;
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("读取 ZIP 文件项失败: {e}"))?;
        let outpath = dest_dir.join(file.mangled_name());
        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath).map_err(|e| format!("创建目录失败: {e}"))?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p).map_err(|e| format!("创建父目录失败: {e}"))?;
                }
            }
            let mut outfile =
                fs::File::create(&outpath).map_err(|e| format!("创建文件失败: {e}"))?;
            std::io::copy(&mut file, &mut outfile).map_err(|e| format!("写入文件失败: {e}"))?;
        }
    }
    Ok(())
}

pub fn extract_tar_gz(tar_path: &Path, dest_dir: &Path) -> Result<(), String> {
    let tar_path_str = safe_path_to_str(tar_path).map_err(|e| format!("路径转换失败: {e}"))?;
    let dest_dir_str = safe_path_to_str(dest_dir).map_err(|e| format!("目标路径转换失败: {e}"))?;

    let output = std::process::Command::new("tar")
        .args([
            "-xzf",
            tar_path_str,
            "-C",
            dest_dir_str,
            "--strip-components=1",
        ])
        .output()
        .map_err(|e| format!("执行解压命令失败: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("解压失败: {stderr}"));
    }
    Ok(())
}

pub fn pick_best_version(
    versions: Vec<UnifiedJavaVersion>,
    spec: &str,
) -> Result<UnifiedJavaVersion, crate::remote::DownloadError> {
    let spec_cleaned = spec
        .trim()
        .to_lowercase()
        .replace("v", "")
        .replace("jdk", "")
        .replace("java", "")
        .trim()
        .to_string();

    if spec_cleaned == "lts" || spec_cleaned == "latest-lts" {
        // 返回最新的 LTS 版本
        let mut lts_versions: Vec<UnifiedJavaVersion> =
            versions.into_iter().filter(|v| v.is_lts).collect();
        lts_versions.sort_by(|a, b| b.major.cmp(&a.major));
        return lts_versions
            .into_iter()
            .next()
            .ok_or(crate::remote::DownloadError::NotFound);
    } else if spec_cleaned == "latest" || spec_cleaned == "newest" {
        // 返回最新版本
        let mut sorted_versions: Vec<UnifiedJavaVersion> = versions.into_iter().collect();
        sorted_versions.sort_by(|a, b| b.major.cmp(&a.major));
        return sorted_versions
            .into_iter()
            .next()
            .ok_or(crate::remote::DownloadError::NotFound);
    }

    // 尝试解析为主版本号或完整版本号
    let parts: Vec<&str> = spec_cleaned.split('.').filter(|p| !p.is_empty()).collect();

    if !parts.is_empty() && parts[0].parse::<u32>().is_ok() {
        if parts.len() == 1 {
            // 主版本号输入（如 "8"）- LTS优先策略
            let major = parts[0]
                .parse::<u32>()
                .map_err(|_| crate::remote::DownloadError::VersionParse)?;

            // 首先查找该主版本的LTS版本，按版本号倒序（最新版本优先）
            let mut lts_versions: Vec<UnifiedJavaVersion> = versions
                .iter()
                .filter(|v| v.major == major && v.is_lts)
                .cloned()
                .collect();

            lts_versions.sort_by(|a, b| {
                let a_parts: Vec<&str> = a.version.split('.').collect();
                let b_parts: Vec<&str> = b.version.split('.').collect();
                b_parts.cmp(&a_parts) // 倒序
            });

            if let Some(latest_lts) = lts_versions.first() {
                return Ok(latest_lts.clone());
            }

            // 如果没有LTS版本，返回该主版本的最新版本
            let mut major_versions: Vec<UnifiedJavaVersion> = versions
                .iter()
                .filter(|v| v.major == major)
                .cloned()
                .collect();

            major_versions.sort_by(|a, b| {
                let a_parts: Vec<&str> = a.version.split('.').collect();
                let b_parts: Vec<&str> = b.version.split('.').collect();
                b_parts.cmp(&a_parts) // 倒序
            });

            if let Some(latest) = major_versions.first() {
                return Ok(latest.clone());
            }

            return Err(crate::remote::DownloadError::NotFound);
        } else {
            // 完整版本号输入（如 "8.0.2"）- 精确匹配优先
            let full_version = parts.join(".");

            // 首先尝试精确匹配
            for version in &versions {
                if version.version == full_version
                    || version.version.replace('-', ".") == full_version
                    || version.tag_name.contains(&full_version)
                    || version.release_name.to_lowercase().contains(&full_version)
                {
                    return Ok(version.clone());
                }
            }

            // 精确匹配失败，尝试主版本匹配
            let major = parts[0]
                .parse::<u32>()
                .map_err(|_| crate::remote::DownloadError::VersionParse)?;
            for version in &versions {
                if version.major == major {
                    return Ok(version.clone());
                }
            }

            return Err(crate::remote::DownloadError::NotFound);
        }
    }

    // 尝试直接字符串匹配（向后兼容）
    for version in &versions {
        if version.version == spec_cleaned
            || version.tag_name == spec_cleaned
            || version.release_name.to_lowercase().contains(&spec_cleaned)
        {
            return Ok(version.clone());
        }
    }
    Err(crate::remote::DownloadError::NotFound)
}
