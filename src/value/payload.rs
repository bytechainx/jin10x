//! 信封、三类载荷与它们的校验入口。
//!
//! 载荷形状来自 `specs/adapter/jin10.md` §1.2：清单**只**固定了
//! `FlashNews` / `CalendarEvent` / `QuoteSnapshot` 三类字段。
//! 其余五类（`BigEvent` / `CentralBank` / `Treasury` / `Cot` / `EtfInventory`）
//! 的字段清单未固定，故离线解析对它们返回
//! [`NotApplicable`](crate::Jin10ErrorKind::NotApplicable) —— **不编造字段**。

use super::{DataKind, Jin10Importance, Jin10Timestamp, SOURCE_ID};

/// 快讯载荷。
#[derive(Debug, Clone, PartialEq)]
pub struct FlashNews {
    /// 重要度 1–5。
    pub importance: Jin10Importance,
    /// 频道 tag（可为空）。
    pub channels: Vec<String>,
    /// 正文。
    pub content: String,
    /// 是否数据型快讯（与纯文字快讯区分）。
    pub is_data: bool,
}

/// 经济日历载荷。
///
/// `surprise_z` 是**源侧字段**：本层只承载源给出的值，**不计算** z-score
/// （派生指标归 analytics，值对象禁止实现派生公式）。
#[derive(Debug, Clone, PartialEq)]
pub struct CalendarEvent {
    /// 国家 / 地区。
    pub country: String,
    /// 指标名（**可验证 indicator** 的原始形态）。
    pub indicator: String,
    /// 前值。
    pub previous: Option<f64>,
    /// 预期值。
    pub consensus: Option<f64>,
    /// 公布值。
    pub actual: Option<f64>,
    /// 修正值。
    pub revised: Option<f64>,
    /// 源侧给出的 surprise z（源未给出时为 `None`，**不得**自行计算）。
    pub surprise_z: Option<f64>,
}

/// 行情快照载荷。
#[derive(Debug, Clone, PartialEq)]
pub struct QuoteSnapshot {
    /// 标的符号。
    pub symbol: String,
    /// 价格。
    pub price: f64,
    /// 涨跌幅（源侧口径，单位为百分比）。
    pub change_pct: f64,
    /// 源侧时间戳。
    pub ts: Jin10Timestamp,
}

/// 信封载荷：清单**已固定字段**的三类。
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum Jin10Payload {
    /// 快讯。
    Flash(FlashNews),
    /// 经济日历。
    Calendar(CalendarEvent),
    /// 行情快照。
    Quote(QuoteSnapshot),
}

impl Jin10Payload {
    /// 载荷对应的数据类别。
    #[must_use]
    pub fn kind(&self) -> DataKind {
        match self {
            Self::Flash(_) => DataKind::Flash,
            Self::Calendar(_) => DataKind::Calendar,
            Self::Quote(_) => DataKind::Quote,
        }
    }
}

/// 统一信封（清单 §1.2 的字段集）。
#[derive(Debug, Clone, PartialEq)]
pub struct Jin10Envelope<T> {
    /// 消息标识。
    pub msg_id: String,
    /// 源标识，须等于 [`SOURCE_ID`]。
    pub source: String,
    /// 数据类别。
    pub kind: DataKind,
    /// 事件时刻（源侧给出）。
    pub event_time: Jin10Timestamp,
    /// 采集时刻（源侧给出）。
    pub collect_time: Jin10Timestamp,
    /// 载荷。
    pub payload: T,
    /// 原始报文引用（**不是** URL；本层不解析、不访问）。
    pub raw_ref: Option<String>,
}

/// 校验信封：标识非空、源标识一致、时间序合理、载荷与被声明的类别一致。
///
/// # Errors
///
/// 任一不变量被破坏时返回 [`Jin10Error::Invalid`](crate::Jin10Error::Invalid) 或
/// [`Jin10Error::SemanticallyRejected`](crate::Jin10Error::SemanticallyRejected)。
pub fn validate_envelope(envelope: &Jin10Envelope<Jin10Payload>) -> Result<(), crate::Jin10Error> {
    if envelope.msg_id.trim().is_empty() {
        return Err(crate::Jin10Error::Missing("msg_id".to_owned()));
    }
    if envelope.source != SOURCE_ID {
        return Err(crate::Jin10Error::SemanticallyRejected(format!(
            "source 须为 {SOURCE_ID:?}，收到 {:?}",
            envelope.source
        )));
    }
    if envelope.collect_time < envelope.event_time {
        return Err(crate::Jin10Error::Invalid(
            "collect_time 早于 event_time".to_owned(),
        ));
    }
    if envelope.payload.kind() != envelope.kind {
        return Err(crate::Jin10Error::SemanticallyRejected(format!(
            "载荷类别 {} 与声明的 kind {} 不一致",
            envelope.payload.kind().as_str(),
            envelope.kind.as_str()
        )));
    }
    validate_payload(&envelope.payload)
}

/// 校验载荷自身约束。
///
/// # Errors
///
/// 载荷字段非法时返回 [`Jin10Error::Invalid`](crate::Jin10Error::Invalid)。
pub fn validate_payload(payload: &Jin10Payload) -> Result<(), crate::Jin10Error> {
    match payload {
        Jin10Payload::Flash(news) => {
            if news.content.trim().is_empty() {
                return Err(crate::Jin10Error::Missing("content".to_owned()));
            }
        }
        Jin10Payload::Calendar(event) => {
            if event.country.trim().is_empty() {
                return Err(crate::Jin10Error::Missing("country".to_owned()));
            }
            if event.indicator.trim().is_empty() {
                return Err(crate::Jin10Error::Missing("indicator".to_owned()));
            }
        }
        Jin10Payload::Quote(quote) => {
            if quote.symbol.trim().is_empty() {
                return Err(crate::Jin10Error::Missing("symbol".to_owned()));
            }
            if !quote.price.is_finite() || !quote.change_pct.is_finite() {
                return Err(crate::Jin10Error::Invalid(
                    "price 与 change_pct 须为有限数值".to_owned(),
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        validate_envelope, validate_payload, DataKind, FlashNews, Jin10Envelope, Jin10Importance,
        Jin10Payload, Jin10Timestamp, QuoteSnapshot, SOURCE_ID,
    };
    use crate::error::Jin10ErrorKind;
    use crate::value::Date;

    fn stamp(text: &str) -> Jin10Timestamp {
        Jin10Timestamp::parse(text).expect("测试时间戳合法")
    }

    fn quote(price: f64) -> Jin10Payload {
        Jin10Payload::Quote(QuoteSnapshot {
            symbol: "XAU".to_owned(),
            price,
            change_pct: 0.1,
            ts: stamp("2026-09-18T09:32:01"),
        })
    }

    #[test]
    fn envelope_requires_matching_source_and_kind() {
        let mut envelope = Jin10Envelope {
            msg_id: "m-1".to_owned(),
            source: SOURCE_ID.to_owned(),
            kind: DataKind::Quote,
            event_time: stamp("2026-09-18T09:32:01"),
            collect_time: stamp("2026-09-18T09:32:02"),
            payload: quote(2500.0),
            raw_ref: None,
        };
        assert!(validate_envelope(&envelope).is_ok());
        envelope.source = "other".to_owned();
        assert_eq!(
            validate_envelope(&envelope).expect_err("源不符").kind(),
            Jin10ErrorKind::SemanticallyRejected
        );
    }

    #[test]
    fn envelope_rejects_time_travel_and_missing_id() {
        let mut envelope = Jin10Envelope {
            msg_id: String::new(),
            source: SOURCE_ID.to_owned(),
            kind: DataKind::Quote,
            event_time: stamp("2026-09-18T09:32:01"),
            collect_time: stamp("2026-09-18T09:32:02"),
            payload: quote(2500.0),
            raw_ref: None,
        };
        assert_eq!(
            validate_envelope(&envelope).expect_err("空 msg_id").kind(),
            Jin10ErrorKind::Missing
        );
        envelope.msg_id = "m-1".to_owned();
        envelope.collect_time = stamp("2026-09-18T09:32:00");
        assert_eq!(
            validate_envelope(&envelope)
                .expect_err("采集早于事件")
                .kind(),
            Jin10ErrorKind::Invalid
        );
    }

    #[test]
    fn payload_requires_content_and_finite_numbers() {
        let empty = Jin10Payload::Flash(FlashNews {
            importance: Jin10Importance::new(1).expect("1–5"),
            channels: Vec::new(),
            content: "  ".to_owned(),
            is_data: false,
        });
        assert_eq!(
            validate_payload(&empty).expect_err("空正文").kind(),
            Jin10ErrorKind::Missing
        );
        assert_eq!(
            validate_payload(&quote(f64::NAN)).expect_err("NaN").kind(),
            Jin10ErrorKind::Invalid
        );
    }

    #[test]
    fn timestamp_is_only_expanded_when_the_source_gives_a_clock() {
        assert_eq!(
            Jin10Timestamp::parse("2026-09-18").expect("日粒度").clock,
            None
        );
        assert_eq!(
            Jin10Timestamp::date_only(Date::parse("2026-09-18").expect("日期")).clock,
            None
        );
    }
}
