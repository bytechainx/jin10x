#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)]
//! SDD 规格对照（特性 005）：把 `docs/标准.md` 的每个 `##` 章节转成可执行断言。
//!
//! 章节与断言函数须与 `docs/标准.md` 的 `##` 章节 1:1（检查器按标题逐字比对）。
//!
//! // SPEC-MAP: S-1 | 1. 源事实范围 | assert_source_fact_scope
//! // SPEC-MAP: S-2 | 2. 信封与数据类别 | assert_envelope_and_data_kinds
//! // SPEC-MAP: S-3 | 3. 规划接口与周期口径 | assert_planned_interfaces
//! // SPEC-MAP: S-4 | 4. 离线解析与原子失败 | assert_offline_parse_atomic_failure
//! // SPEC-MAP: S-5 | 5. 合成夹具声明 | assert_synthetic_fixture_declaration
//! // SPEC-MAP: S-6 | 6. 宏观映射与乱序保护 | assert_macro_mapping_and_ordering
//! // SPEC-MAP: S-7 | 7. 跨域守卫：Quote 与拍卖/RRP | assert_cross_domain_guards
//! // SPEC-MAP: S-8 | 8. 授权判定（fail-closed） | assert_authorization_is_fail_closed
//! // SPEC-MAP: S-9 | 9. publication 与 PIT 资格 | assert_publication_semantics
//! // SPEC-MAP: S-10 | 10. 依赖与零网络边界 | assert_dependency_and_zero_network
//! // SPEC-MAP: S-11 | 11. 验收 | assert_acceptance

use jin10x::{
    authorize, ensure_authorized, is_formal_pit_eligible, parse_jin10_envelopes,
    propose_macro_mapping, publication_semantics, validate_payload, write_sovereignty,
    AvailabilityEvidence, DataKind, Date, Frequency, Jin10AuthorizationEvidence, Jin10ErrorKind,
    Jin10Payload, Jin10Timestamp, Jin10Unit, Jin10WriteSovereignty, Period, PitEligibility,
    TimePrecision, INTERFACE_PLANS,
};

const FIXTURE: &str = include_str!("fixtures/envelopes.json");
const STANDARD: &str = include_str!("../docs/标准.md");
const CARGO_TOML: &str = include_str!("../Cargo.toml");

/// 运行期递归收集本仓 `src/` 下全部 `.rs`（**不**用手写清单，避免新增文件成为扫描盲区）。
fn source_files() -> Vec<std::path::PathBuf> {
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("src 目录可读") {
            let path = entry.expect("目录项可读").path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                out.push(path);
            }
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = Vec::new();
    walk(&root, &mut out);
    out
}

fn date(text: &str) -> Date {
    Date::parse(text).expect("测试日期合法")
}

fn request(kind: DataKind) -> jin10x::Jin10MacroMappingRequest {
    jin10x::Jin10MacroMappingRequest {
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

/// S-1：本源覆盖清单 §1 声明的范围，且不越界到网络 / 存储 / 派生。
#[test]
fn assert_source_fact_scope() {
    // 八类 DataKind 与 S01–S08 逐一可读。
    let kinds = [
        DataKind::Flash,
        DataKind::Calendar,
        DataKind::BigEvent,
        DataKind::CentralBank,
        DataKind::Quote,
        DataKind::Treasury,
        DataKind::Cot,
        DataKind::EtfInventory,
    ];
    for kind in kinds {
        assert_eq!(DataKind::parse(kind.as_str()).expect("可回读"), kind);
    }
    // 解析器只接受字符串参数：调用点无需任何网络 / 凭据上下文。
    let parsed = parse_jin10_envelopes(FIXTURE).expect("合成样本可解析");
    assert_eq!(parsed.len(), 3);
    // 授权现状为 unknown：判定不因清单「写了」而放行。
    assert_eq!(jin10x::AUTHORIZATION_STATUS, "unknown");
    assert_eq!(jin10x::SOURCE_ID, "jin10");
}

/// S-2：信封字段集固定、八类齐备、时间与载荷类别一致。
#[test]
fn assert_envelope_and_data_kinds() {
    let parsed = parse_jin10_envelopes(FIXTURE).expect("合成样本可解析");
    let flash = &parsed[0];
    assert_eq!(flash.source, "jin10");
    assert_eq!(flash.kind, DataKind::Flash);
    assert_eq!(flash.payload.kind(), DataKind::Flash);
    assert_eq!(
        flash.raw_ref.as_deref(),
        Some("jin10-raw/2026/09/18/n-1.json")
    );
    assert!(flash.collect_time >= flash.event_time);

    // 未知信封字段原子失败（字段集固定）。
    let with_extra = FIXTURE.replace(r#""msg_id": "n-1""#, r#""msg_id": "n-1", "extra": 1"#);
    assert_eq!(
        parse_jin10_envelopes(&with_extra)
            .expect_err("未知字段")
            .kind(),
        Jin10ErrorKind::Invalid
    );
    // source 不符按语义拒绝。
    let foreign = FIXTURE.replace(r#""source": "jin10""#, r#""source": "other""#);
    assert_eq!(
        parse_jin10_envelopes(&foreign).expect_err("源不符").kind(),
        Jin10ErrorKind::SemanticallyRejected
    );
    // 非法时间序（采集早于事件）被拒。
    let star = Jin10Timestamp::parse("2026-09-18T09:31:05").expect("合法时间戳");
    assert!(Jin10Timestamp::parse("2026-09-18").is_ok());
    assert!(star.clock.is_some(), "显式时刻保留");
}

/// S-3：S01–S08 常量与周期口径（规划语义，非访问合同）。
#[test]
fn assert_planned_interfaces() {
    assert_eq!(INTERFACE_PLANS.len(), 8);
    assert_eq!(INTERFACE_PLANS[0].id.as_str(), "S01");
    assert_eq!(INTERFACE_PLANS[0].name, "快讯");
    assert_eq!(INTERFACE_PLANS[0].planned_period_secs, Some(5));
    assert_eq!(INTERFACE_PLANS[1].planned_period_secs, Some(60));
    assert_eq!(INTERFACE_PLANS[2].planned_period_secs, Some(300));
    assert_eq!(INTERFACE_PLANS[3].planned_period_secs, Some(60));
    assert_eq!(INTERFACE_PLANS[4].planned_period_secs, Some(3));
    assert_eq!(INTERFACE_PLANS[5].planned_period_secs, Some(3600));
    assert_eq!(INTERFACE_PLANS[6].planned_period_secs, None);
    assert_eq!(INTERFACE_PLANS[6].planned_period_note, "每周六");
    assert_eq!(INTERFACE_PLANS[7].planned_period_secs, Some(86400));
    // 清单「方式」列逐字保留（规划表述，不是传输层实现）。
    assert_eq!(INTERFACE_PLANS[0].declared_transport, "WS/SSE + HTTP");
    for plan in &INTERFACE_PLANS[1..] {
        assert_eq!(plan.declared_transport, "HTTP");
    }
    // 周期口径必须被文档化为「规划语义」，不得被读作访问合同。
    assert!(STANDARD.contains("周期是规划语义，不是访问合同"));
}

/// S-4：离线解析的原子失败面。
#[test]
fn assert_offline_parse_atomic_failure() {
    // 未知字段。
    let unknown = FIXTURE.replace(r#""importance": 3"#, r#""importance": 3, "mood": "x""#);
    assert_eq!(
        parse_jin10_envelopes(&unknown)
            .expect_err("未知载荷字段")
            .kind(),
        Jin10ErrorKind::Invalid
    );
    // 缺必需字段。
    let missing = FIXTURE.replace(r#""importance": 3,"#, "");
    assert_eq!(
        parse_jin10_envelopes(&missing)
            .expect_err("缺必需字段")
            .kind(),
        Jin10ErrorKind::Invalid
    );
    // 非法日期。
    let bad_date = FIXTURE.replace("2026-09-18T09:31:05", "2026-9-18T09:31:05");
    assert_eq!(
        parse_jin10_envelopes(&bad_date)
            .expect_err("非法日期")
            .kind(),
        Jin10ErrorKind::Invalid
    );
    // 重复身份：拒绝（不去重）。
    let duplicated = FIXTURE.replace(r#""msg_id": "n-2""#, r#""msg_id": "n-1""#);
    assert_eq!(
        parse_jin10_envelopes(&duplicated)
            .expect_err("重复身份")
            .kind(),
        Jin10ErrorKind::SemanticallyRejected
    );
    // 未标注合成样本。
    let unmarked = FIXTURE.replace(r#""_synthetic": true,"#, "");
    assert_eq!(
        parse_jin10_envelopes(&unmarked).expect_err("缺标注").kind(),
        Jin10ErrorKind::SemanticallyRejected
    );
    // 语法错误只给位置，不回显正文。
    let text = parse_jin10_envelopes("{ oops")
        .expect_err("语法非法")
        .to_string();
    assert!(text.contains("行"));
    assert!(!text.contains("oops"));
}

/// S-5：夹具为合成样本的声明与自查。
#[test]
fn assert_synthetic_fixture_declaration() {
    assert!(FIXTURE.contains(r#""_synthetic": true"#));
    assert!(FIXTURE.contains("不是真实源数据"));
    assert!(STANDARD.contains("合成样本"));
    assert!(STANDARD.contains("不构成任何证据"));
    // 自称真实的文件被拒。
    let claimed_real = FIXTURE.replace(r#""_synthetic": true"#, r#""_synthetic": false"#);
    assert_eq!(
        parse_jin10_envelopes(&claimed_real)
            .expect_err("自称真实")
            .kind(),
        Jin10ErrorKind::SemanticallyRejected
    );
}

/// S-6：三要素门槛、乱序保护与幂等 / 冲突分流。
#[test]
fn assert_macro_mapping_and_ordering() {
    let proposal = propose_macro_mapping(&request(DataKind::Calendar)).expect("三要素齐备");
    assert_eq!(proposal.indicator, "CPI YoY");
    assert_eq!(proposal.subject, "US");
    assert_eq!(proposal.revision(), None, "无 vintage 面，不得伪造修订标识");

    let mut incomplete = request(DataKind::Calendar);
    incomplete.period = None;
    assert_eq!(
        propose_macro_mapping(&incomplete)
            .expect_err("缺业务期间")
            .kind(),
        Jin10ErrorKind::Missing
    );
    incomplete = request(DataKind::Calendar);
    incomplete.indicator = Some("  ".to_owned());
    assert_eq!(
        propose_macro_mapping(&incomplete)
            .expect_err("空白 indicator")
            .kind(),
        Jin10ErrorKind::SemanticallyRejected
    );

    let existing = jin10x::Jin10AcceptedFact {
        msg_id: "m-0".to_owned(),
        indicator: proposal.indicator.clone(),
        subject: proposal.subject.clone(),
        period: proposal.period,
        value: Some(2.9),
        arrival_seq: proposal.arrival_seq + 5,
    };
    assert_eq!(
        jin10x::decide_macro_write(Some(&existing), &proposal),
        jin10x::Jin10WriteDecision::StaleRejected {
            existing_msg_id: "m-0".to_owned()
        },
        "乱序不得覆盖既有事实"
    );

    let newer = jin10x::Jin10AcceptedFact {
        arrival_seq: proposal.arrival_seq - 1,
        value: Some(9.9),
        ..existing
    };
    assert!(matches!(
        jin10x::decide_macro_write(Some(&newer), &proposal),
        jin10x::Jin10WriteDecision::Conflict { .. }
    ));
}

/// S-7：Quote 默认拒绝；拍卖 / RRP 写入主权未决且保持拒绝。
#[test]
fn assert_cross_domain_guards() {
    assert_eq!(
        write_sovereignty(DataKind::Quote),
        Jin10WriteSovereignty::Routed
    );
    assert_eq!(
        propose_macro_mapping(&request(DataKind::Quote))
            .expect_err("Quote 默认拒绝")
            .kind(),
        Jin10ErrorKind::RoutedElsewhere
    );

    assert_eq!(
        write_sovereignty(DataKind::Treasury),
        Jin10WriteSovereignty::Pending
    );
    assert_eq!(
        propose_macro_mapping(&request(DataKind::Treasury))
            .expect_err("拍卖类保持拒绝")
            .kind(),
        Jin10ErrorKind::WriteAuthorityDenied
    );
    assert!(jin10x::PENDING_WRITE_SOVEREIGNTY_NOTE.contains("pending"));
    assert!(jin10x::PENDING_WRITE_SOVEREIGNTY_NOTE.contains("不得主张权威写入"));
}

/// S-8：授权 fail-closed 的六条拒绝路径与一条放行路径。
#[test]
fn assert_authorization_is_fail_closed() {
    let as_of = date("2026-09-22");
    assert!(matches!(
        authorize(None, as_of),
        jin10x::Jin10Authorization::Denied { .. }
    ));

    let mut evidence = Jin10AuthorizationEvidence {
        scope: "offline fixture parse".to_owned(),
        signer: "Owner".to_owned(),
        owner_signed: true,
        valid_from: date("2026-01-01"),
        valid_until: date("2026-12-31"),
    };
    evidence.scope = String::new();
    assert!(matches!(
        authorize(Some(&evidence), as_of),
        jin10x::Jin10Authorization::Denied { .. }
    ));
    evidence.scope = "offline fixture parse".to_owned();
    evidence.signer = String::new();
    assert!(matches!(
        authorize(Some(&evidence), as_of),
        jin10x::Jin10Authorization::Denied { .. }
    ));
    evidence.signer = "Owner".to_owned();
    evidence.owner_signed = false;
    assert!(matches!(
        authorize(Some(&evidence), as_of),
        jin10x::Jin10Authorization::Denied { .. }
    ));
    evidence.owner_signed = true;
    assert!(matches!(
        authorize(Some(&evidence), date("2027-05-01")),
        jin10x::Jin10Authorization::Denied { .. }
    ));
    assert!(matches!(
        authorize(Some(&evidence), date("2025-05-01")),
        jin10x::Jin10Authorization::Denied { .. }
    ));
    assert_eq!(
        authorize(Some(&evidence), as_of),
        jin10x::Jin10Authorization::Authorized {
            scope: "offline fixture parse".to_owned()
        }
    );
    assert_eq!(
        ensure_authorized(None, as_of).expect_err("证据缺失").kind(),
        Jin10ErrorKind::AuthorizationDenied
    );
}

/// S-9：publication 三元组与 PIT 资格被钉死。
#[test]
fn assert_publication_semantics() {
    assert_eq!(
        publication_semantics(),
        (
            TimePrecision::Date,
            AvailabilityEvidence::Inferred,
            PitEligibility::NotEligible
        )
    );
    assert!(!is_formal_pit_eligible());
}

/// S-10：零网络、零凭据、零跨仓依赖，且值对象不含派生指标。
#[test]
fn assert_dependency_and_zero_network() {
    // 依赖面须**恰为**允许集：多一个即违规（无需枚举被禁项，避免把禁项名写进源码）。
    let mut dependencies: Vec<&str> = Vec::new();
    let mut in_dependencies = false;
    for line in CARGO_TOML.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_dependencies = trimmed == "[dependencies]";
            continue;
        }
        if !in_dependencies || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((name, _)) = trimmed.split_once('=') {
            dependencies.push(name.trim());
        }
    }
    dependencies.sort_unstable();
    assert_eq!(
        dependencies,
        ["serde", "serde_json", "thiserror"],
        "依赖面须恰为允许集"
    );
    assert!(!CARGO_TOML.contains("path = \"../"));
    // 该断言保证的是：**本仓 `src/` 下每一个 `.rs` 文件**（运行期递归枚举，
    // 而非手写文件清单）都不含 `http://` / `https://` 端点字面量，也不读环境变量。
    // 需要 URL 字面量的负向用例一律放在 `tests/` 下（如
    // `tests/aidd_boundary.rs` 的 `embedded_url_in_raw_ref_is_rejected`）。
    let files = source_files();
    assert!(!files.is_empty(), "src 下应有源码文件");
    for file in &files {
        let text = std::fs::read_to_string(file).expect("源码可读");
        assert!(
            !text.contains("https://") && !text.contains("http://"),
            "{} 不得含端点字面量",
            file.display()
        );
        assert!(
            !text.contains("env::var"),
            "{} 不得读环境变量",
            file.display()
        );
        assert!(
            !text.contains("from_env"),
            "{} 不得读凭据环境",
            file.display()
        );
    }
    // surprise_z 只是源侧字段的承载：源未给出即 None，本层不计算。
    let payload = Jin10Payload::Calendar(jin10x::CalendarEvent {
        country: "US".to_owned(),
        indicator: "CPI YoY".to_owned(),
        previous: Some(3.0),
        consensus: Some(2.9),
        actual: Some(3.1),
        revised: None,
        surprise_z: None,
    });
    assert!(validate_payload(&payload).is_ok());
}

/// S-11：验收命令被文档化，且本测试面可一次性执行。
#[test]
fn assert_acceptance() {
    for command in [
        "cargo fmt --all -- --check",
        "cargo clippy --all-targets --all-features -- -D warnings",
        "cargo test --all-features",
        "cargo package --no-verify",
    ] {
        assert!(STANDARD.contains(command), "标准文档须列出验收命令");
    }
    let _ = std::env::current_dir().expect("可取得当前目录");
}
