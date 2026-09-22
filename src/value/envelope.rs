//! 信封、数据类别、规划接口常量与三类载荷。
//!
//! 本模块的**源事实**来自 `specs/adapter/jin10.md` §1：
//! `Envelope<T>` · `FlashNews` · `CalendarEvent` · `QuoteSnapshot` · `DataKind` 八类，
//! 以及 S01–S08 规划接口。清单**未**固定的内容（其余五类的载荷字段、端点、限流数字）
//! 一律不在此编造：那五类的离线解析返回
//! [`NotApplicable`](crate::Jin10ErrorKind::NotApplicable)。

use super::Date;

/// 本源标识。信封的 `source` 字段须等于本值。
pub const SOURCE_ID: &str = "jin10";

/// 规划周期**不是**访问合同。
///
/// `S01–S08` 的周期取自清单的「规划」表，仅用于说明规划意图；
/// 本层无网络、无授权，故任何周期数字都**不得**被当作访问频率、
/// 限流配额或 SLA 承诺。
pub const PLANNED_PERIODS_ARE_NOT_ACCESS_CONTRACT: bool = true;

/// 数据类别（清单固定的八类）。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataKind {
    /// 快讯。
    Flash,
    /// 经济日历。
    Calendar,
    /// 财经大事。
    BigEvent,
    /// 央行动态。
    CentralBank,
    /// 行情快照（**默认拒绝**进入宏观观测）。
    Quote,
    /// 国债拍卖 / 逆回购（写入主权**未决**）。
    Treasury,
    /// COT 持仓。
    Cot,
    /// ETF / 库存。
    EtfInventory,
}

impl DataKind {
    /// 清单使用的稳定标识。
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Flash => "Flash",
            Self::Calendar => "Calendar",
            Self::BigEvent => "BigEvent",
            Self::CentralBank => "CentralBank",
            Self::Quote => "Quote",
            Self::Treasury => "Treasury",
            Self::Cot => "Cot",
            Self::EtfInventory => "EtfInventory",
        }
    }

    /// 解析数据类别（大小写敏感，**不**做近义推断）。
    ///
    /// # Errors
    ///
    /// 不属于八类时返回 [`Jin10Error::Invalid`](crate::Jin10Error::Invalid)。
    pub fn parse(input: &str) -> Result<Self, crate::Jin10Error> {
        let kind = match input {
            "Flash" => Self::Flash,
            "Calendar" => Self::Calendar,
            "BigEvent" => Self::BigEvent,
            "CentralBank" => Self::CentralBank,
            "Quote" => Self::Quote,
            "Treasury" => Self::Treasury,
            "Cot" => Self::Cot,
            "EtfInventory" => Self::EtfInventory,
            _ => {
                return Err(crate::Jin10Error::Invalid(format!(
                    "数据类别 {input:?} 不属于清单固定八类"
                )))
            }
        };
        Ok(kind)
    }

    /// 是否为行情类（**默认拒绝**进入宏观观测，`MD-1-R04` 精神）。
    #[must_use]
    pub fn is_quote(self) -> bool {
        matches!(self, Self::Quote)
    }

    /// 是否为与 `treasuryx` 重叠的拍卖 / 逆回购类（写入主权**未决**）。
    #[must_use]
    pub fn overlaps_treasury_write(self) -> bool {
        matches!(self, Self::Treasury)
    }
}

/// 时刻（`HH:MM:SS`，24 小时制）。仅当源显式给出时刻时保留，**不补造**。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Jin10Clock {
    /// 时，0–23。
    pub hour: u8,
    /// 分，0–59。
    pub minute: u8,
    /// 秒，0–59。
    pub second: u8,
}

impl Jin10Clock {
    /// 构造并校验一个时刻。
    ///
    /// # Errors
    ///
    /// 时 / 分 / 秒越界时返回 [`Jin10Error::Invalid`](crate::Jin10Error::Invalid)。
    pub fn new(hour: u8, minute: u8, second: u8) -> Result<Self, crate::Jin10Error> {
        let clock = Self {
            hour,
            minute,
            second,
        };
        clock.validate()?;
        Ok(clock)
    }

    /// 校验时 / 分 / 秒范围。
    ///
    /// # Errors
    ///
    /// 任一分量越界时返回 [`Jin10Error::Invalid`](crate::Jin10Error::Invalid)。
    pub fn validate(&self) -> Result<(), crate::Jin10Error> {
        if self.hour > 23 {
            return Err(crate::Jin10Error::Invalid(format!(
                "时须在 0–23，收到 {}",
                self.hour
            )));
        }
        if self.minute > 59 || self.second > 59 {
            return Err(crate::Jin10Error::Invalid("分与秒须在 0–59".to_owned()));
        }
        Ok(())
    }
}

/// 源侧时间戳：日期必有，时刻**仅在源显式给出时**保留。
///
/// **接受**：`2026-09-18`、`2026-09-18T09:31:05`。
/// **拒绝**：带时区偏移、带小数秒、或缺少 `T` 分隔符者。
/// 本层**不**把日粒度补造成 `00:00:00`（那是伪装成 `Instant`，契约 §5 明令禁止）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Jin10Timestamp {
    /// 日期部分。
    pub date: Date,
    /// 时刻部分；源未给出时为 `None`。
    pub clock: Option<Jin10Clock>,
}

impl Jin10Timestamp {
    /// 仅日期。
    #[must_use]
    pub fn date_only(date: Date) -> Self {
        Self { date, clock: None }
    }

    /// 严格解析 `YYYY-MM-DD` 或 `YYYY-MM-DDTHH:MM:SS`。
    ///
    /// # Errors
    ///
    /// 形态不合规或字段越界时返回 [`Jin10Error::Invalid`](crate::Jin10Error::Invalid)。
    pub fn parse(input: &str) -> Result<Self, crate::Jin10Error> {
        if input.len() == 10 {
            return Ok(Self::date_only(Date::parse(input)?));
        }
        let bytes = input.as_bytes();
        if bytes.len() != 19 || bytes[10] != b'T' {
            return Err(crate::Jin10Error::Invalid(format!(
                "时间戳须为 YYYY-MM-DD 或 YYYY-MM-DDTHH:MM:SS，收到 {input:?}"
            )));
        }
        if bytes[13] != b':' || bytes[16] != b':' {
            return Err(crate::Jin10Error::Invalid("时刻须为 HH:MM:SS".to_owned()));
        }
        let date = Date::parse(&input[0..10])?;
        let hour = parse_two(&bytes[11..13], "时")?;
        let minute = parse_two(&bytes[14..16], "分")?;
        let second = parse_two(&bytes[17..19], "秒")?;
        Ok(Self {
            date,
            clock: Some(Jin10Clock::new(hour, minute, second)?),
        })
    }
}

/// 解析两位补零十进制数。
fn parse_two(bytes: &[u8], label: &str) -> Result<u8, crate::Jin10Error> {
    let mut value: u8 = 0;
    for &b in bytes {
        if !b.is_ascii_digit() {
            return Err(crate::Jin10Error::Invalid(format!(
                "{label}须为两位补零十进制数字"
            )));
        }
        value = value * 10 + (b - b'0');
    }
    Ok(value)
}

/// 规划接口标识（S01–S08）。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InterfaceId {
    /// S01 快讯（WS/SSE + HTTP 兜底）。
    S01,
    /// S02 经济日历。
    S02,
    /// S03 财经大事。
    S03,
    /// S04 央行动态。
    S04,
    /// S05 行情快照。
    S05,
    /// S06 国债拍卖 / 逆回购。
    S06,
    /// S07 COT 持仓。
    S07,
    /// S08 ETF / 库存。
    S08,
}

impl InterfaceId {
    /// 接口标识字符串。
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::S01 => "S01",
            Self::S02 => "S02",
            Self::S03 => "S03",
            Self::S04 => "S04",
            Self::S05 => "S05",
            Self::S06 => "S06",
            Self::S07 => "S07",
            Self::S08 => "S08",
        }
    }

    /// 该接口的登记条目（[`INTERFACE_PLANS`] 的按标识查询）。
    #[must_use]
    pub fn plan(self) -> Jin10InterfacePlan {
        let index = match self {
            Self::S01 => 0,
            Self::S02 => 1,
            Self::S03 => 2,
            Self::S04 => 3,
            Self::S05 => 4,
            Self::S06 => 5,
            Self::S07 => 6,
            Self::S08 => 7,
        };
        INTERFACE_PLANS[index]
    }
}

/// 一个规划接口：标识 + 名称 + 规划周期（秒）+ 清单「方式」逐字文本。
///
/// `planned_period_secs` 是**规划语义**，见
/// [`PLANNED_PERIODS_ARE_NOT_ACCESS_CONTRACT`]；`declared_transport` 同理 ——
/// 它只是清单「方式」列的**逐字文本**，**不**代表本层实现任何传输层，
/// 也**不**代表本层可发起请求。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Jin10InterfacePlan {
    /// 接口标识。
    pub id: InterfaceId,
    /// 清单给的接口名称。
    pub name: &'static str,
    /// 清单「方式」列的逐字文本（规划表述，非实现）。
    pub declared_transport: &'static str,
    /// 清单给的规划周期（秒）；`None` 表示清单未给秒级周期（如「每周六」）。
    pub planned_period_secs: Option<u32>,
    /// 清单给的周期原文，便于人工核对。
    pub planned_period_note: &'static str,
}

/// S01–S08 规划接口表（清单 §1.1 的逐条落点）。
pub const INTERFACE_PLANS: [Jin10InterfacePlan; 8] = [
    Jin10InterfacePlan {
        id: InterfaceId::S01,
        name: "快讯",
        declared_transport: "WS/SSE + HTTP",
        planned_period_secs: Some(5),
        planned_period_note: "实时/5s",
    },
    Jin10InterfacePlan {
        id: InterfaceId::S02,
        name: "经济日历",
        declared_transport: "HTTP",
        planned_period_secs: Some(60),
        planned_period_note: "60s（窗 5s）",
    },
    Jin10InterfacePlan {
        id: InterfaceId::S03,
        name: "财经大事",
        declared_transport: "HTTP",
        planned_period_secs: Some(300),
        planned_period_note: "300s",
    },
    Jin10InterfacePlan {
        id: InterfaceId::S04,
        name: "央行动态",
        declared_transport: "HTTP",
        planned_period_secs: Some(60),
        planned_period_note: "60s",
    },
    Jin10InterfacePlan {
        id: InterfaceId::S05,
        name: "行情快照",
        declared_transport: "HTTP",
        planned_period_secs: Some(3),
        planned_period_note: "3s",
    },
    Jin10InterfacePlan {
        id: InterfaceId::S06,
        name: "国债拍卖/逆回购",
        declared_transport: "HTTP",
        planned_period_secs: Some(3600),
        planned_period_note: "3600s",
    },
    Jin10InterfacePlan {
        id: InterfaceId::S07,
        name: "COT 持仓",
        declared_transport: "HTTP",
        planned_period_secs: None,
        planned_period_note: "每周六",
    },
    Jin10InterfacePlan {
        id: InterfaceId::S08,
        name: "ETF/库存",
        declared_transport: "HTTP",
        planned_period_secs: Some(86400),
        planned_period_note: "86400s",
    },
];

/// 重要度：**仅** 1–5。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Jin10Importance(u8);

impl Jin10Importance {
    /// 构造并校验重要度。
    ///
    /// # Errors
    ///
    /// 不在 1–5 时返回 [`Jin10Error::Invalid`](crate::Jin10Error::Invalid)。
    pub fn new(value: u8) -> Result<Self, crate::Jin10Error> {
        if (1..=5).contains(&value) {
            Ok(Self(value))
        } else {
            Err(crate::Jin10Error::Invalid(format!(
                "重要度须在 1–5，收到 {value}"
            )))
        }
    }

    /// 取值。
    #[must_use]
    pub fn get(self) -> u8 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DataKind, InterfaceId, Jin10Clock, Jin10Importance, Jin10Timestamp, INTERFACE_PLANS,
    };
    use crate::error::Jin10ErrorKind;

    #[test]
    fn data_kind_is_exactly_eight_and_round_trips() {
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
        assert_eq!(all.len(), 8);
        for kind in all {
            assert_eq!(DataKind::parse(kind.as_str()).expect("可回读"), kind);
        }
        assert_eq!(
            DataKind::parse("flash").expect_err("大小写敏感").kind(),
            Jin10ErrorKind::Invalid
        );
    }

    #[test]
    fn guard_predicates_are_exact() {
        assert!(DataKind::Quote.is_quote());
        assert!(!DataKind::Calendar.is_quote());
        assert!(DataKind::Treasury.overlaps_treasury_write());
        assert!(!DataKind::Cot.overlaps_treasury_write());
    }

    #[test]
    fn clock_and_timestamp_are_strict() {
        assert!(Jin10Clock::new(23, 59, 59).is_ok());
        assert_eq!(
            Jin10Clock::new(24, 0, 0).expect_err("时越界").kind(),
            Jin10ErrorKind::Invalid
        );
        assert_eq!(
            Jin10Timestamp::parse("2026-09-18T09:31:05Z")
                .expect_err("带时区")
                .kind(),
            Jin10ErrorKind::Invalid
        );
    }

    #[test]
    fn importance_is_closed_at_one_to_five() {
        for value in 1..=5u8 {
            assert_eq!(Jin10Importance::new(value).expect("1–5").get(), value);
        }
        assert!(Jin10Importance::new(0).is_err());
        assert!(Jin10Importance::new(6).is_err());
    }

    #[test]
    fn interface_plan_lookup_matches_the_table() {
        assert_eq!(INTERFACE_PLANS.len(), 8);
        for plan in INTERFACE_PLANS {
            assert_eq!(plan.id.plan(), plan);
        }
        assert_eq!(InterfaceId::S07.plan().planned_period_secs, None);
        assert_eq!(InterfaceId::S08.plan().planned_period_secs, Some(86400));
        assert_eq!(InterfaceId::S01.plan().declared_transport, "WS/SSE + HTTP");
    }
}
