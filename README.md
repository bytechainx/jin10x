# jin10x

`jin10x` 是金十数据源（`source_id = jin10`）的**源事实层**：把清单声明的信封、数据类别、
规划接口与跨域禁则落成可编译、可测、可判定的 Rust 库，并提供**离线**解析与 fail-closed 授权判定。

- **零网络**：没有 HTTP 客户端、没有异步运行时、没有端点字面量、没有凭据读取。
- **fail-closed**：本源授权为 `unknown`（无 Owner 签核文件），授权判定默认拒绝。
- **类型化源事实**：八类 `DataKind`、七字段信封、三类载荷、S01–S08 规划接口常量。
- **可判定的禁则**：`Quote` 默认拒绝进入宏观观测；拍卖 / RRP 写入主权 `pending`（未决），保持拒绝。
- **诚实边界**：清单未固定的内容（端点、Header、限流数字、其余五类载荷字段）一律不编造。

## 安装

本 crate **不发布到 crates.io**，仅以 GitHub 源码 / **git 依赖**形式引入：

```toml
[dependencies]
jin10x = { git = "https://github.com/bytechainx/jin10x" }
```

本地同仓开发也可用路径引入：

```toml
jin10x = { path = "../jin10x" }
```

## 用法

最小用法：解析一份**显式标注**的合成样本，读回结构化信封。

```rust
use jin10x::{parse_jin10_envelopes, DataKind};

let input = r#"{
  "_synthetic": true,
  "_note": "合成样本，非真实源数据",
  "envelopes": [
    {
      "msg_id": "n-1",
      "source": "jin10",
      "kind": "Flash",
      "event_time": "2026-09-18T09:31:05",
      "collect_time": "2026-09-18T09:31:07",
      "payload": { "importance": 3, "channels": ["macro"], "content": "合成", "is_data": false }
    }
  ]
}"#;

let envelopes = parse_jin10_envelopes(input)?;
assert_eq!(envelopes[0].kind, DataKind::Flash);
# Ok::<(), jin10x::Jin10Error>(())
```

判定「能否提议进入宏观观测」（**提议**，不是写入）：

```rust
use jin10x::{propose_macro_mapping, DataKind, Frequency, Jin10MacroMappingRequest, Jin10Unit, Period};

let request = Jin10MacroMappingRequest {
    msg_id: "n-2".to_owned(),
    kind: DataKind::Calendar,
    indicator: Some("CPI YoY".to_owned()),
    subject: Some("US".to_owned()),
    period: Some(Period::Month { year: 2026, month: 8 }),
    value: Some(3.1),
    unit: Jin10Unit::Percent,
    frequency: Frequency::Monthly,
    arrival_seq: 1,
};
let proposal = propose_macro_mapping(&request)?;
assert_eq!(proposal.revision(), None);
# Ok::<(), jin10x::Jin10Error>(())
```

## 能力矩阵

| 能力 | 生产入口 / 类型 | 边界 |
| --- | --- | --- |
| 信封与数据类别 | `Jin10Envelope<T>` / `DataKind` | 八类固定；字段集固定；不增不减 |
| 规划接口常量 | `INTERFACE_PLANS`（S01–S08） | **规划语义**，不是访问合同 |
| 三类载荷 | `FlashNews` / `CalendarEvent` / `QuoteSnapshot` | 其余五类载荷未由清单固定，返回 `NotApplicable` |
| 离线解析 | `parse_jin10_envelopes` | 只接受显式标注的合成样本；未知字段原子失败；重复 `msg_id` 拒绝 |
| 映射守卫 | `propose_macro_mapping` | 三要素齐备才提议；`Quote` → `RoutedElsewhere`；拍卖/RRP → `WriteAuthorityDenied` |
| 乱序保护 | `decide_macro_write` | 乱序不覆盖；更晚但不同值 → `Conflict`，不静默覆盖 |
| 授权判定 | `authorize` / `ensure_authorized` | 证据缺失 / 范围不明 / 签署者不明 / 未签核 / 未生效 / 已过期 → 一律拒绝 |
| publication | `publication_semantics` | 恒 `Date` + `Inferred` + `NotEligible` |

## 解析与拒绝口径

| 情形 | 结果 |
| --- | --- |
| `_synthetic` 缺失或为 `false`、`_note` 为空 | `SemanticallyRejected` |
| 根对象 / 信封 / 载荷出现未知字段 | `Invalid`（原子失败） |
| 缺必需字段（含 `msg_id` 为空串） | `Missing` |
| 日期或时间戳形态非法 | `Invalid` |
| `source` 不等于 `jin10` | `SemanticallyRejected` |
| 同一 `msg_id` 出现两次 | `SemanticallyRejected`（**拒绝**，不去重） |
| `raw_ref` 内嵌 `://` | `SemanticallyRejected` |
| 五类未固定载荷的 `DataKind` | `NotApplicable` |

错误消息**只给形态与位置**，不回显原始正文、取值片段或凭据。

## 跨域守卫

- **Quote 默认拒绝**：行情类不得自动进入宏观观测（`MD-1-R04` 精神）。
- **拍卖 / RRP**：与 `treasuryx` 重叠，写入主权登记为 `pending`（未决）。
  本库**保持拒绝**并把 `pending` 如实登记，**不自行裁定**归属。

## 非目标

不做联网采集与传输层；不猜端点、Header、限流数字或统计码全集；不做存储与再分发；
不做单位换算；不算派生指标（净流动性 / 利差 / Credit Impulse / z-score）；
不宣称 package stable、SLA 或新鲜度保证。

## 门禁

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo package --no-verify
```

`production_decision = NO-GO`。清单 COMPLETE ≠ ship；authorization ≠ Production Ready。

## 许可

MIT OR Apache-2.0
