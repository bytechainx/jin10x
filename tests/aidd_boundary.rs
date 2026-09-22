#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)]
//! AIDD 对抗 / 边界用例（特性 005）。
//!
//! 候选由 AI 生成，逐条人工复核后仅保留「结论=保留」项；丢弃项登记于 PR 描述。
//!
//! // AIDD: event_time 与 collect_time 同刻 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §2 时间序合理 | 结论=保留
//! // AIDD: 闰年 2000 / 平年 1900 的二月 29 日 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §2 日按月份与闰年 | 结论=保留
//! // AIDD: importance 取上下界 0 与 6 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §4 载荷必需字段 | 结论=保留
//! // AIDD: msg_id 为空串 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §4 缺必需字段失败 | 结论=保留
//! // AIDD: kind 使用小写变体 flash | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §2 数据类别八类 | 结论=保留
//! // AIDD: price 取 NaN 与 INFINITY | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §4 原子失败 | 结论=保留
//! // AIDD: 三要素中的主体为纯空白 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §6 三要素门槛 | 结论=保留
//! // AIDD: 同身份同值且到达序号相同的重复消息 | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §6 幂等处理 | 结论=保留
//! // AIDD: raw_ref 内嵌完整 URL | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §4 原子失败 | 结论=保留
//! // AIDD: 十万字符的 msg_id | 来源=AI | 复核=ZoneCNH/2026-09-22 | 依据=标准.md §4 解析不得 panic | 结论=保留

use jin10x::{
    decide_macro_write, parse_jin10_envelopes, propose_macro_mapping, validate_payload, DataKind,
    Date, Frequency, Jin10AcceptedFact, Jin10ErrorKind, Jin10Payload, Jin10Timestamp, Jin10Unit,
    Jin10WriteDecision, Period, QuoteSnapshot,
};

const FIXTURE: &str = include_str!("fixtures/envelopes.json");

fn stamp(text: &str) -> Jin10Timestamp {
    Jin10Timestamp::parse(text).expect("测试时间戳合法")
}

/// 边界：同一时刻采集与事件（`collect_time == event_time`）是合法的，不得被误判为乱序。
#[test]
fn equal_event_and_collect_time_is_accepted() {
    let input = FIXTURE.replace(
        r#""collect_time": "2026-09-18T09:31:07""#,
        r#""collect_time": "2026-09-18T09:31:05""#,
    );
    let parsed = parse_jin10_envelopes(&input).expect("同刻合法");
    assert_eq!(parsed[0].event_time, parsed[0].collect_time);
}

/// 边界：闰年规则必须是公历规则，不能只按「4 年一闰」。
#[test]
fn leap_year_edge_years() {
    assert!(Date::is_leap_year(2000), "400 年一闰");
    assert!(!Date::is_leap_year(1900), "100 年不闰");
    assert!(Date::new(2000, 2, 29).is_ok());
    assert_eq!(
        Date::new(1900, 2, 29).expect_err("1900 非闰年").kind(),
        Jin10ErrorKind::Invalid
    );
}

/// 边界：`importance` 的上下界之外一律拒绝（0 与 6）。
#[test]
fn importance_bounds_are_closed() {
    for value in [0u8, 6, 255] {
        assert_eq!(
            jin10x::Jin10Importance::new(value)
                .expect_err("越界")
                .kind(),
            Jin10ErrorKind::Invalid
        );
    }
    assert!(jin10x::Jin10Importance::new(1).is_ok());
    assert!(jin10x::Jin10Importance::new(5).is_ok());
}

/// 边界：`msg_id` 为空串属缺必需项，整体失败。
#[test]
fn empty_msg_id_is_rejected() {
    let input = FIXTURE.replace(r#""msg_id": "n-1""#, r#""msg_id": """#);
    assert_eq!(
        parse_jin10_envelopes(&input).expect_err("空 msg_id").kind(),
        Jin10ErrorKind::Missing
    );
}

/// 边界：`kind` 大小写敏感，小写变体不得被近义放行。
#[test]
fn lowercase_kind_is_not_aliased() {
    let input = FIXTURE.replace(r#""kind": "Flash""#, r#""kind": "flash""#);
    assert_eq!(
        parse_jin10_envelopes(&input).expect_err("小写变体").kind(),
        Jin10ErrorKind::Invalid
    );
}

/// 边界：非有限浮点不得被当作合法行情值。
#[test]
fn non_finite_prices_are_rejected() {
    for price in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let payload = Jin10Payload::Quote(QuoteSnapshot {
            symbol: "XAU".to_owned(),
            price,
            change_pct: 0.0,
            ts: stamp("2026-09-18T09:32:01"),
        });
        assert_eq!(
            validate_payload(&payload).expect_err("非有限值").kind(),
            Jin10ErrorKind::Invalid
        );
    }
}

/// 边界：三要素中的「纯空白」不得被当作已提供。
#[test]
fn whitespace_only_subject_is_not_provided() {
    let request = jin10x::Jin10MacroMappingRequest {
        msg_id: "m-1".to_owned(),
        kind: DataKind::Calendar,
        indicator: Some("CPI YoY".to_owned()),
        subject: Some("\t \n".to_owned()),
        period: Some(Period::Month {
            year: 2026,
            month: 8,
        }),
        value: Some(3.1),
        unit: Jin10Unit::Percent,
        frequency: Frequency::Monthly,
        arrival_seq: 1,
    };
    assert_eq!(
        propose_macro_mapping(&request)
            .expect_err("空白主体")
            .kind(),
        Jin10ErrorKind::SemanticallyRejected
    );
}

/// 边界：同身份同值且到达序号相同的重复消息按幂等处理，不产生第二次写入。
#[test]
fn identical_arrival_is_idempotent() {
    let request = jin10x::Jin10MacroMappingRequest {
        msg_id: "m-1".to_owned(),
        kind: DataKind::Calendar,
        indicator: Some("CPI YoY".to_owned()),
        subject: Some("US".to_owned()),
        period: Some(Period::Month {
            year: 2026,
            month: 8,
        }),
        value: Some(3.1),
        unit: Jin10Unit::Percent,
        frequency: Frequency::Monthly,
        arrival_seq: 7,
    };
    let proposal = propose_macro_mapping(&request).expect("三要素齐备");
    let existing = Jin10AcceptedFact {
        msg_id: "m-0".to_owned(),
        indicator: proposal.indicator.clone(),
        subject: proposal.subject.clone(),
        period: proposal.period,
        value: proposal.value,
        arrival_seq: proposal.arrival_seq,
    };
    assert_eq!(
        decide_macro_write(Some(&existing), &proposal),
        Jin10WriteDecision::Duplicate
    );
}

/// 边界：`raw_ref` 内嵌完整 URL 必须被拒（本层不做任何网络引用解析）。
#[test]
fn embedded_url_in_raw_ref_is_rejected() {
    let input = FIXTURE.replace(
        "jin10-raw/2026/09/18/n-3.json",
        "https://example.invalid/n-3.json",
    );
    assert_eq!(
        parse_jin10_envelopes(&input).expect_err("URL 引用").kind(),
        Jin10ErrorKind::SemanticallyRejected
    );
}

/// 边界：超长 `msg_id` 必须被正常解析或正常拒绝，绝不 panic、不失真。
#[test]
fn huge_msg_id_does_not_panic() {
    let huge = "m".repeat(100_000);
    let input = FIXTURE.replace(r#""msg_id": "n-1""#, &format!(r#""msg_id": "{huge}""#));
    let parsed = parse_jin10_envelopes(&input).expect("超长标识仍应可解析");
    assert_eq!(parsed[0].msg_id.len(), 100_000);
}

/// 日历的五个可选数值字段逐一拒绝非有限值，并保留缺失与有限极值。
#[test]
fn calendar_numeric_fields_reject_non_finite_values() {
    for field in 0..5 {
        for value in [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::MAX,
            -f64::MAX,
        ] {
            let mut fields = [None; 5];
            fields[field] = Some(value);
            let payload = Jin10Payload::Calendar(jin10x::CalendarEvent {
                country: "SYNTH_COUNTRY".into(),
                indicator: "SYNTH_INDICATOR".into(),
                previous: fields[0],
                consensus: fields[1],
                actual: fields[2],
                revised: fields[3],
                surprise_z: fields[4],
            });
            let result = validate_payload(&payload);
            if value.is_finite() {
                assert!(result.is_ok());
            } else {
                assert_eq!(
                    result.expect_err("非有限数必须拒绝").kind(),
                    Jin10ErrorKind::Invalid
                );
            }
        }
    }
}

/// 映射入口不得通过公开 Rust API 把非有限数送入宏观提议。
#[test]
fn macro_mapping_rejects_non_finite_values() {
    for value in [
        Some(f64::NAN),
        Some(f64::INFINITY),
        Some(f64::NEG_INFINITY),
        Some(f64::MAX),
        Some(-f64::MAX),
        None,
    ] {
        let request = jin10x::Jin10MacroMappingRequest {
            msg_id: "SYNTH_MSG".into(),
            kind: DataKind::Calendar,
            indicator: Some("SYNTH_INDICATOR".into()),
            subject: Some("SYNTH_SUBJECT".into()),
            period: Some(Period::Month {
                year: 2026,
                month: 1,
            }),
            value,
            unit: Jin10Unit::Unspecified,
            frequency: Frequency::Monthly,
            arrival_seq: 1,
        };
        let result = propose_macro_mapping(&request);
        if value.map_or(true, f64::is_finite) {
            assert!(result.is_ok());
        } else {
            assert_eq!(
                result.expect_err("非有限数必须拒绝").kind(),
                Jin10ErrorKind::Invalid
            );
        }
    }
}

/// 直接构造提议或既有事实不得绕过非有限值边界。
#[test]
fn write_decision_rejects_non_finite_values_before_any_branch() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut candidate = jin10x::Jin10MacroProposal {
            msg_id: "SYNTH_MSG".into(),
            indicator: "SYNTH_INDICATOR".into(),
            subject: "SYNTH_SUBJECT".into(),
            period: Period::Year(2026),
            value: Some(value),
            unit: Jin10Unit::Unspecified,
            frequency: Frequency::Annual,
            arrival_seq: 1,
        };
        assert_eq!(
            decide_macro_write(None, &candidate),
            Jin10WriteDecision::InvalidRejected
        );
        for seq in [0, 1, 2] {
            let mut existing = Jin10AcceptedFact {
                msg_id: "SYNTH_OLD".into(),
                indicator: candidate.indicator.clone(),
                subject: candidate.subject.clone(),
                period: candidate.period,
                value: Some(1.0),
                arrival_seq: seq,
            };
            assert_eq!(
                decide_macro_write(Some(&existing), &candidate),
                Jin10WriteDecision::InvalidRejected
            );
            candidate.value = Some(1.0);
            existing.value = Some(value);
            assert_eq!(
                decide_macro_write(Some(&existing), &candidate),
                Jin10WriteDecision::InvalidRejected
            );
            candidate.value = Some(value);
        }
    }
}
