# jin10x 上下文

本文件定义 `jin10x` 与其使用方共享的核心词汇、能力边界与已知缺口。
它只记录领域含义与取舍，不复制 API 签名。

## 角色与边界

**源事实层**：把 `specs/adapter/jin10.md` 声明的类型、常量与禁则落成可编译、可判定、
可测试的代码。它**不是**采集器，**不是**存储层，也**不是**派生指标层。
_Avoid_: 采集器 / 适配器运行时（本层无网络、无凭据、无调度）

**离线解析**：输入是**字符串**（JSON 文档），输出是类型化值对象集合。
参数里没有 URL、客户端、认证信息与环境变量。
_Avoid_: 客户端 / 传输层（解析不产生任何 I/O）

**提议 ≠ 写入**：`propose_macro_mapping` 产生的是「可否进入宏观观测」的**提议**；
`decide_macro_write` 产生的是**决策**。本层不持有任何存储，也不双写。
_Avoid_: 写入 / 落库（本层只做判定）

## 源事实词汇

**信封**（`Jin10Envelope<T>`）：清单固定的七字段容器 —— `msg_id` / `source` / `kind` /
`event_time` / `collect_time` / `payload` / `raw_ref`。字段集不增不减。
_Avoid_: 消息体（信封不解释载荷内部语义）

**数据类别**（`DataKind`）：清单固定的八类。其中 `Quote` 属行情域、`Treasury`
（拍卖 / RRP）与 `treasuryx` 重叠。
_Avoid_: 主题 / 标签（类别是固定枚举，不是自由文本）

**规划周期**：`INTERFACE_PLANS` 里的秒数字段，取自清单的「规划」表。
它是**规划语义**，**不是**访问合同：不表达访问频率、限流配额、SLA 或新鲜度保证。
_Avoid_: 采集周期 / 限流（本层无采集能力）

**原始报文引用**（`raw_ref`）：对象引用式的定位串，**不是** URL。
含 `://` 者按语义拒绝，因为本层不做任何网络引用解析。
_Avoid_: 下载链接（本层不解引用）

## 判定词汇

**三要素**：可验证 `indicator` + 主体 + 业务期间。三者**齐备**才可提议映射；
缺失返回 `Missing`，存在但为空/全空白返回 `SemanticallyRejected`。
_Avoid_: 启发式推断（本层不猜 indicator，也不猜期间）

**乱序保护**：候选到达序号小于既有事实时返回 `StaleRejected`，**不得**覆盖既有事实；
同身份同值按幂等处理；更晚到达但取值不同返回 `Conflict`，**不**静默覆盖。
_Avoid_: 覆盖 / 更新（本层没有修订面，实测修订需显式流程）

**写入主权**（`Jin10WriteSovereignty`）：`Own` / `Pending` / `Routed` 三态**只读结论**。
拍卖 / RRP 为 `Pending`（未决），双方均不得主张权威写入 —— 争议由 Owner 另裁。
_Avoid_: 授权（主权归属与授权是两件事）

**fail-closed 授权**：证据缺失、覆盖范围不明、签署者不明、未获 Owner 签核、
尚未生效、已过期 —— 六条路径一律 `Denied`。**不得**默认放行。
_Avoid_: 清单批准（清单登记的是现状，不是签核证据）

## 已落地与未落地

**已落地**：类型、常量、离线解析（仅三类载荷）、校验、映射守卫、写入主权登记、
授权判定、publication 语义、三类测试与合成夹具、微基准。

**未落地（不假装支持）**：

1. S01–S08 的传输层与采集（无授权，`FR-037`）；
2. 五类载荷（`BigEvent` / `CentralBank` / `Treasury` / `Cot` / `EtfInventory`）
   的字段未被清单固定，解析返回 `NotApplicable`；
3. 官方 vintage / 发布日历 —— 故 PIT 资格恒 `NotEligible`。

## rust-version 推导

规则：`rust-version` = 依赖图中所有依赖所声明 `rust_version` 的**最大值**
（`FR-014` 的可核验化，见 `contracts/source-library-contract.md` §7.3）。

`cargo metadata --format-version 1` 实读结果（2026-09-22）：

| 依赖 | 版本 | 其 `rust_version` |
| --- | --- | --- |
| `thiserror` | 2.0.20 | 1.71 |
| `thiserror-impl` | 2.0.20 | 1.71 |
| `serde` | 1.0.229 | 1.56 |
| `serde_core` | 1.0.229 | 1.56 |
| `serde_derive` | 1.0.229 | 1.71 |
| `serde_json` | 1.0.151 | 1.71 |
| `proc-macro2` | 1.0.107 | 1.71 |
| `quote` | 1.0.47 | 1.71 |
| `syn` | 3.0.6 | 1.71 |
| `unicode-ident` | 1.0.26 | 1.71 |
| `memchr` | 2.8.3 | 1.61 |
| `itoa` | 1.0.18 | 1.68 |

**推导结果**：`1.71`（最大值取自 `thiserror` / `serde_derive` / `serde_json` 等）。
语言特性方面本 crate 只用到 edition 2021 与标准库，未额外抬高该下界。

## 已知缺口

1. **夹具全为合成样本**：来自上游的真实 fixture 已随重写删除，本层无真实样本可用；
   因此解析器只接受显式标注（`_synthetic: true` + 非空 `_note`）的文件。
2. **时间精度**：信封的 `event_time` / `collect_time` 支持 `YYYY-MM-DD` 与
   `YYYY-MM-DDTHH:MM:SS` 两种形态；清单未固定格式，故本层不推断时区、不补造时刻。
3. **五类载荷未实现**：非「待办」，而是清单未固定字段 —— 实现它们等于编造源事实。
4. **不可用于生产**：`production_decision = NO-GO`，且授权判定在真实调用上返回 `Denied`。
