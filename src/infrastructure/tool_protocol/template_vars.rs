use std::collections::HashMap;

/// 模板渲染的通用变量上下文。
///
/// 取代 `TemplateDownloader::render_url` 写死的 6 个具名参数。关键差异:
/// `major` / `tag` 为 `Option`——Java 填,Maven 不填也能渲染,这是统一两类工具
/// 模板的支点。
#[derive(Debug, Clone, Default)]
pub struct TemplateVars {
    /// `{version}` —— Maven / Java 通用
    pub version: String,
    /// `{major}` —— Java 专属,Maven 为 `None`
    pub major: Option<u32>,
    /// `{tag}` —— Java 的 jdk-x.y.z+q,Maven 为 `None`
    pub tag: Option<String>,
    /// `{filename}` —— 已按平台解析好的最终文件名
    pub filename: String,
    /// `{os}` —— linux / macos / windows
    pub os: String,
    /// `{arch}` —— x64 / aarch64
    pub arch: String,
    /// 额外自由变量(供未来扩展,如 {classifier})
    pub extra: HashMap<String, String>,
}

impl TemplateVars {
    /// 把模板里的 `{base_url}` / `{version}` / `{major}` / `{tag}` /
    /// `{filename}` / `{os}` / `{arch}` 及 `extra` 中的自定义占位符全部替换。
    ///
    /// `base_url` 单独传入:每个 mirror 的 base_url 不同,由 resolver 在循环里提供。
    pub fn render(template: &str, base_url: &str, vars: &TemplateVars) -> String {
        let mut out = template
            .replace("{base_url}", base_url)
            .replace("{version}", &vars.version)
            .replace(
                "{major}",
                vars.major.map(|m| m.to_string()).as_deref().unwrap_or(""),
            )
            .replace("{tag}", vars.tag.as_deref().unwrap_or(""))
            .replace("{filename}", &vars.filename)
            .replace("{os}", &vars.os)
            .replace("{arch}", &vars.arch);
        for (k, v) in &vars.extra {
            out = out.replace(&format!("{{{k}}}"), v);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_java_template() {
        let vars = TemplateVars {
            version: "21.0.10+7".into(),
            major: Some(21),
            tag: Some("jdk-21.0.10+7".into()),
            filename: "OpenJDK21U-jdk_x64_linux_hotspot_21.0.10_7.tar.gz".into(),
            os: "linux".into(),
            arch: "x64".into(),
            ..Default::default()
        };
        let url = TemplateVars::render(
            "{base_url}/{major}/jdk/{arch}/{os}/{filename}",
            "https://mirrors.tuna.tsinghua.edu.cn/Adoptium",
            &vars,
        );
        assert_eq!(
            url,
            "https://mirrors.tuna.tsinghua.edu.cn/Adoptium/21/jdk/x64/linux/\
             OpenJDK21U-jdk_x64_linux_hotspot_21.0.10_7.tar.gz"
        );
    }

    #[test]
    fn render_maven_template_without_major_tag() {
        // Maven 单包跨平台:模板只用 {version} / {filename},major/tag 留空
        let vars = TemplateVars {
            version: "3.9.16".into(),
            filename: "apache-maven-3.9.16-bin.tar.gz".into(),
            ..Default::default()
        };
        let url = TemplateVars::render(
            "{base_url}/{version}/binaries/apache-maven-{version}-bin.tar.gz",
            "https://archive.apache.org/dist/maven/maven-3",
            &vars,
        );
        assert_eq!(
            url,
            "https://archive.apache.org/dist/maven/maven-3/\
             3.9.16/binaries/apache-maven-3.9.16-bin.tar.gz"
        );
    }

    #[test]
    fn render_extra_var() {
        let mut vars = TemplateVars::default();
        vars.extra.insert("classifier".into(), "sources".into());
        let url = TemplateVars::render("{base_url}/{classifier}", "https://x", &vars);
        assert_eq!(url, "https://x/sources");
    }
}
