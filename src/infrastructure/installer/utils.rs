use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn create_progress_bar() -> ProgressBar {
    let pb = ProgressBar::new(0);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta}) {percent}%")
            .unwrap()
            .progress_chars("#>-")
    );
    pb
}

pub fn extract_zip(zip_path: &Path, dest_dir: &Path) -> Result<(), String> {
    let file = fs::File::open(zip_path).map_err(|e| format!("打开 ZIP 文件失败: {}", e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("读取 ZIP 文件失败: {}", e))?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| format!("读取 ZIP 文件项失败: {}", e))?;
        let outpath = dest_dir.join(file.mangled_name());
        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath).map_err(|e| format!("创建目录失败: {}", e))?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p).map_err(|e| format!("创建父目录失败: {}", e))?;
                }
            }
            let mut outfile = fs::File::create(&outpath).map_err(|e| format!("创建文件失败: {}", e))?;
            std::io::copy(&mut file, &mut outfile).map_err(|e| format!("写入文件失败: {}", e))?;
        }
    }
    Ok(())
}

pub fn extract_tar_gz(tar_path: &Path, dest_dir: &Path) -> Result<(), String> {
    let output = Command::new("tar")
        .args(["-xzf", tar_path.to_str().unwrap(), "-C", dest_dir.to_str().unwrap(), "--strip-components=1"])
        .output()
        .map_err(|e| format!("执行解压命令失败: {}", e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("解压失败: {}", stderr));
    }
    Ok(())
}

