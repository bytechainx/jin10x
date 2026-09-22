#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable
)]
//! jin10x 热路径微基准：信封解析 + 跨域写入守卫。
//!
//! 离线、无网络、无凭据；输入为**合成样本**（见 `tests/fixtures/`）。
//! 本基准只用于观察本层纯函数开销，**不构成**任何能力或 SLA 声明。

use std::hint::black_box;
use std::time::Instant;

use jin10x::{
    parse_jin10_envelopes, write_sovereignty, DataKind, Frequency, Jin10MacroMappingRequest,
    Jin10Unit, Period,
};

const ITERATIONS: u32 = 2_000;

const INPUT: &str = r#"{
  "_synthetic": true,
  "_note": "微基准内联合成样本，不是真实源数据。",
  "envelopes": [
    {
      "msg_id": "b-1",
      "source": "jin10",
      "kind": "Flash",
      "event_time": "2026-09-18T09:31:05",
      "collect_time": "2026-09-18T09:31:07",
      "payload": { "importance": 3, "channels": ["macro"], "content": "合成", "is_data": false }
    },
    {
      "msg_id": "b-2",
      "source": "jin10",
      "kind": "Calendar",
      "event_time": "2026-09-18",
      "collect_time": "2026-09-18T16:00:00",
      "payload": {
        "country": "US",
        "indicator": "CPI YoY",
        "previous": 3.0,
        "consensus": 2.9,
        "actual": 3.1
      }
    }
  ]
}"#;

fn main() {
    let start = Instant::now();
    let mut parsed_envelopes = 0usize;
    for _ in 0..ITERATIONS {
        let envelopes = parse_jin10_envelopes(black_box(INPUT)).expect("合成样本应可解析");
        parsed_envelopes += envelopes.len();
        for envelope in &envelopes {
            black_box(envelope.payload.kind());
        }
    }
    let parse_elapsed = start.elapsed();

    let request = Jin10MacroMappingRequest {
        msg_id: "b-3".to_owned(),
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
        arrival_seq: 1,
    };
    let start = Instant::now();
    let mut own = 0usize;
    for _ in 0..ITERATIONS {
        let proposal = jin10x::propose_macro_mapping(black_box(&request)).expect("三要素齐备");
        own += usize::from(proposal.revision().is_none());
    }
    let guard_elapsed = start.elapsed();

    let start = Instant::now();
    let mut routed = 0usize;
    for _ in 0..ITERATIONS {
        for kind in [DataKind::Quote, DataKind::Treasury, DataKind::Calendar] {
            black_box(write_sovereignty(black_box(kind)));
            routed += 1;
        }
    }
    let sovereignty_elapsed = start.elapsed();

    println!(
        "bench_jin10x_parse: iters={ITERATIONS} envelopes={parsed_envelopes} total={parse_elapsed:?} per_iter={:?}",
        parse_elapsed / ITERATIONS
    );
    println!(
        "bench_jin10x_mapping_guard: iters={ITERATIONS} own={own} total={guard_elapsed:?} per_iter={:?}",
        guard_elapsed / ITERATIONS
    );
    println!(
        "bench_jin10x_sovereignty: iters={ITERATIONS} checks={routed} total={sovereignty_elapsed:?} per_check={:?}",
        sovereignty_elapsed / u32::try_from(routed.max(1)).unwrap_or(1)
    );
}
