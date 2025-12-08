use reqwest;
use std::time::Duration;
use tokio::net::TcpStream;

/// 网络连接测试工具
pub struct NetworkTester;

impl NetworkTester {
    /// 运行完整的网络诊断
    pub async fn run_full_diagnosis() -> Result<(), String> {
        println!("🔍 fnva 网络连接诊断");
        println!("====================");

        // 测试基本网络连接
        Self::test_basic_connectivity().await?;

        // 测试 Adoptium API
        Self::test_adoptium_api().await?;

        // 测试 GitHub 下载
        Self::test_github_download().await?;

        // 测试 DNS 解析
        Self::test_dns_resolution().await?;

        println!("\n✅ 网络诊断完成");
        Ok(())
    }

    /// 测试基本网络连接
    async fn test_basic_connectivity() -> Result<(), String> {
        println!("\n🌐 测试基本网络连接...");

        let test_urls = vec![
            ("Google DNS", "8.8.8.8:53"),
            ("Cloudflare DNS", "1.1.1.1:53"),
        ];

        for (name, address) in test_urls {
            match tokio::time::timeout(Duration::from_secs(5), TcpStream::connect(address)).await {
                Ok(Ok(_)) => {
                    println!("  ✅ {name}: 连接成功");
                }
                Ok(Err(e)) => {
                    println!("  ❌ {name}: 连接失败 - {e}");
                }
                Err(_) => {
                    println!("  ⏰ {name}: 连接超时");
                }
            }
        }

        Ok(())
    }

    /// 测试 Adoptium API
    async fn test_adoptium_api() -> Result<(), String> {
        println!("\n🔍 测试 Adoptium API...");

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| format!("创建客户端失败: {e}"))?;

        let test_urls = vec![
            (
                "官方 API",
                "https://api.adoptium.net/v3/info/available_releases",
            ),
            (
                "备用 API",
                "https://api.adoptopenjdk.net/v3/info/available_releases",
            ),
        ];

        for (name, url) in test_urls {
            match client.get(url).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        println!("  ✅ {}: 响应正常 ({})", name, response.status());
                    } else {
                        println!("  ⚠️  {}: 响应异常 ({})", name, response.status());
                    }
                }
                Err(e) => {
                    println!("  ❌ {name}: 请求失败 - {e}");
                }
            }
        }

        Ok(())
    }

    /// 测试 GitHub 下载
    async fn test_github_download() -> Result<(), String> {
        println!("\n📥 测试 GitHub 下载连接...");

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| format!("创建客户端失败: {e}"))?;

        let test_url = "https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.1+12/OpenJDK21U-jdk_x64_windows_hotspot_21.0.1_12.msi";

        match client.head(test_url).send().await {
            Ok(response) => {
                println!("  ✅ GitHub: 响应正常 ({})", response.status());
                if let Some(size) = response.headers().get("content-length") {
                    if let Ok(size_str) = size.to_str() {
                        if let Ok(bytes) = size_str.parse::<u64>() {
                            println!("  📊 文件大小: {} MB", bytes / (1024 * 1024));
                        }
                    }
                }
            }
            Err(e) => {
                println!("  ❌ GitHub: 连接失败 - {e}");
            }
        }

        Ok(())
    }

    /// 测试 DNS 解析
    async fn test_dns_resolution() -> Result<(), String> {
        println!("\n🔍 测试 DNS 解析...");

        let hosts = vec!["github.com", "api.adoptium.net", "api.adoptopenjdk.net"];

        for host in hosts {
            match tokio::net::lookup_host(format!("{host}:443")).await {
                Ok(addresses) => {
                    let addr_vec: Vec<_> = addresses.collect();
                    if !addr_vec.is_empty() {
                        println!("  ✅ {}: 解析成功 ({})", host, addr_vec.first().unwrap());
                    } else {
                        println!("  ⚠️  {host}: 解析成功但无地址");
                    }
                }
                Err(e) => {
                    println!("  ❌ {host}: 解析失败 - {e}");
                }
            }
        }

        Ok(())
    }

    /// 测试特定 URL 的可访问性
    pub async fn test_url_accessibility(url: &str) -> Result<(), String> {
        println!("🔍 测试 URL 可访问性: {url}");

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| format!("创建客户端失败: {e}"))?;

        let start_time = std::time::Instant::now();

        match client.head(url).send().await {
            Ok(response) => {
                let duration = start_time.elapsed();
                println!("  ✅ 响应时间: {duration:?}");
                println!("  📊 状态码: {}", response.status());

                if let Some(size) = response.headers().get("content-length") {
                    if let Ok(size_str) = size.to_str() {
                        println!("  📊 内容长度: {size_str}");
                    }
                }

                if response.status().is_success() {
                    Ok(())
                } else {
                    Err(format!("服务器返回错误: {}", response.status()))
                }
            }
            Err(e) => Err(format!("请求失败: {e}")),
        }
    }

    /// 提供网络问题的解决建议
    pub fn provide_suggestions(error: &str) -> Vec<String> {
        let mut suggestions = Vec::new();

        if error.contains("DNS") || error.contains("resolve") {
            suggestions.push("尝试更换 DNS 服务器（如 8.8.8.8 或 1.1.1.1）".to_string());
            suggestions.push("检查 hosts 文件是否被修改".to_string());
            suggestions.push("运行 'fnva network-test' 进行详细诊断".to_string());
        }

        if error.contains("timeout") || error.contains("timed out") {
            suggestions.push("检查防火墙设置".to_string());
            suggestions.push("尝试使用不同的网络连接".to_string());
            suggestions.push("确认网络代理配置正确".to_string());
        }

        if error.contains("connection closed") || error.contains("reset") {
            suggestions.push("网络连接不稳定，请稍后重试".to_string());
            suggestions.push("尝试使用有线连接".to_string());
            suggestions.push("关闭其他占用带宽的应用".to_string());
        }

        if error.contains("SSL") || error.contains("TLS") || error.contains("certificate") {
            suggestions.push("更新系统证书".to_string());
            suggestions.push("检查系统时间是否正确".to_string());
            suggestions.push("确认没有中间人攻击".to_string());
        }

        if suggestions.is_empty() {
            suggestions.push("运行 'fnva network-test' 进行详细诊断".to_string());
            suggestions.push("查看 NETWORK_TROUBLESHOOTING.md 获取更多信息".to_string());
            suggestions.push("如果问题持续，请尝试手动下载安装".to_string());
        }

        suggestions
    }
}
