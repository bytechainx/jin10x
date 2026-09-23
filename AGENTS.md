# jin10x Agent 指南

> 本文件为 AI Agent 在本仓库工作时的入口指南。

## 项目定位

金十数据源（`source_id = jin10`）的**源事实层**：类型化信封与数据类别 + 离线解析 +
跨域守卫 + fail-closed 授权判定。**不是**采集器，**不是**存储层。

`production_decision = NO-GO`；本源 `authorization = unknown`（无 Owner 签核文件）。

## 技术栈

- Rust edition 2021, rust-version 1.71（推导见 `CONTEXT.md`）
- 依赖：`thiserror` / `serde`（`derive`）/ `serde_json`；**无**其它依赖
- 禁止：任何 HTTP 客户端、异步运行时、`chrono` / `time` / `rand`、
  任何 `bytechainx/*` crate、任何 `path = "../…"` 依赖
- crate 级 lint：`unwrap_used` / `expect_used` / `panic` / `unreachable` / `todo` /
  `unimplemented` 全部 `deny`（测试与 bench 经 `#![allow(…)]` 或 `cfg_attr(test, allow(…))` 豁免）

## 代码结构

```text
src/
├── lib.rs              # crate 文档 + 模块声明 + 门面 re-export
├── error.rs            # Jin10ErrorKind / Jin10Error / Jin10Result
├── value.rs            # Date / Period / Frequency / Jin10Unit + 子模块门面
├── value/
│   ├── envelope.rs     # DataKind / 接口常量 / 时间戳 / 重要度
│   ├── payload.rs      # 信封 + 三类载荷 + validate_*
│   └── mapping.rs      # 宏观映射提议 + 乱序保护 + 写入主权登记
├── parse.rs            # 离线解析 parse_jin10_envelopes
├── authz.rs            # fail-closed 授权判定
└── pit.rs              # publication 语义三元组
tests/
├── tdd_contracts.rs    # TDD-PROBE 四列表（覆盖全部公开入口）
├── sdd_spec.rs         # SPEC-MAP 与 docs/标准.md 的 ## 章节 1:1
├── aidd_boundary.rs    # AIDD 复核表（≥5 条，五列非空）
└── fixtures/           # 合成样本（_synthetic 标注）
benches/hot_path.rs     # 微基准（harness = false）
```

模块依赖方向单向：`error ← value ← {authz, parse}`，`pit` 独立；无环。

## 开发约定

- 注释与文档使用简体中文；标识符保持英文
- 错误：`thiserror` 枚举 + `#[non_exhaustive]` + `Result` 别名；分类决定「如何反应」
- **不得**在 `src/` 写端点的 URL / Header / 限流数字 / 统计码全集
- **不得**用「拆开 scheme 在运行期拼装」规避 `src/` 的端点字面量扫描：需要 URL
  字面量的负向用例放 `tests/`；`tests/sdd_spec.rs` 会运行期递归枚举 `src/` 全部
  `.rs` 并断言零端点字面量
- `tests/tdd_contracts.rs` 的入口列须为 `类型::方法` 或裸函数名（不带参数与返回值）
- **归属判定只看证据**：`mtime` 只证明「某时刻被写过」，**不能**证明「不是我写的」。
  主张「他人并发写入」前，必须先问「这是不是我上一轮的动作」（查自己的工具调用记录即可），
  并同时具备三件证据：① 内容与本意不符；② 可复现且指向**另一主体**的载荷；
  ③ 已排除自己早前动作。三条不齐时，记录为「归属未决」，**不得**写成结论。
- **不得**读环境变量或凭据；本层只做离线解析
- **不得**把清单未固定的内容补齐（其余五类载荷字段、端点、限流数字）
- **不得**把合成夹具表述为「实测」「核验 PASS」
- 授权判定保持 fail-closed：证据缺失一律 `Denied`，不得默认放行
- 拍卖 / RRP 的写入主权保持 `pending`，**不得**自行裁定归属

## 门禁

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo package --no-verify
```

微基准（不参与 P0 门禁）：

```bash
cargo bench --bench hot_path
```

## 相关文档

- 采集范围权威：`specs/adapter/jin10.md`
- 公共形状契约：`specs/features/005-macro-data-source-crates/contracts/source-library-contract.md`
- 跨源路由与主权：`specs/features/005-macro-data-source-crates/contracts/cross-source-routing.md`
- API 文档：`docs/API.md`
- 标准与验收：`docs/标准.md`
- 术语与领域语言：`CONTEXT.md`
- 贡献指南：`CONTRIBUTING.md`
- 变更记录：`CHANGELOG.md`
