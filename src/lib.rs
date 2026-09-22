#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable
    )
)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(unreachable_pub)]

//! jin10x —— 金十数据源（`source_id = jin10`）的**源事实层**：
//! 信封与数据类别类型化、离线解析、fail-closed 授权判定与跨域守卫。
//!
//! ## 能力
//!
//! | 能力 | 类型 / 入口 | 状态 |
//! |------|-------------|------|
//! | 信封与数据类别 | [`Jin10Envelope`] / [`DataKind`]（八类） | 已落地 |
//! | 规划接口常量 | [`INTERFACE_PLANS`]（S01–S08） | 已落地（**规划语义**，非访问合同） |
//! | 载荷类型 | [`FlashNews`] / [`CalendarEvent`] / [`QuoteSnapshot`] | 已落地 |
//! | 离线解析 | [`parse_jin10_envelopes`] | 已落地（仅上述三类载荷） |
//! | 宏观映射守卫 | [`propose_macro_mapping`] / [`decide_macro_write`] | 已落地 |
//! | 跨域写入守卫 | [`write_sovereignty`] | 已落地（Quote 拒绝 / 拍卖 RRP 未决） |
//! | 授权判定 | [`authorize`] / [`ensure_authorized`] | fail-closed |
//! | publication 语义 | [`publication_semantics`] | 恒 `Date + Inferred + NotEligible` |
//! | 联网采集 | — | **未实现**（无授权，`FR-037`） |
//!
//! ## 责任边界
//!
//! 本库做：把清单声明的类型、常量与禁则落成**可编译、可测、可判定**的代码；
//! 对离线字符串做严格解析；对「可否提议进入宏观观测」给出可判定的结论。
//!
//! 本库**不做**：联网采集、代理 / 会话管理、凭据注入、存储与再分发、派生指标。
//!
//! ## 非目标
//!
//! 不猜端点、Header 或限流数字；不实现 S01–S08 的传输层；不把 `Quote` 自动晋级为宏观观测；
//! 不裁定拍卖 / RRP 的写入主权（该争议为 `pending`，由 Owner 另裁）。
//!
//! ## 诚实边界
//!
//! `production_decision = NO-GO`；清单 COMPLETE ≠ ship；authorization ≠ Production Ready。
//! 本源授权为 `unknown`（无 Owner 签核文件），因此：
//!
//! - 授权判定默认拒绝（[`authorize`] 的 fail-closed 落点）；
//! - 解析器**只接受**显式标注的合成样本（`_synthetic: true` + 非空 `_note`）；
//! - 五类载荷未被清单固定的 `DataKind`（`BigEvent` / `CentralBank` / `Treasury` /
//!   `Cot` / `EtfInventory`）返回 `NotApplicable`，**不编造字段**。
//!
//! # 最小示例
//!
//! ```
//! use jin10x::{parse_jin10_envelopes, DataKind};
//!
//! let input = r#"{
//!   "_synthetic": true,
//!   "_note": "合成样本，非真实源数据",
//!   "envelopes": [
//!     {
//!       "msg_id": "n-1",
//!       "source": "jin10",
//!       "kind": "Flash",
//!       "event_time": "2026-09-18T09:31:05",
//!       "collect_time": "2026-09-18T09:31:07",
//!       "payload": { "importance": 3, "channels": ["macro"], "content": "合成", "is_data": false }
//!     }
//!   ]
//! }"#;
//! let envelopes = parse_jin10_envelopes(input)?;
//! assert_eq!(envelopes[0].kind, DataKind::Flash);
//! # Ok::<(), jin10x::Jin10Error>(())
//! ```

pub mod authz;
pub mod error;
pub mod parse;
pub mod pit;
pub mod value;

pub use authz::{
    authorize, ensure_authorized, Jin10Authorization, Jin10AuthorizationEvidence,
    AUTHORIZATION_EVIDENCE, AUTHORIZATION_STATUS,
};
pub use error::{Jin10Error, Jin10ErrorKind, Jin10Result};
pub use parse::parse_jin10_envelopes;
pub use pit::{
    is_formal_pit_eligible, publication_semantics, AvailabilityEvidence, PitEligibility,
    TimePrecision,
};
pub use value::{
    decide_macro_write, propose_macro_mapping, validate_envelope, validate_payload,
    write_sovereignty, CalendarEvent, DataKind, Date, FlashNews, Frequency, InterfaceId,
    Jin10AcceptedFact, Jin10Clock, Jin10Envelope, Jin10Importance, Jin10InterfacePlan,
    Jin10MacroMappingRequest, Jin10MacroProposal, Jin10Payload, Jin10Timestamp, Jin10Unit,
    Jin10WriteDecision, Jin10WriteSovereignty, Period, QuoteSnapshot, INTERFACE_PLANS,
    PENDING_WRITE_SOVEREIGNTY_NOTE, PLANNED_PERIODS_ARE_NOT_ACCESS_CONTRACT, SOURCE_ID,
};
