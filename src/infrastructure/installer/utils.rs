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
        let outpath = dest_dir.join(file.mangled_name());
        if file.name().ends_with('/') {
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
