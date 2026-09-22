//! jin10x 的离线解析器：字符串 → 信封集合。
//!
//! **输入形态**：单个 JSON 文档（见 `docs/标准.md` 与 `tests/fixtures/`）。
//!
//! 本模块**没有**网络参数：不接受 URL、HTTP 客户端、认证信息、环境变量或任何代理配置。
//!
//! 三条硬约束：
//!
//! 1. **只接受显式标注的合成样本**：根对象须含 `_synthetic: true` 与非空 `_note`。
//!    本层无任何采集授权，不接受未标注或自称真实的文件。
//! 2. **未知字段原子失败**：根对象、信封与载荷三层都拒绝未知字段，不做静默忽略。
//! 3. **重复身份拒绝**（**不**去重）：同一 `msg_id` 出现两次即整体失败。
//!
//! 另：五类载荷字段未被清单固定的 `DataKind`（`BigEvent` / `CentralBank` / `Treasury` /
//! `Cot` / `EtfInventory`）返回
//! [`NotApplicable`](crate::Jin10ErrorKind::NotApplicable) —— **不编造字段**。

use serde::Deserialize;

use crate::error::{Jin10Error, Jin10Result};
use crate::value::{
    CalendarEvent, DataKind, FlashNews, Jin10Envelope, Jin10Importance, Jin10Payload,
    Jin10Timestamp, QuoteSnapshot,
};

/// 解析一份 jin10 信封 JSON 文档。
///
/// # Errors
///
/// - 语法非法 / 结构非法 / 未知字段 / 缺必需字段 → [`Jin10Error::Invalid`]；
/// - 缺少 `_synthetic` 标注 → [`Jin10Error::SemanticallyRejected`]；
/// - `msg_id` 重复 → [`Jin10Error::SemanticallyRejected`]；
/// - 载荷字段未被清单固定的类别 → [`Jin10Error::NotApplicable`]。
pub fn parse_jin10_envelopes(input: &str) -> Jin10Result<Vec<Jin10Envelope<Jin10Payload>>> {
    let raw: RawEnvelopeFile = serde_json::from_str(input).map_err(|error| syntax_error(&error))?;
    require_synthetic_marker(raw.synthetic, raw.note.as_deref())?;

    let mut parsed: Vec<Jin10Envelope<Jin10Payload>> = Vec::with_capacity(raw.envelopes.len());
    for envelope in raw.envelopes {
        let msg_id = envelope.msg_id.clone();
        if parsed.iter().any(|seen| seen.msg_id == msg_id) {
            return Err(Jin10Error::SemanticallyRejected(format!(
                "msg_id 重复：{msg_id}"
            )));
        }
        parsed.push(convert_envelope(envelope)?);
    }
    Ok(parsed)
}

/// 校验「显式合成标注」。
fn require_synthetic_marker(synthetic: Option<bool>, note: Option<&str>) -> Jin10Result<()> {
    match synthetic {
        Some(true) => {}
        Some(false) => {
            return Err(Jin10Error::SemanticallyRejected(
                "文件声明 _synthetic=false：本层只接受显式标注的合成样本".to_owned(),
            ));
        }
        None => {
            return Err(Jin10Error::SemanticallyRejected(
                "文件缺少 _synthetic 标注：本层只接受显式标注的合成样本".to_owned(),
            ));
        }
    }
    if note.unwrap_or("").trim().is_empty() {
        return Err(Jin10Error::SemanticallyRejected(
            "文件缺少 _note 说明：合成样本须自带可直接阅读的说明".to_owned(),
        ));
    }
    Ok(())
}

/// 单条信封的转换。
fn convert_envelope(raw: RawEnvelope) -> Jin10Result<Jin10Envelope<Jin10Payload>> {
    let kind = DataKind::parse(&raw.kind)?;
    let event_time = Jin10Timestamp::parse(&raw.event_time)?;
    let collect_time = Jin10Timestamp::parse(&raw.collect_time)?;
    if let Some(reference) = raw.raw_ref.as_deref() {
        if reference.contains("://") {
            return Err(Jin10Error::SemanticallyRejected(
                "raw_ref 须为原始报文引用，不得为 URL".to_owned(),
            ));
        }
    }
    let payload = convert_payload(kind, raw.payload)?;
    let envelope = Jin10Envelope {
        msg_id: raw.msg_id,
        source: raw.source,
        kind,
        event_time,
        collect_time,
        payload,
        raw_ref: raw.raw_ref,
    };
    crate::value::validate_envelope(&envelope)?;
    Ok(envelope)
}

/// 载荷分派：清单未固定字段的五类一律 `NotApplicable`。
fn convert_payload(kind: DataKind, payload: serde_json::Value) -> Jin10Result<Jin10Payload> {
    match kind {
        DataKind::Flash => {
            let raw: RawFlashNews =
                serde_json::from_value(payload).map_err(|_| payload_structure_error(kind))?;
            Ok(Jin10Payload::Flash(FlashNews {
                importance: Jin10Importance::new(raw.importance)?,
                channels: raw.channels,
                content: raw.content,
                is_data: raw.is_data,
            }))
        }
        DataKind::Calendar => {
            let raw: RawCalendarEvent =
                serde_json::from_value(payload).map_err(|_| payload_structure_error(kind))?;
            Ok(Jin10Payload::Calendar(CalendarEvent {
                country: raw.country,
                indicator: raw.indicator,
                previous: raw.previous,
                consensus: raw.consensus,
                actual: raw.actual,
                revised: raw.revised,
                surprise_z: raw.surprise_z,
            }))
        }
        DataKind::Quote => {
            let raw: RawQuoteSnapshot =
                serde_json::from_value(payload).map_err(|_| payload_structure_error(kind))?;
            Ok(Jin10Payload::Quote(QuoteSnapshot {
                symbol: raw.symbol,
                price: raw.price,
                change_pct: raw.change_pct,
                ts: Jin10Timestamp::parse(&raw.ts)?,
            }))
        }
        _ => Err(Jin10Error::NotApplicable(format!(
            "{} 的载荷字段未被清单固定，离线解析不实现（不得编造字段）",
            kind.as_str()
        ))),
    }
}

/// 内容无关的结构错误：**不**回显原始正文、取值片段或凭据。
fn payload_structure_error(kind: DataKind) -> Jin10Error {
    Jin10Error::Invalid(format!(
        "{} 载荷结构非法：存在未知字段、缺少必需字段或类型不符",
        kind.as_str()
    ))
}

/// 内容无关的语法错误：只给行列位置。
fn syntax_error(error: &serde_json::Error) -> Jin10Error {
    Jin10Error::Invalid(format!(
        "JSON 语法非法（第 {} 行第 {} 列）",
        error.line(),
        error.column()
    ))
}

/// 根对象。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEnvelopeFile {
    #[serde(rename = "_synthetic", default)]
    synthetic: Option<bool>,
    #[serde(rename = "_note", default)]
    note: Option<String>,
    envelopes: Vec<RawEnvelope>,
}

/// 信封原始形态。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEnvelope {
    msg_id: String,
    source: String,
    kind: String,
    event_time: String,
    collect_time: String,
    payload: serde_json::Value,
    #[serde(default)]
    raw_ref: Option<String>,
}

/// 快讯载荷原始形态。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFlashNews {
    importance: u8,
    channels: Vec<String>,
    content: String,
    is_data: bool,
}

/// 经济日历载荷原始形态。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCalendarEvent {
    country: String,
    indicator: String,
    #[serde(default)]
    previous: Option<f64>,
    #[serde(default)]
    consensus: Option<f64>,
    #[serde(default)]
    actual: Option<f64>,
    #[serde(default)]
    revised: Option<f64>,
    #[serde(default)]
    surprise_z: Option<f64>,
}

/// 行情快照载荷原始形态。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawQuoteSnapshot {
    symbol: String,
    price: f64,
    change_pct: f64,
    ts: String,
}

#[cfg(test)]
mod tests {
    use super::parse_jin10_envelopes;
    use crate::error::Jin10ErrorKind;
    use crate::value::DataKind;

    const VALID: &str = include_str!("../tests/fixtures/envelopes.json");

    fn single(kind: &str, payload: &str) -> String {
        format!(
            r#"{{
  "_synthetic": true,
  "_note": "合成样本",
  "envelopes": [
    {{
      "msg_id": "m-1",
      "source": "jin10",
      "kind": "{kind}",
      "event_time": "2026-09-18T09:31:05",
      "collect_time": "2026-09-18T09:31:07",
      "payload": {payload}
    }}
  ]
}}"#
        )
    }

    #[test]
    fn parses_the_synthetic_fixture() {
        let envelopes = parse_jin10_envelopes(VALID).expect("合成夹具应可解析");
        assert_eq!(envelopes.len(), 3);
        assert_eq!(envelopes[0].kind, DataKind::Flash);
        assert_eq!(envelopes[1].kind, DataKind::Calendar);
        assert_eq!(envelopes[2].kind, DataKind::Quote);
        assert_eq!(envelopes[0].payload.kind(), DataKind::Flash);
    }

    #[test]
    fn rejects_unknown_field_at_envelope_level() {
        let input = VALID.replace(r#""msg_id": "n-1""#, r#""msg_id": "n-1", "extra": 1"#);
        assert_eq!(
            parse_jin10_envelopes(&input)
                .expect_err("未知字段应失败")
                .kind(),
            Jin10ErrorKind::Invalid
        );
    }

    #[test]
    fn rejects_missing_required_payload_field() {
        let input = single(
            "Flash",
            r#"{"channels": [], "content": "x", "is_data": false}"#,
        );
        assert_eq!(
            parse_jin10_envelopes(&input)
                .expect_err("缺 importance")
                .kind(),
            Jin10ErrorKind::Invalid
        );
    }

    #[test]
    fn rejects_illegal_date() {
        let input = VALID.replace("2026-09-18T09:31:05", "2026-9-18T09:31:05");
        assert_eq!(
            parse_jin10_envelopes(&input).expect_err("非法日期").kind(),
            Jin10ErrorKind::Invalid
        );
    }

    #[test]
    fn rejects_duplicate_msg_id() {
        let input = VALID.replace(r#""msg_id": "n-2""#, r#""msg_id": "n-1""#);
        assert_eq!(
            parse_jin10_envelopes(&input).expect_err("重复身份").kind(),
            Jin10ErrorKind::SemanticallyRejected
        );
    }

    #[test]
    fn rejects_unmarked_file() {
        let input = VALID.replace(r#""_synthetic": true"#, r#""_synthetic": false"#);
        assert_eq!(
            parse_jin10_envelopes(&input).expect_err("自称真实").kind(),
            Jin10ErrorKind::SemanticallyRejected
        );
        let bare = VALID.replace(r#""_synthetic": true,"#, "");
        assert_eq!(
            parse_jin10_envelopes(&bare).expect_err("缺标注").kind(),
            Jin10ErrorKind::SemanticallyRejected
        );
    }

    #[test]
    fn unfixed_payload_kinds_return_not_applicable() {
        for kind in ["BigEvent", "CentralBank", "Treasury", "Cot", "EtfInventory"] {
            let input = single(kind, "{}");
            assert_eq!(
                parse_jin10_envelopes(&input).expect_err(kind).kind(),
                Jin10ErrorKind::NotApplicable
            );
        }
    }

    // raw_ref 为 URL 的负向用例见 `tests/aidd_boundary.rs`
    // （`embedded_url_in_raw_ref_is_rejected`）：该用例需要 URL 字面量，
    // 而 `src/` 内不得出现端点字面量，故不在此重复。

    #[test]
    fn error_messages_do_not_echo_payload_values() {
        let input = single(
            "Flash",
            r#"{"importance": "AKIA-SECRET-LOOKING", "channels": [], "content": "x", "is_data": false}"#,
        );
        let text = parse_jin10_envelopes(&input)
            .expect_err("类型不符")
            .to_string();
        assert!(!text.contains("AKIA-SECRET-LOOKING"));
    }

    #[test]
    fn syntax_error_reports_position_only() {
        let text = parse_jin10_envelopes("{ not json")
            .expect_err("语法非法")
            .to_string();
        assert!(text.contains("行"));
        assert!(!text.contains("not json"));
    }

    #[test]
    fn source_mismatch_is_semantically_rejected() {
        let input = VALID.replace(r#""source": "jin10""#, r#""source": "other""#);
        assert_eq!(
            parse_jin10_envelopes(&input).expect_err("源不符").kind(),
            Jin10ErrorKind::SemanticallyRejected
        );
    }
}
