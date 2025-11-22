use super::{UnifiedJavaVersion, DownloadError};

pub fn find_best_version(versions: &[UnifiedJavaVersion], spec: &str) -> Result<UnifiedJavaVersion, DownloadError> {
    let spec_cleaned = spec.trim().to_lowercase()
        .replace("v", "")
        .replace("jdk", "")
        .replace("java", "")
        .trim()
        .to_string();

    if spec_cleaned == "lts" || spec_cleaned == "latest-lts" {
        for version in versions {
            if version.is_lts {
                return Ok(version.clone());
            }
        }
        return Err(DownloadError::from("未找到 LTS 版本".to_string()));
    } else if spec_cleaned == "latest" || spec_cleaned == "newest" {
         return versions.first().cloned()
            .ok_or_else(|| DownloadError::from("未找到可用版本".to_string()));
    }

    // 尝试解析为主版本号或完整版本号
    let parts: Vec<&str> = spec_cleaned.split('.').filter(|p| !p.is_empty()).collect();
    
    if !parts.is_empty() && parts[0].parse::<u32>().is_ok() {
        if parts.len() == 1 {
            // 主版本号输入（如 "8"）- LTS优先策略
            let major = parts[0].parse::<u32>().unwrap();
            
            // 首先查找该主版本的LTS版本
            // versions 已经是降序排列
            let lts_version = versions.iter()
                .find(|v| v.major == major && v.is_lts);
            
            if let Some(v) = lts_version {
                return Ok(v.clone());
            }
            
            // 如果没有LTS版本，返回该主版本的最新版本
            let latest_version = versions.iter()
                .find(|v| v.major == major);
            
            if let Some(v) = latest_version {
                return Ok(v.clone());
            }
            
            return Err(DownloadError::from(format!("未找到 Java {}", major)));
        } else {
            // 完整版本号输入（如 "8.0.2"）- 精确匹配优先
            let full_version = parts.join(".");
            
            // 首先尝试精确匹配
            for version in versions {
                if version.version == full_version ||
                   version.version.replace('-', ".") == full_version ||
                   version.tag_name.contains(&full_version) ||
                   version.release_name.to_lowercase().contains(&full_version) {
                    return Ok(version.clone());
                }
            }
            
            // 精确匹配失败，尝试主版本匹配
            let major = parts[0].parse::<u32>().unwrap();
            for version in versions {
                if version.major == major {
                    return Ok(version.clone());
                }
            }
            
            return Err(DownloadError::from(format!("未找到版本: {}", spec)));
        }
    }

    // 尝试直接字符串匹配（向后兼容）
    for version in versions {
        if version.version == spec_cleaned || 
           version.tag_name == spec_cleaned ||
           version.release_name.to_lowercase().contains(&spec_cleaned) {
            return Ok(version.clone());
        }
    }

    Err(DownloadError::from(format!("未找到版本: {}", spec)))
}

