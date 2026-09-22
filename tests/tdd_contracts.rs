#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)]
//! TDD 行为契约（特性 005）。
//!
//! 入口集合 = 本 crate 全部公开入口（含 `validate*` / 判定函数 / 解析器 / 值对象方法）。
//! 入口列按 `specs/002-public-api-compliance-and-test-tiers/contracts/public-api-contract.md`
//! 的机器格式书写：只写 `类型::方法` 或裸标识符（自由函数名 / 公开类型名），
//! **不带**参数、返回值与泛型 —— 它同时是该契约的登记基准，检查器按**字符串精确比对**，
//! 且要求标识符为裸形式（`entryIsReal` 只认 `fn <名>` 与
//! `pub struct|enum|trait|type <名>`）。常量类事实（如 `INTERFACE_PLANS`）由承载它的
//! 公开类型条目（`InterfaceId::plan`）覆盖；其余常量（如 `SOURCE_ID`）由
//! `tests/sdd_spec.rs` 的对应断言覆盖。
//!
//! 下表每个入口先在 `/tmp` 变异副本上观测应红、再在本树观测绿；
//! 变异为一次性探测（未提交脚本），红/绿用例名均在本文件内真实存在。
//!
//! // TDD-PROBE: parse_jin10_envelopes | 变异：不检查 msg_id 重复 | 红=parse_rejects_duplicate_msg_id | 绿=parse_rejects_duplicate_msg_id
//! // TDD-PROBE: validate_envelope | 变异：不校验 source 必须为 jin10 | 红=validate_envelope_rejects_foreign_source | 绿=validate_envelope_rejects_foreign_source
//! // TDD-PROBE: validate_payload | 变异：允许 price 为非有限值 | 红=validate_payload_rejects_non_finite_price | 绿=validate_payload_rejects_non_finite_price
//! // TDD-PROBE: propose_macro_mapping | 变异：Quote 走 Own 分支放行 | 红=propose_macro_mapping_rejects_quote | 绿=propose_macro_mapping_rejects_quote
//! // TDD-PROBE: decide_macro_write | 变异：乱序候选按 Accept 处理 | 红=decide_macro_write_rejects_stale_candidate | 绿=decide_macro_write_rejects_stale_candidate
//! // TDD-PROBE: write_sovereignty | 变异：Treasury 归为 Own | 红=write_sovereignty_registers_treasury_as_pending | 绿=write_sovereignty_registers_treasury_as_pending
//! // TDD-PROBE: authorize | 变异：过期检查失效（valid_until 不再拒绝） | 红=authorize_denies_expired_evidence | 绿=authorize_denies_expired_evidence
//! // TDD-PROBE: ensure_authorized | 变异：Denied 映射为 Invalid | 红=ensure_authorized_maps_to_authorization_denied | 绿=ensure_authorized_maps_to_authorization_denied
//! // TDD-PROBE: publication_semantics | 变异：时间精度返回 Instant | 红=publication_semantics_is_date_inferred_not_eligible | 绿=publication_semantics_is_date_inferred_not_eligible
//! // TDD-PROBE: is_formal_pit_eligible | 变异：恒返回 true | 红=formal_pit_is_never_eligible | 绿=formal_pit_is_never_eligible
//! // TDD-PROBE: Date::parse | 变异：接受 2026/09/18 分隔符 | 红=date_parse_requires_iso_separator | 绿=date_parse_requires_iso_separator
//! // TDD-PROBE: Date::new | 变异：不校验日上限 | 红=date_new_rejects_out_of_range_day | 绿=date_new_rejects_out_of_range_day
//! // TDD-PROBE: Date::validate | 变异：闰年二月按 28 天 | 红=date_validate_handles_leap_february | 绿=date_validate_handles_leap_february
//! // TDD-PROBE: Date::is_leap_year | 变异：仅按 4 年一闰判断 | 红=leap_year_follows_gregorian_rule | 绿=leap_year_follows_gregorian_rule
//! // TDD-PROBE: Period::validate | 变异：季不校验上限 | 红=period_validate_rejects_bad_quarter | 绿=period_validate_rejects_bad_quarter
//! // TDD-PROBE: Frequency::as_str | 变异：Quarterly 返回 "quarter" | 红=frequency_round_trips | 绿=frequency_round_trips
//! // TDD-PROBE: Frequency::parse | 变异：未知频率回落到 Daily | 红=frequency_parse_rejects_unknown | 绿=frequency_parse_rejects_unknown
//! // TDD-PROBE: DataKind::as_str | 变异：EtfInventory 返回 "ETFInventory" | 红=data_kind_round_trips | 绿=data_kind_round_trips
//! // TDD-PROBE: DataKind::parse | 变异：接受小写别名 flash | 红=data_kind_parse_rejects_unknown | 绿=data_kind_parse_rejects_unknown
//! // TDD-PROBE: DataKind::is_quote | 变异：所有类别都返回 true | 红=only_quote_is_quote | 绿=only_quote_is_quote
//! // TDD-PROBE: DataKind::overlaps_treasury_write | 变异：Calendar 也返回 true | 红=only_treasury_overlaps_write | 绿=only_treasury_overlaps_write
//! // TDD-PROBE: Jin10Clock::new | 变异：不校验时上限 | 红=clock_new_rejects_hour_24 | 绿=clock_new_rejects_hour_24
//! // TDD-PROBE: Jin10Clock::validate | 变异：分 / 秒上限校验失效 | 红=clock_validate_rejects_minute_60 | 绿=clock_validate_rejects_minute_60
//! // TDD-PROBE: Jin10Timestamp::parse | 变异：接受带时区偏移者 | 红=timestamp_parse_rejects_timezone_suffix | 绿=timestamp_parse_rejects_timezone_suffix
//! // TDD-PROBE: Jin10Timestamp::date_only | 变异：补造 00:00:00 时刻 | 红=timestamp_date_only_has_no_clock | 绿=timestamp_date_only_has_no_clock
//! // TDD-PROBE: Jin10Payload::kind | 变异：Quote 载荷的 kind 返回 Flash | 红=payload_kind_matches_variant | 绿=payload_kind_matches_variant
//! // TDD-PROBE: Jin10Importance::new | 变异：接受 0 与 6（越界不再拒绝） | 红=importance_rejects_zero | 绿=importance_rejects_zero
//! // TDD-PROBE: Jin10Importance::get | 变异：恒返回 0 | 红=importance_round_trips | 绿=importance_round_trips
//! // TDD-PROBE: InterfaceId::as_str | 变异：S07 返回 "S7" | 红=interface_plan_table_is_pinned | 绿=interface_plan_table_is_pinned
//! // TDD-PROBE: InterfaceId::plan | 变异：S08 周期写成 3600 | 红=interface_plan_table_is_pinned | 绿=interface_plan_table_is_pinned
//! // TDD-PROBE: Jin10MacroProposal::revision | 变异：返回 Some("v1") | 红=revision_is_absent | 绿=revision_is_absent
//! // TDD-PROBE: Jin10Error::kind | 变异：NotApplicable 映射为 Invalid | 红=error_kind_mapping_is_exact | 绿=error_kind_mapping_is_exact
//! // TDD-PROBE: Jin10Error::is_retryable | 变异：AuthorizationDenied 判为可重试 | 红=only_invariant_is_retryable_at_the_boundary | 绿=only_invariant_is_retryable_at_the_boundary

use jin10x::{
    authorize, decide_macro_write, ensure_authorized, is_formal_pit_eligible,
    parse_jin10_envelopes, propose_macro_mapping, publication_semantics, validate_envelope,
    validate_payload, write_sovereignty, AvailabilityEvidence, DataKind, Date, FlashNews,
    Frequency, InterfaceId, Jin10AcceptedFact, Jin10Authorization, Jin10AuthorizationEvidence,
    Jin10Clock, Jin10Envelope, Jin10Error, Jin10ErrorKind, Jin10Importance,
    Jin10MacroMappingRequest, Jin10MacroProposal, Jin10Payload, Jin10Timestamp, Jin10Unit,
    Jin10WriteDecision, Jin10WriteSovereignty, Period, PitEligibility, QuoteSnapshot,
    TimePrecision, INTERFACE_PLANS,
};

const FIXTURE: &str = include_str!("fixtures/envelopes.json");

fn date(text: &str) -> Date {
    Date::parse(text).expect("测试日期合法")
}

fn timestamp(text: &str) -> Jin10Timestamp {
    Jin10Timestamp::parse(text).expect("测试时间戳合法")
}

fn quote_payload() -> Jin10Payload {
    Jin10Payload::Quote(QuoteSnapshot {
        symbol: "XAU".to_owned(),
        price: 2500.0,
        change_pct: 0.42,
        ts: timestamp("2026-09-18T09:32:01"),
    })
}

fn mapping_request(kind: DataKind) -> Jin10MacroMappingRequest {
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

fn proposal() -> Jin10MacroProposal {
    propose_macro_mapping(&mapping_request(DataKind::Calendar)).expect("三要素齐备")
}

fn evidence() -> Jin10AuthorizationEvidence {
    Jin10AuthorizationEvidence {
        scope: "offline fixture parse".to_owned(),
        signer: "Owner".to_owned(),
        owner_signed: true,
        valid_from: date("2026-01-01"),
        valid_until: date("2026-12-31"),
    }
}

#[test]
fn parse_rejects_duplicate_msg_id() {
    let input = FIXTURE.replace(r#""msg_id": "n-2""#, r#""msg_id": "n-1""#);
    assert_eq!(
        parse_jin10_envelopes(&input).expect_err("重复身份").kind(),
        Jin10ErrorKind::SemanticallyRejected
    );
}

#[test]
fn validate_envelope_rejects_foreign_source() {
    let envelope = Jin10Envelope {
        msg_id: "m-1".to_owned(),
        source: "other".to_owned(),
        kind: DataKind::Quote,
        event_time: timestamp("2026-09-18T09:32:01"),
        collect_time: timestamp("2026-09-18T09:32:02"),
        payload: quote_payload(),
        raw_ref: None,
    };
    assert_eq!(
        validate_envelope(&envelope).expect_err("源不符").kind(),
        Jin10ErrorKind::SemanticallyRejected
    );
}

#[test]
fn validate_payload_rejects_non_finite_price() {
    let payload = Jin10Payload::Quote(QuoteSnapshot {
        symbol: "XAU".to_owned(),
        price: f64::NAN,
        change_pct: 0.0,
        ts: timestamp("2026-09-18T09:32:01"),
    });
    assert_eq!(
        validate_payload(&payload).expect_err("NaN").kind(),
        Jin10ErrorKind::Invalid
    );
    assert!(validate_payload(&quote_payload()).is_ok());
}

#[test]
fn propose_macro_mapping_rejects_quote() {
    assert_eq!(
        propose_macro_mapping(&mapping_request(DataKind::Quote))
            .expect_err("Quote 默认拒绝")
            .kind(),
        Jin10ErrorKind::RoutedElsewhere
    );
}

#[test]
fn decide_macro_write_rejects_stale_candidate() {
    let candidate = proposal();
    let existing = Jin10AcceptedFact {
        msg_id: "m-0".to_owned(),
        indicator: candidate.indicator.clone(),
        subject: candidate.subject.clone(),
        period: candidate.period,
        value: Some(2.9),
        arrival_seq: candidate.arrival_seq + 1,
    };
    assert_eq!(
        decide_macro_write(Some(&existing), &candidate),
        Jin10WriteDecision::StaleRejected {
            existing_msg_id: "m-0".to_owned()
        }
    );
}

#[test]
fn write_sovereignty_registers_treasury_as_pending() {
    assert_eq!(
        write_sovereignty(DataKind::Treasury),
        Jin10WriteSovereignty::Pending
    );
    assert_eq!(
        write_sovereignty(DataKind::Quote),
        Jin10WriteSovereignty::Routed
    );
    assert_eq!(write_sovereignty(DataKind::Cot), Jin10WriteSovereignty::Own);
}

#[test]
fn authorize_denies_missing_evidence() {
    assert!(matches!(
        authorize(None, date("2026-09-22")),
        Jin10Authorization::Denied { .. }
    ));
    assert_eq!(
        authorize(Some(&evidence()), date("2026-09-22")),
        Jin10Authorization::Authorized {
            scope: "offline fixture parse".to_owned()
        }
    );
}

#[test]
fn authorize_denies_expired_evidence() {
    let mut expired = evidence();
    expired.valid_until = date("2026-09-01");
    assert!(matches!(
        authorize(Some(&expired), date("2026-09-22")),
        Jin10Authorization::Denied { .. }
    ));
    let mut not_yet = evidence();
    not_yet.valid_from = date("2026-10-01");
    assert!(matches!(
        authorize(Some(&not_yet), date("2026-09-22")),
        Jin10Authorization::Denied { .. }
    ));
}

#[test]
fn ensure_authorized_maps_to_authorization_denied() {
    assert_eq!(
        ensure_authorized(None, date("2026-09-22"))
            .expect_err("证据缺失")
            .kind(),
        Jin10ErrorKind::AuthorizationDenied
    );
}

#[test]
fn publication_semantics_is_date_inferred_not_eligible() {
    assert_eq!(
        publication_semantics(),
        (
            TimePrecision::Date,
            AvailabilityEvidence::Inferred,
            PitEligibility::NotEligible
        )
    );
}

#[test]
fn formal_pit_is_never_eligible() {
    assert!(!is_formal_pit_eligible());
}

#[test]
fn date_parse_requires_iso_separator() {
    assert_eq!(date("2026-09-18").to_string(), "2026-09-18");
    for bad in ["2026/09/18", "2026-9-18", "2026-09-18T00:00:00"] {
        assert_eq!(
            Date::parse(bad).expect_err(bad).kind(),
            Jin10ErrorKind::Invalid
        );
    }
}

#[test]
fn date_new_rejects_out_of_range_day() {
    assert_eq!(
        Date::new(2026, 4, 31).expect_err("四月无 31 日").kind(),
        Jin10ErrorKind::Invalid
    );
    assert!(Date::new(2026, 4, 30).is_ok());
}

#[test]
fn date_validate_handles_leap_february() {
    assert!(Date::new(2024, 2, 29).is_ok(), "闰年二月有 29 日");
    assert!(Date::new(2023, 2, 29).is_err(), "平年二月没有 29 日");
    assert_eq!(
        Date::parse("2026-02-30").expect_err("二月无 30 日").kind(),
        Jin10ErrorKind::Invalid
    );
}

#[test]
fn leap_year_follows_gregorian_rule() {
    assert!(Date::is_leap_year(2024));
    assert!(!Date::is_leap_year(1900), "百年不闰");
    assert!(Date::is_leap_year(2000), "四百年再闰");
    assert!(!Date::is_leap_year(2023));
}

#[test]
fn period_validate_rejects_bad_quarter() {
    for bad in [
        Period::Quarter {
            year: 2026,
            quarter: 5,
        },
        Period::Month {
            year: 2026,
            month: 13,
        },
    ] {
        assert_eq!(
            bad.validate().expect_err("越界").kind(),
            Jin10ErrorKind::Invalid
        );
    }
    assert!(Period::Year(2026).validate().is_ok());
    assert!(Period::Day(date("2026-09-18")).validate().is_ok());
    assert!(Period::Event {
        date: date("2026-09-18")
    }
    .validate()
    .is_ok());
}

#[test]
fn frequency_round_trips() {
    for frequency in [
        Frequency::Daily,
        Frequency::Weekly,
        Frequency::Monthly,
        Frequency::Quarterly,
        Frequency::Annual,
        Frequency::Event,
        Frequency::Irregular,
    ] {
        assert_eq!(
            Frequency::parse(frequency.as_str()).expect("可回读"),
            frequency
        );
    }
}

#[test]
fn frequency_parse_rejects_unknown() {
    assert_eq!(
        Frequency::parse("fortnightly")
            .expect_err("未知频率")
            .kind(),
        Jin10ErrorKind::NotApplicable
    );
}

#[test]
fn data_kind_round_trips() {
    let all = [
        DataKind::Flash,
        DataKind::Calendar,
        DataKind::BigEvent,
        DataKind::CentralBank,
        DataKind::Quote,
        DataKind::Treasury,
        DataKind::Cot,
        DataKind::EtfInventory,
    ];
    assert_eq!(all.len(), 8, "八类");
    for kind in all {
        assert_eq!(DataKind::parse(kind.as_str()).expect("可回读"), kind);
    }
}

#[test]
fn data_kind_parse_rejects_unknown() {
    for bad in ["FlashNews", "flash", "FLASH", ""] {
        assert_eq!(
            DataKind::parse(bad).expect_err(bad).kind(),
            Jin10ErrorKind::Invalid
        );
    }
}

#[test]
fn only_quote_is_quote() {
    for kind in [DataKind::Flash, DataKind::Calendar, DataKind::Cot] {
        assert!(!kind.is_quote(), "{kind:?} 不是行情类");
    }
    assert!(DataKind::Quote.is_quote());
}

#[test]
fn only_treasury_overlaps_write() {
    for kind in [DataKind::Calendar, DataKind::CentralBank, DataKind::Cot] {
        assert!(
            !kind.overlaps_treasury_write(),
            "{kind:?} 不与 treasury 重叠"
        );
    }
    assert!(DataKind::Treasury.overlaps_treasury_write());
}

#[test]
fn clock_new_rejects_hour_24() {
    assert_eq!(
        Jin10Clock::new(24, 0, 0).expect_err("时越界").kind(),
        Jin10ErrorKind::Invalid
    );
    assert!(Jin10Clock::new(23, 59, 59).is_ok());
}

#[test]
fn clock_validate_rejects_minute_60() {
    assert_eq!(
        Jin10Clock::new(1, 60, 0).expect_err("分越界").kind(),
        Jin10ErrorKind::Invalid
    );
    assert_eq!(
        Jin10Clock::new(1, 0, 60).expect_err("秒越界").kind(),
        Jin10ErrorKind::Invalid
    );
    assert!(Jin10Clock::new(1, 59, 59)
        .expect("合法时刻")
        .validate()
        .is_ok());
}

#[test]
fn payload_kind_matches_variant() {
    assert_eq!(quote_payload().kind(), DataKind::Quote);
    let flash = Jin10Payload::Flash(FlashNews {
        importance: Jin10Importance::new(3).expect("1–5 合法"),
        channels: Vec::new(),
        content: "合成".to_owned(),
        is_data: false,
    });
    assert_eq!(flash.kind(), DataKind::Flash);
}

#[test]
fn timestamp_parse_rejects_timezone_suffix() {
    assert_eq!(
        Jin10Timestamp::parse("2026-09-18T09:31:05Z")
            .expect_err("带时区")
            .kind(),
        Jin10ErrorKind::Invalid
    );
    assert_eq!(
        Jin10Timestamp::parse("2026-09-1809:31:05")
            .expect_err("缺 T")
            .kind(),
        Jin10ErrorKind::Invalid
    );
}

#[test]
fn timestamp_date_only_has_no_clock() {
    let stamp = Jin10Timestamp::date_only(date("2026-09-18"));
    assert_eq!(stamp.clock, None, "不得补造时刻");
    assert_eq!(stamp.date, date("2026-09-18"));
}

#[test]
fn importance_rejects_zero() {
    assert_eq!(
        Jin10Importance::new(0).expect_err("0 越界").kind(),
        Jin10ErrorKind::Invalid
    );
    assert_eq!(
        Jin10Importance::new(6).expect_err("6 越界").kind(),
        Jin10ErrorKind::Invalid
    );
}

#[test]
fn importance_round_trips() {
    for value in 1..=5u8 {
        assert_eq!(Jin10Importance::new(value).expect("1–5 合法").get(), value);
    }
}

#[test]
fn interface_plan_table_is_pinned() {
    assert_eq!(INTERFACE_PLANS.len(), 8);
    assert_eq!(INTERFACE_PLANS[0].id.as_str(), "S01");
    assert_eq!(INTERFACE_PLANS[0].planned_period_secs, Some(5));
    assert_eq!(INTERFACE_PLANS[4].id, InterfaceId::S05);
    assert_eq!(INTERFACE_PLANS[4].planned_period_secs, Some(3));
    assert_eq!(INTERFACE_PLANS[6].id, InterfaceId::S07);
    assert_eq!(INTERFACE_PLANS[6].id.as_str(), "S07");
    assert_eq!(
        INTERFACE_PLANS[6].planned_period_secs, None,
        "每周六无秒级周期"
    );
    assert_eq!(INTERFACE_PLANS[7].planned_period_secs, Some(86400));
    assert_eq!(INTERFACE_PLANS[0].declared_transport, "WS/SSE + HTTP");
    assert_eq!(INTERFACE_PLANS[1].declared_transport, "HTTP");
    assert_eq!(INTERFACE_PLANS[7].name, "ETF/库存");
    // 按标识查询必须与常量表一致（`InterfaceId::plan`）。
    for plan in INTERFACE_PLANS {
        assert_eq!(plan.id.plan(), plan);
    }
    assert_eq!(InterfaceId::S05.plan().planned_period_secs, Some(3));
    assert_eq!(InterfaceId::S08.plan().planned_period_secs, Some(86400));
}

#[test]
fn revision_is_absent() {
    assert_eq!(proposal().revision(), None);
}

#[test]
fn error_kind_mapping_is_exact() {
    let cases: [(Jin10Error, Jin10ErrorKind); 8] = [
        (Jin10Error::Invalid("i".to_owned()), Jin10ErrorKind::Invalid),
        (Jin10Error::Missing("m".to_owned()), Jin10ErrorKind::Missing),
        (
            Jin10Error::AuthorizationDenied("a".to_owned()),
            Jin10ErrorKind::AuthorizationDenied,
        ),
        (
            Jin10Error::RoutedElsewhere("r".to_owned()),
            Jin10ErrorKind::RoutedElsewhere,
        ),
        (
            Jin10Error::WriteAuthorityDenied("w".to_owned()),
            Jin10ErrorKind::WriteAuthorityDenied,
        ),
        (
            Jin10Error::SemanticallyRejected("s".to_owned()),
            Jin10ErrorKind::SemanticallyRejected,
        ),
        (
            Jin10Error::NotApplicable("n".to_owned()),
            Jin10ErrorKind::NotApplicable,
        ),
        (
            Jin10Error::Invariant("v".to_owned()),
            Jin10ErrorKind::Invariant,
        ),
    ];
    for (error, expected) in cases {
        assert_eq!(error.kind(), expected);
    }
}

#[test]
fn only_invariant_is_retryable_at_the_boundary() {
    assert!(!Jin10Error::AuthorizationDenied("a".to_owned()).is_retryable());
    assert!(!Jin10Error::NotApplicable("n".to_owned()).is_retryable());
    assert!(Jin10Error::Invariant("v".to_owned()).is_retryable());
}
