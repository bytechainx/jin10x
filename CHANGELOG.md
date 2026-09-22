# Changelog — jin10x

本文件记录 `jin10x` 的用户可见变更，遵循 [Keep a Changelog](https://keepachangelog.com/)
与 [Semantic Versioning](https://semver.org/)。

本 crate 为特性 005（数据模块独立 API 库）新建，版本从 `0.1.0` 起算。

## [Unreleased]

## [0.1.0] - 2026-09-22

### 新增

- 从 `specs/adapter/jin10.md` 的源事实新建独立 crate，`source_id = jin10`，
  零内部依赖、零网络、零凭据。
- `src/error.rs`：`Jin10ErrorKind`（8 个语义分类）、`Jin10Error` 与 `Jin10Result<T>`；
  `is_retryable` 除 `Invariant` 外恒为 `false`（本层无网络）。
- `src/value.rs`：严格 `YYYY-MM-DD` 的 `Date`（月 / 日 / 闰年校验）、`Period`、
  `Frequency`、源侧单位 `Jin10Unit`。
- `src/value/envelope.rs`：`DataKind` 八类、`Jin10Timestamp` / `Jin10Clock`、
  S01–S08 的 `INTERFACE_PLANS`（标识 + 名称 + 清单「方式」逐字文本 + **规划**周期）
  与按标识查询入口 `InterfaceId::plan`。
- `src/value/payload.rs`：`FlashNews` / `CalendarEvent` / `QuoteSnapshot`、
  `Jin10Envelope<T>` 与 `validate_envelope` / `validate_payload`。
- `src/value/mapping.rs`：`Jin10MacroMappingRequest` / `Jin10MacroProposal`、
  `propose_macro_mapping`（三要素门槛 + Quote / 拍卖 RRP 跨域拒绝）、
  `decide_macro_write`（`Accept` / `StaleRejected` / `Duplicate` / `Conflict` 乱序保护）、
  `write_sovereignty`（`Own` / `Pending` / `Routed`）。
- `src/parse.rs`：离线解析 `parse_jin10_envelopes`；未知字段三层原子失败、
  重复 `msg_id` 拒绝、只接受显式标注的合成样本、错误消息不回显正文。
- `src/authz.rs`：fail-closed 授权判定 `authorize` / `ensure_authorized`。
- `src/pit.rs`：publication 语义三元组，恒 `Date` + `Inferred` + `NotEligible`。
- `tests/{tdd_contracts,sdd_spec,aidd_boundary}.rs` 与合成夹具
  `tests/fixtures/envelopes.json`；`benches/hot_path.rs` 微基准。
- `src/` 每个模块（除门面 `lib.rs`）均带**与源码同文件**的内联单元测试
  （组织 P0：`testing.md` §1.1）。

### 变更

- **TDD-PROBE 入口列格式归一（仅测试面，公开 API 一字未改）**：按
  `specs/002-public-api-compliance-and-test-tiers/contracts/public-api-contract.md` 的
  机器解析格式，把 `tests/tdd_contracts.rs` 的入口列由「带签名」改写为
  `类型::方法` 或裸函数名（例：`parse_jin10_envelopes`、`Date::parse`）。
  该格式是 C2 精确集合比对与 `entryIsReal` 判定的共同基准；变异列 / 红列 / 绿列未改动。
- 入口表 33 行与 33 条语义变异一一对应，`/tmp` 变异副本实跑 **33/33 被捕获**
  （先跑未变异对照为绿，再逐条施加变异并断言红）。
- **内部结构改写（公开 API 不变）**：把信封、三类载荷与 `validate_*` 自
  `src/value/envelope.rs` 拆至 `src/value/payload.rs`，以腾出 `MR-STRUCT-007` 余量 ——
  拆分前该文件生产段逼近 500 行 WARN 阈值，拆分后 `envelope.rs` 366 行、`payload.rs` 166 行。

### 说明

- `production_decision = NO-GO`：本源无 Owner 签核文件，`authorization = unknown`，
  故不实现采集、不引入 HTTP 客户端、不猜端点。
- 清单未固定的五类载荷（`BigEvent` / `CentralBank` / `Treasury` / `Cot` / `EtfInventory`）
  返回 `NotApplicable`，不编造字段。
- 夹具全部为**合成样本**，不是真实源数据，不构成任何证据。
