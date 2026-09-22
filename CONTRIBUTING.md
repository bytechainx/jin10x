# CONTRIBUTING.md — 贡献指南（jin10x）

本文件面向贡献者，汇总本地门禁与提交约定。
AI Agent 的工作约定另见 [`AGENTS.md`](./AGENTS.md)；术语与领域语言见 [`CONTEXT.md`](./CONTEXT.md)。

## 开发流程

- 本仓库是**独立的单 crate 仓库**，不依赖 `xhyper.rs` 主工程及其内部 crate，
  也**不依赖**任何其它 `bytechainx/*` 数据仓（`FR-016`）；无 `path` 依赖。
- substantial 变更走 feature branch → PR → review → merge，**禁止直接 push `main`**。
- 提交信息遵循 Conventional Commits（`feat:` / `fix:` / `docs:` / `ci:` / `chore:` / `refactor:`），
  描述用简体中文。

## 本地门禁（P0 三件套 + 打包元数据）

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo package --no-verify
```

本 crate **不发布到 crates.io**，`cargo package` 只校验打包元数据完整性。

## 复用口径（不发布 crates.io）

- 本 crate **不发布到 crates.io**，仅以 GitHub 源码 / git 依赖形式复用。
- 文档与元数据中不得出现「可独立发布」「可直接 `cargo publish`」等表述，
  也不得放置 crates.io / docs.rs 徽章与外链。
- `Cargo.toml` 的 `documentation` 指向 `https://github.com/bytechainx/jin10x#readme`。
- 消费方引入方式（README「安装」小节为准）：

  ```toml
  [dependencies]
  jin10x = { git = "https://github.com/bytechainx/jin10x" }
  ```

## 开发约定

- 注释、文档、错误消息使用**简体中文**；标识符保持英文。
- MSRV 为 Rust 1.71、edition 2021（推导过程见 `CONTEXT.md`）。
- **零网络**：不得引入 `reqwest` / `hyper` / `ureq` / `curl` / `isahc` 等 HTTP 客户端，
  也不得引入 `tokio` / `async-std` / `chrono` / `time` / `rand`。
- **零端点**：`src/` 内不得出现任何 `http://` / `https://` 字面量（含内联测试代码）；
  需要 URL 字面量的负向用例请放到 `tests/` 下。
  **不得**用「拆开 scheme 在运行期拼装」之类写法规避扫描 —— 那会让
  `tests/sdd_spec.rs` 的零端点断言失去意义。
- **零凭据**：不得读环境变量（`std::env::var` / `from_env`）或任何凭据来源。
- 不在库代码里裸 `unwrap()` / `expect()` / `panic!`
  （`[lints.clippy]` 已 `deny`；`benches/` 与测试经显式 `allow`）。
- 所有 `pub` 项必须有中文 `///` 文档（`missing_docs` 已 `deny`）。
- 集成测试**必须离线运行**，不触碰真实网络。
- 值对象不得实现派生指标；`CalendarEvent::surprise_z` 只是源侧字段的承载。
- 单位**保留源侧口径**，不在本层换算。
- 不许用近义 ID 互相顶替（如 `Flash` 与 `FlashNews`、`Quote` 与行情域符号），
  `DataKind::parse` 大小写敏感且不做别名。
- 夹具必须显式标注 `_synthetic`；不得把合成样本写成「实测」「核验 PASS」。

## 提交前自检清单

- [ ] `cargo fmt --all -- --check` 通过
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` 通过
- [ ] `cargo test --all-features` 通过
- [ ] `cargo package --no-verify` 通过
- [ ] `src/` 内无 `http://` / `https://`、无 `env::var`、无 `path = "../`
- [ ] 新增 `pub` 项都有中文 `///` 文档
- [ ] 新增夹具带 `_synthetic: true` 与非空 `_note`
- [ ] `tests/sdd_spec.rs` 的 `// SPEC-MAP:` 与 `docs/标准.md` 的 `##` 章节保持 1:1
