use std::fmt;

/// 简单封装的平台信息，统一 OS / Arch / 默认压缩格式的判定。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Platform {
    pub os: String,
    pub arch: String,
}

impl Platform {
    /// 检测当前运行平台。
    pub fn current() -> Self {
        let os = if cfg!(target_os = "windows") {
            "windows"
        } else if cfg!(target_os = "macos") {
            "macos"
        } else if cfg!(target_os = "linux") {
            "linux"
        } else {
            "unknown"
        };

        let arch = if cfg!(target_arch = "x86_64") {
            "x64"
        } else if cfg!(target_arch = "aarch64") {
            "aarch64"
        } else if cfg!(target_arch = "x86") {
            "x86"
        } else {
            "unknown"
        };

        Platform {
            os: os.to_string(),
            arch: arch.to_string(),
        }
    }

    /// 针对当前平台返回默认压缩格式。
    pub fn archive_ext(&self) -> &'static str {
        match self.os.as_str() {
            "windows" => "zip",
            _ => "tar.gz",
        }
    }

    /// 生成 key 供下载 URL 查找使用。
    pub fn key(&self) -> String {
        format!("{}-{}", self.os, self.arch)
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}", self.os, self.arch)
    }
}
