//! 宏观观测的**提议**与跨源写入守卫。
//!
//! 本模块落两条硬规则（`cross-source-routing.md` §3 / §7 的 `jin10x` 行）：
//!
//! 1. **映射规则**：仅当消息**显式提供可验证 indicator + 主体 + 业务期间**时，
//!    才可**提议**映射 `domain_macro`；乱序**不得**覆盖既有事实。
//! 2. **Quote 默认拒绝**：`Quote` 类一律不得自动进入宏观观测（`MD-1-R04` 精神）。
//!    拍卖 / RRP（`Treasury`）与 `treasuryx` 重叠，写入主权 **`pending`（未决）**，
//!    本库**保持拒绝**并把 `pending` 如实登记，**不得**自行裁定归谁。
//!
//! 「提议」不是「写入」：本模块不写任何存储，也不改变任何授权或裁决。

use super::{Frequency, Jin10Unit, Period};

/// 拍卖 / RRP 的写入主权登记值：**未决**，双方均不得主张权威写入。
pub const PENDING_WRITE_SOVEREIGNTY_NOTE: &str =
    "拍卖/RRP 与 treasuryx 重叠，写入主权 pending（未决）；双方均不得主张权威写入；争议由 Owner 另裁";

/// 某类别的写入主权登记（只读结论，**不改变**授权状态）。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Jin10WriteSovereignty {
    /// 本库可提议写入宏观观测（仍需三要素齐备）。
    Own,
    /// 主权未决：与 `treasuryx` 重叠，本库**保持拒绝**。
    Pending,
    /// 不属于本域：行情类商品归行情域。
    Routed,
}

/// 查询某数据类别的写入主权登记。
#[must_use]
pub fn write_sovereignty(kind: super::DataKind) -> Jin10WriteSovereignty {
    if kind.is_quote() {
        Jin10WriteSovereignty::Routed
    } else if kind.overlaps_treasury_write() {
        Jin10WriteSovereignty::Pending
    } else {
        Jin10WriteSovereignty::Own
    }
}

/// 提议宏观映射的输入。
///
/// 三要素（`indicator` / `subject` / `period`）为 `Option`，**正是**为了让「缺失」
/// 成为一种可判定的结果，而不是被默认值掩盖。
#[derive(Debug, Clone, PartialEq)]
pub struct Jin10MacroMappingRequest {
    /// 信封的消息标识（用于溯源）。
    pub msg_id: String,
    /// 数据类别。
    pub kind: super::DataKind,
    /// 可验证 indicator；缺省视为不可映射。
    pub indicator: Option<String>,
    /// 主体（国家 / 机构 / 标的）；缺省视为不可映射。
    pub subject: Option<String>,
    /// 业务期间；缺省视为不可映射。
    pub period: Option<Period>,
    /// 值；源未给出时保持 `None`，**不得**静默转 0。
    pub value: Option<f64>,
    /// 源侧单位。
    pub unit: Jin10Unit,
    /// 频率。
    pub frequency: Frequency,
    /// 采集侧的单调到达序号（由采集侧提供；本层**只比较**，不生成）。
    pub arrival_seq: u64,
}

/// 宏观观测的**提议**（不是写入）。
///
/// 满足观测值对象形状：源条目标识 + 业务期间 + 值 + 源侧单位 + 频率 + 修订标识。
#[derive(Debug, Clone, PartialEq)]
pub struct Jin10MacroProposal {
    /// 源条目标识（信封消息标识）。
    pub msg_id: String,
    /// 可验证 indicator。
    pub indicator: String,
    /// 主体。
    pub subject: String,
    /// 业务期间。
    pub period: Period,
    /// 值；源未给出时为 `None`。
    pub value: Option<f64>,
    /// 源侧单位。
    pub unit: Jin10Unit,
    /// 频率。
    pub frequency: Frequency,
    /// 采集侧到达序号。
    pub arrival_seq: u64,
}

impl Jin10MacroProposal {
    /// 修订标识。本源**无**官方 vintage 面 ⇒ **恒为 `None`**，且不得伪造。
    #[must_use]
    pub fn revision(&self) -> Option<&str> {
        None
    }
}

/// 把一条映射请求提议为宏观观测。
///
/// 判定顺序（**先跨域，再完备性**）：
///
/// 1. 写入主权为 `Routed`（`Quote`）→
///    [`RoutedElsewhere`](crate::Jin10ErrorKind::RoutedElsewhere)；
/// 2. 写入主权为 `Pending`（拍卖 / RRP）→
///    [`WriteAuthorityDenied`](crate::Jin10ErrorKind::WriteAuthorityDenied)；
/// 3. 三要素缺失 → [`Missing`](crate::Jin10ErrorKind::Missing)；
/// 4. 三要素为空串 / 全空白 → [`SemanticallyRejected`](crate::Jin10ErrorKind::SemanticallyRejected)；
/// 5. 期间非法或值为非有限数 → [`Invalid`](crate::Jin10ErrorKind::Invalid)。
///
/// # Errors
///
/// 见上表五类；本函数**不**返回 `Ok` 以外的「部分接受」。
pub fn propose_macro_mapping(
    request: &Jin10MacroMappingRequest,
) -> Result<Jin10MacroProposal, crate::Jin10Error> {
    match write_sovereignty(request.kind) {
        Jin10WriteSovereignty::Routed => {
            return Err(crate::Jin10Error::RoutedElsewhere(format!(
                "{} 属行情域，默认不进宏观观测",
                request.kind.as_str()
            )));
        }
        Jin10WriteSovereignty::Pending => {
            return Err(crate::Jin10Error::WriteAuthorityDenied(format!(
                "{}：{PENDING_WRITE_SOVEREIGNTY_NOTE}",
                request.kind.as_str()
            )));
        }
        Jin10WriteSovereignty::Own => {}
    }

    let indicator = require_text(request.indicator.as_deref(), "indicator")?;
    let subject = require_text(request.subject.as_deref(), "subject")?;
    let period = request
        .period
        .ok_or_else(|| crate::Jin10Error::Missing("period（业务期间）".to_owned()))?;
    period.validate()?;
    if request.value.is_some_and(|value| !value.is_finite()) {
        return Err(crate::Jin10Error::Invalid(
            "宏观映射值须为有限数值，缺失须保持 None".to_owned(),
        ));
    }

    Ok(Jin10MacroProposal {
        msg_id: request.msg_id.clone(),
        indicator,
        subject,
        period,
        value: request.value,
        unit: request.unit,
        frequency: request.frequency,
        arrival_seq: request.arrival_seq,
    })
}

/// 必需文本项：缺失 → `Missing`；空白 → `SemanticallyRejected`。
fn require_text(value: Option<&str>, label: &str) -> Result<String, crate::Jin10Error> {
    match value {
        None => Err(crate::Jin10Error::Missing(format!(
            "{label}（映射三要素之一）"
        ))),
        Some(text) if text.trim().is_empty() => Err(crate::Jin10Error::SemanticallyRejected(
            format!("{label} 为空或全空白，不构成可验证要素"),
        )),
        Some(text) => Ok(text.to_owned()),
    }
}

/// 已接受的宏观事实（只读快照，供乱序判定）。
#[derive(Debug, Clone, PartialEq)]
pub struct Jin10AcceptedFact {
    /// 已接受事实的来源消息标识。
    pub msg_id: String,
    /// indicator。
    pub indicator: String,
    /// 主体。
    pub subject: String,
    /// 业务期间。
    pub period: Period,
    /// 值。
    pub value: Option<f64>,
    /// 该事实的到达序号。
    pub arrival_seq: u64,
}

/// 写入决策（**决策**，不是写入动作）。
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Jin10WriteDecision {
    /// 拒绝：候选或既有事实含非有限数值，不得参与写入或幂等判定。
    InvalidRejected,
    /// 接受为新的既有事实。
    Accept,
    /// 拒绝：候选更旧（乱序），**不得**覆盖既有事实。
    StaleRejected {
        /// 既有事实的消息标识。
        existing_msg_id: String,
    },
    /// 幂等：同身份同值重复到达，忽略。
    Duplicate,
    /// 同身份同期间但取值不同：需显式修订流程，**不得**静默覆盖。
    Conflict {
        /// 既有事实的消息标识。
        existing_msg_id: String,
    },
}

/// 乱序保护判定。
///
/// 判定口径：
///
/// - 候选或既有事实含非有限数值 → [`InvalidRejected`](Jin10WriteDecision::InvalidRejected)；
/// - 无既有事实 → [`Accept`](Jin10WriteDecision::Accept)；
/// - 候选 `arrival_seq` **小于**既有事实 → [`StaleRejected`](Jin10WriteDecision::StaleRejected)
///   （乱序，不得覆盖）；
/// - `arrival_seq` 相等且取值相同 → [`Duplicate`](Jin10WriteDecision::Duplicate)；
/// - `arrival_seq` 更新且取值相同 → `Duplicate`（幂等）；
/// - `arrival_seq` 更新但取值不同 → [`Conflict`](Jin10WriteDecision::Conflict)
///   （**不**静默覆盖，需要显式修订流程）。
#[must_use]
pub fn decide_macro_write(
    existing: Option<&Jin10AcceptedFact>,
    candidate: &Jin10MacroProposal,
) -> Jin10WriteDecision {
    if candidate.value.is_some_and(|value| !value.is_finite())
        || existing.is_some_and(|fact| fact.value.is_some_and(|value| !value.is_finite()))
    {
        return Jin10WriteDecision::InvalidRejected;
    }
    let Some(fact) = existing else {
        return Jin10WriteDecision::Accept;
    };
    if candidate.arrival_seq < fact.arrival_seq {
        return Jin10WriteDecision::StaleRejected {
            existing_msg_id: fact.msg_id.clone(),
        };
    }
    if candidate.value == fact.value {
        return Jin10WriteDecision::Duplicate;
    }
    Jin10WriteDecision::Conflict {
        existing_msg_id: fact.msg_id.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        decide_macro_write, propose_macro_mapping, write_sovereignty, Jin10AcceptedFact,
        Jin10MacroMappingRequest, Jin10MacroProposal, Jin10WriteDecision, Jin10WriteSovereignty,
        PENDING_WRITE_SOVEREIGNTY_NOTE,
    };
    use crate::value::{DataKind, Frequency, Jin10Unit, Period};
    use crate::Jin10ErrorKind;

    fn request(kind: DataKind) -> Jin10MacroMappingRequest {
        Jin10MacroMappingRequest {
            msg_id: "m-1".to_owned(),
            kind,
            indicator: Some("CPI YoY".to_owned()),
            subject: Some("US".to_owned()),
            period: Some(Period::Month {
                year: 2026,
                month: 8,
            }),
            value: Some(3.1),
            unit: Jin10Unit::Percent,
            frequency: Frequency::Monthly,
            arrival_seq: 10,
        }
    }

    #[test]
    fn sovereignty_table_is_registered_as_read_only_decision() {
        assert_eq!(
            write_sovereignty(DataKind::Quote),
            Jin10WriteSovereignty::Routed
        );
        assert_eq!(
            write_sovereignty(DataKind::Treasury),
            Jin10WriteSovereignty::Pending
        );
        assert_eq!(
            write_sovereignty(DataKind::Calendar),
            Jin10WriteSovereignty::Own
        );
        assert!(PENDING_WRITE_SOVEREIGNTY_NOTE.contains("pending"));
    }

    #[test]
    fn quote_is_routed_elsewhere_even_with_all_three_elements() {
        let error = propose_macro_mapping(&request(DataKind::Quote)).expect_err("Quote 应被拒绝");
        assert_eq!(error.kind(), Jin10ErrorKind::RoutedElsewhere);
    }

    #[test]
    fn treasury_stays_denied_and_pending() {
        let error =
            propose_macro_mapping(&request(DataKind::Treasury)).expect_err("拍卖类应保持拒绝");
        assert_eq!(error.kind(), Jin10ErrorKind::WriteAuthorityDenied);
    }

    #[test]
    fn missing_three_elements_are_reported_per_element() {
        let mut input = request(DataKind::Calendar);
        input.indicator = None;
        assert_eq!(
            propose_macro_mapping(&input)
                .expect_err("缺 indicator")
                .kind(),
            Jin10ErrorKind::Missing
        );
        input.indicator = Some("CPI YoY".to_owned());
        input.subject = None;
        assert_eq!(
            propose_macro_mapping(&input)
                .expect_err("缺 subject")
                .kind(),
            Jin10ErrorKind::Missing
        );
        input.subject = Some("US".to_owned());
        input.period = None;
        assert_eq!(
            propose_macro_mapping(&input).expect_err("缺 period").kind(),
            Jin10ErrorKind::Missing
        );
    }

    #[test]
    fn blank_elements_are_semantically_rejected() {
        let mut input = request(DataKind::Calendar);
        input.subject = Some("   ".to_owned());
        assert_eq!(
            propose_macro_mapping(&input).expect_err("空白主体").kind(),
            Jin10ErrorKind::SemanticallyRejected
        );
    }

    #[test]
    fn out_of_order_candidate_must_not_overwrite() {
        let candidate = propose_macro_mapping(&request(DataKind::Calendar)).expect("三要素齐备");
        let existing = Jin10AcceptedFact {
            msg_id: "m-0".to_owned(),
            indicator: candidate.indicator.clone(),
            subject: candidate.subject.clone(),
            period: candidate.period,
            value: Some(2.9),
            arrival_seq: 11,
        };
        assert_eq!(
            decide_macro_write(Some(&existing), &candidate),
            Jin10WriteDecision::StaleRejected {
                existing_msg_id: "m-0".to_owned()
            }
        );
    }

    #[test]
    fn newer_arrival_with_different_value_is_a_conflict_not_an_overwrite() {
        let candidate = propose_macro_mapping(&request(DataKind::Calendar)).expect("三要素齐备");
        let existing = Jin10AcceptedFact {
            msg_id: "m-0".to_owned(),
            indicator: candidate.indicator.clone(),
            subject: candidate.subject.clone(),
            period: candidate.period,
            value: Some(2.9),
            arrival_seq: 9,
        };
        assert_eq!(
            decide_macro_write(Some(&existing), &candidate),
            Jin10WriteDecision::Conflict {
                existing_msg_id: "m-0".to_owned()
            }
        );
    }

    #[test]
    fn identical_value_is_idempotent_and_no_fact_is_an_accept() {
        let candidate = propose_macro_mapping(&request(DataKind::Calendar)).expect("三要素齐备");
        assert_eq!(
            decide_macro_write(None, &candidate),
            Jin10WriteDecision::Accept
        );
        let same: Jin10MacroProposal = candidate.clone();
        let existing = Jin10AcceptedFact {
            msg_id: "m-0".to_owned(),
            indicator: candidate.indicator.clone(),
            subject: candidate.subject.clone(),
            period: candidate.period,
            value: same.value,
            arrival_seq: 9,
        };
        assert_eq!(
            decide_macro_write(Some(&existing), &candidate),
            Jin10WriteDecision::Duplicate
        );
    }

    #[test]
    fn revision_surface_is_absent_and_not_faked() {
        let candidate = propose_macro_mapping(&request(DataKind::Calendar)).expect("三要素齐备");
        assert_eq!(candidate.revision(), None);
    }
}
