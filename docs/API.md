# jin10x 公开 API

本文对应 `jin10x 0.1.0` 的公开消费面（`source_id = jin10`）。
本层是**离线源事实层**：无网络、无凭据、无存储。

## 公开消费面

| 能力 | API | 一行语义 |
|------|-----|----------|
| 离线解析 | `parse_jin10_envelopes(&str)` | JSON 字符串 → 信封集合；未知字段 / 重复身份 / 未标注合成样本一律原子失败 |
| 信封校验 | `validate_envelope(&Jin10Envelope<Jin10Payload>)` | 标识、源标识、时间序、载荷与 `kind` 一致性的完整性校验 |
| 载荷校验 | `validate_payload(&Jin10Payload)` | 载荷自身必填项与数值有限性校验 |
| 映射守卫 | `propose_macro_mapping(&Jin10MacroMappingRequest)` | 三要素齐备才提议；Quote 拒绝、拍卖/RRP 保持拒绝 |
| 乱序保护 | `decide_macro_write(Option<&Jin10AcceptedFact>, &Jin10MacroProposal)` | `Accept` / `StaleRejected` / `Duplicate` / `Conflict` |
| 写入主权登记 | `write_sovereignty(DataKind)` | `Own` / `Pending` / `Routed`（只读结论） |
| 授权判定 | `authorize(Option<&Jin10AuthorizationEvidence>, Date)` | fail-closed；六条拒绝路径 |
| 授权判定（`?` 友好） | `ensure_authorized(Option<&Jin10AuthorizationEvidence>, Date)` | 拒绝时返回 `AuthorizationDenied` |
| publication 语义 | `publication_semantics()` | 恒 `(Date, Inferred, NotEligible)` |
| PIT 资格 | `is_formal_pit_eligible()` | 恒 `false` |
| 错误模型 | `Jin10Error` / `Jin10ErrorKind` / `Jin10Result<T>` | 8 个语义分类 + `kind()` / `is_retryable()` |

## 值对象

| 类型 | 语义 |
|------|------|
| `Date` / `Period` | 严格 `YYYY-MM-DD` 日期与业务期间（日 / 月 / 季 / 年 / 事件）；`Date::new` / `Date::parse` / `Date::validate` / `Date::is_leap_year` |
| `Frequency` | 七档频率（daily … irregular），`as_str` / `parse` 可回读 |
| `Jin10Unit` | 源侧单位（`Unspecified` / `Percent`）；本层不换算 |
| `DataKind` | 清单固定八类；`is_quote` / `overlaps_treasury_write` 为守卫判据 |
| `Jin10Timestamp` / `Jin10Clock` | 日期必有、时刻仅当源显式给出；`Jin10Timestamp::parse` / `date_only`、`Jin10Clock::new` / `validate`，不补造 |
| `Jin10Importance` | 重要度，仅 1–5；`new` 校验、`get` 取值 |
| `Jin10Envelope<T>` | 信封：`msg_id` / `source` / `kind` / `event_time` / `collect_time` / `payload` / `raw_ref` |
| `Jin10Payload` | 已固定字段的三类载荷联合：`Flash` / `Calendar` / `Quote`；`kind()` 取载荷类别 |
| `FlashNews` / `CalendarEvent` / `QuoteSnapshot` | 三类载荷 |
| `Jin10InterfacePlan` / `INTERFACE_PLANS` / `InterfaceId::plan` | S01–S08 的标识、名称、清单「方式」逐字文本与**规划**周期；按标识查询走 `InterfaceId::plan` |
| `Jin10MacroMappingRequest` / `Jin10MacroProposal` | 映射输入与提议（提议不是写入）；`revision()` 恒为 `None`（无 vintage 面） |
| `Jin10AcceptedFact` / `Jin10WriteDecision` | 既有事实快照与写入决策 |
| `Jin10WriteSovereignty` | 写入主权登记值 |

## 选择规则

1. 解析只接受**显式标注**的合成样本（`_synthetic: true` + 非空 `_note`）。
   本层无采集授权，不接受未标注或自称真实的文件。
2. 未知字段不静默忽略：三层结构都开启了未知字段拒绝，整体失败。
3. 重复 `msg_id` 选择**拒绝**而不是去重（口径见 `docs/标准.md` §4）。
4. 想「提议进入宏观观测」只有 `propose_macro_mapping` 一条路，且三要素必须齐备；
   `Quote` 与拍卖 / RRP 在跨域阶段就被拒绝。
5. 拿到的永远是**提议**或**决策**，不是写入；本层不持有任何存储。
6. 判定授权用 `authorize` / `ensure_authorized`；本源当前无签核文件，正常结果是 `Denied`。
7. 需要 PIT 语义时用 `publication_semantics`：本层恒 `NotEligible`，不得自行升格。

## 明确不做

- 联网采集、传输层、重试与限流（无授权，`FR-037`）。
- 端点的 URL / Header / 限流数字（清单禁止猜端点）。
- 派生指标（净流动性、利差、Credit Impulse、z-score）；`surprise_z` 只是源侧字段承载。
- 单位换算、存储与再分发。
