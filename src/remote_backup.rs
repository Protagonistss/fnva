// 备份原有的数据结构
use reqwest;
use serde::Deserialize;

/// 远程查询管理器
pub struct RemoteManager;

/// Adoptium API 返回的 Java 版本信息
#[derive(Debug, Deserialize)]
pub struct AdoptiumRelease {
    pub release_name: String,
    pub version: Option<AdoptiumVersion>,
    pub binaries: Vec<AdoptiumBinary>,
}

#[derive(Debug, Deserialize)]
pub struct AdoptiumVersion {
    pub semver: String,
    pub major: u32,
    pub minor: u32,
    pub security: u32,
    pub build: Option<u32>,
    pub optional: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AdoptiumBinary {
    pub os: String,
    pub architecture: String,
    pub image_type: String,
    pub package: Option<AdoptiumPackage>,
}

#[derive(Debug, Deserialize)]
pub struct AdoptiumPackage {
    pub name: String,
    pub link: String,
}