# Tool Source Protocol 设计

fnva 把「下载二进制发行版的工具」(Java、Maven,及未来 Gradle/Node)统一在一套**工具无关的协议**下。本文档描述该协议的抽象与扩展方式。

## 背景

最初只有 Java 走「版本注册表 + 镜像下载」,且镜像解析逻辑被 Java 绑死(`MirrorsConfig.java` 字段、`{major}/{tag}` 模板变量、`JavaDownloader` trait 吃 `UnifiedJavaVersion`)。本协议把这些泛化,使任意「下载型工具」可插拔接入。

## 核心抽象(`src/infrastructure/tool_protocol/`)

| 组件 | 职责 |
|---|---|
| `ToolDescriptor` | 工具元信息:id、资产模型(`PerPlatform`/`SingleArchive`)、安装子目录、`home_validator`、`locate_home` |
| `TemplateVars` | 通用模板变量(`{version}/{major}/{tag}/{filename}/{os}/{arch}`),`major`/`tag` 为 `Option`——Maven 不填也能渲染 |
| `MirrorResolver` | 工具无关的「模板渲染 + HEAD 探测 + 按优先级回退」,直接拉镜像资源,无需自建静态服务器 |
| `VersionDiscovery`(trait) | 版本发现策略,可插拔:`EmbeddedRegistryDiscovery`(Java,读嵌入 toml)/ `MirrorDirectoryDiscovery`(Maven,抓镜像目录) |
| `ResolvedVersion` | 通用版本模型(取代 Java 专属的 `UnifiedJavaVersion`) |
| `ToolDownloader`(trait) | 泛化下载器(取代 `JavaDownloader`) |

## 源/镜像配置

镜像在 `~/.fnva/config.toml` 的 `[[mirrors.<tool>]]` 段配置,按 `priority` 排序、HEAD 探测后自动回退:

```toml
[[mirrors.maven]]
name = "tsinghua"          # 第一优先级(下载加速)
priority = 1
base_url = "https://mirrors.tuna.tsinghua.edu.cn/apache/maven/maven-3"
url_template = "{base_url}/{version}/binaries/apache-maven-{version}-bin.tar.gz"

[[mirrors.maven]]
name = "apache-archive"    # 回退源(完整历史)
priority = 2
base_url = "https://archive.apache.org/dist/maven/maven-3"
url_template = "{base_url}/{version}/binaries/apache-maven-{version}-bin.tar.gz"
```

> 清华镜像的 `maven-3/` 目录只保留最新版,因此 **list(列版本)用 apache archive 作权威源,清华只作下载加速**。用户装老版本时清华无该版本 → HEAD 失败 → 自动回退 archive,对用户透明。

## 如何接入一个新工具(以 Gradle 为例)

1. **版本发现**:实现 `VersionDiscovery`(如抓 Gradle 的目录或 service。
2. **下载器**:实现 `ToolDownloader`(装配 `MirrorResolver` + 你的 discovery),或复用通用模式。
3. **描述符**:定义 `const GRADLE_DESCRIPTOR: ToolDescriptor { install_subdir: "gradle-packages", home_validator: validate_gradle_home, locate_home: ..., ... }`。
4. **安装器**:调 `infrastructure::installer::generic::download_and_install(&downloader, &version, &platform, name, &GRADLE_DESCRIPTOR)`。
5. **环境切换**:加 `EnvironmentType::Gradle` + 4 个 shell 的 `*_gradle_switch` 模板(注入 `GRADLE_HOME`/`PATH`)。
6. **CLI**:加 `Commands::Gradle` + `handle_gradle_command`。
7. **镜像配置**:加 `[[mirrors.gradle]]` 段 + `Config::sync()` 补全。

`generic::download_and_install`(下载→解压→`locate_home`→`home_validator`)与 shell 集成层(`switcher`/`ScriptGenerator`)对任意工具通用,无需重复实现。
