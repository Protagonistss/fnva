# 贡献指南 (Contributing)

感谢您对 fnva 项目的关注！我们欢迎任何形式的贡献：提交 Bug、提供建议、改进文档或直接提交 PR。

## 本地构建与测试

fnva 使用标准 Rust 工具链进行开发。

1. **安装 Rust**: 请确保已安装最新的稳定版 Rust (推荐通过 rustup)。
2. **克隆项目**:
   ```bash
   git clone https://github.com/Protagonistss/fnva.git
   cd fnva
   ```
3. **格式化与代码检查**:
   ```bash
   cargo fmt
   cargo clippy --all-targets -- -D warnings
   ```
4. **运行单元测试**:
   ```bash
   cargo test
   ```
5. **本地编译**:
   ```bash
   cargo build --release
   ```
   编译产物位于 `target/release/fnva`。

## 代码规范

- 遵循 Rust 标准风格，所有代码在提交前必须通过 `cargo fmt` 和 `cargo clippy`。
- 新增特性需包含相应的单元测试。
- 核心修改需要同步更新相关文档（如 `README.md` 或 `docs/` 下的文件）。
