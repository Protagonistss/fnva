//! 通用安装骨架 —— 工具无关的「下载 → 解压 → 定位 home」流程。
//!
//! 工具差异(安装子目录、home 定位方式)由 [`ToolDescriptor`] 参数化。
//! Java / Maven 的 installer 都调本模块的 [`download_and_install`]。

use crate::infrastructure::installer::utils::{create_progress_bar, extract_tar_gz, extract_zip};
use crate::infrastructure::remote::platform::Platform;
use crate::infrastructure::remote::DownloadTarget;
use crate::infrastructure::tool_protocol::{ResolvedVersion, ToolDescriptor, ToolDownloader};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::{Path, PathBuf};

/// 通用安装骨架:下载 → 解压到 `~/.fnva/{install_subdir}/{env_name}` → 定位 home。
///
/// 返回最终的 home 目录字符串(已通过 `descriptor.home_validator` 校验)。
pub async fn download_and_install(
    downloader: &dyn ToolDownloader,
    version: &ResolvedVersion,
    platform: &Platform,
    env_name: &str,
    descriptor: &ToolDescriptor,
) -> Result<String, String> {
    let pb = create_progress_bar().unwrap_or_else(|_| fallback_spinner());
    let pb_clone = pb.clone();

    let target = downloader
        .download(
            version,
            platform,
            Box::new(move |downloaded, total| {
                if total > 0 {
                    if pb_clone.length().unwrap_or(0) == 0 {
                        pb_clone.set_length(total);
                    }
                    pb_clone.set_position(downloaded);
                } else {
                    pb_clone.set_message(format!("Downloaded: {} MB", downloaded / (1024 * 1024)));
                    pb_clone.tick();
                }
            }),
        )
        .await
        .map_err(|e| format!("Download failed: {e:?}"))?;
    pb.finish_with_message("Download complete");

    let file_path = match target {
        DownloadTarget::File(p) => PathBuf::from(p),
        DownloadTarget::Bytes(_) => {
            return Err("In-memory download not supported".to_string());
        }
    };

    let home = install_archive(&file_path, env_name, descriptor)?;
    if !(descriptor.home_validator)(&home) {
        return Err(format!(
            "{} installation verification failed",
            descriptor.display_name
        ));
    }
    Ok(home)
}

fn fallback_spinner() -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );
    pb
}

/// 解压归档到 `~/.fnva/{install_subdir}/{env_name}`,再用 `descriptor.locate_home`
/// 定位实际 home。
fn install_archive(
    archive_path: &Path,
    env_name: &str,
    descriptor: &ToolDescriptor,
) -> Result<String, String> {
    let fnva_dir = crate::infrastructure::paths::fnva_dir()?.join(descriptor.install_subdir);

    fs::create_dir_all(&fnva_dir).map_err(|e| format!("Failed to create install dir: {e}"))?;

    let install_dir = fnva_dir.join(env_name);
    fs::create_dir_all(&install_dir).map_err(|e| format!("Failed to create version dir: {e}"))?;

    if archive_path.to_str().unwrap().ends_with(".zip") {
        extract_zip(archive_path, &install_dir)?;
    } else {
        extract_tar_gz(archive_path, &install_dir)?;
    }

    let actual_home = (descriptor.locate_home)(&install_dir)?;
    Ok(actual_home)
}
