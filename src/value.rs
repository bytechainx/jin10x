//! jin10x 的基础值对象：日期、期间、频率、源侧单位与校验入口。
//!
//! 本层只需要「日 / 月 / 季 / 年」的**身份**，不需要时区、夏令时、算术或格式化，
//! 因此**不引入** `chrono` / `time`（工作区契约 `source-library-contract` §2.2）。
//!
//! 子模块承载信封、载荷与跨域守卫相关的类型（门面只做声明与重导出）：
//!
//! - [`envelope`]：`DataKind` / 接口常量 / 时间戳 / 重要度；
//! - [`payload`]：信封、三类载荷与校验入口；
//! - [`mapping`]：宏观观测的**提议**与跨域写入守卫。

mod envelope;
mod mapping;
mod payload;

pub use envelope::{
    DataKind, InterfaceId, Jin10Clock, Jin10Importance, Jin10InterfacePlan, Jin10Timestamp,
    INTERFACE_PLANS, PLANNED_PERIODS_ARE_NOT_ACCESS_CONTRACT, SOURCE_ID,
};
pub use mapping::{
    decide_macro_write, propose_macro_mapping, write_sovereignty, Jin10AcceptedFact,
    Jin10MacroMappingRequest, Jin10MacroProposal, Jin10WriteDecision, Jin10WriteSovereignty,
    PENDING_WRITE_SOVEREIGNTY_NOTE,
};
pub use payload::{
    validate_envelope, validate_payload, CalendarEvent, FlashNews, Jin10Envelope, Jin10Payload,
    QuoteSnapshot,
};

use crate::error::{Jin10Error, Jin10Result};

/// 严格 ISO 日历日期 `YYYY-MM-DD`（月 / 日两位补零）。
///
/// **接受**：`2026-09-18`。
/// **拒绝**：`2026-9-18`（未补零）、`2026/09/18`（分隔符）、`2026-09-18T00:00:00`（带时刻）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    /// 年（可负，按来源原样保留）。
    pub year: i16,
    /// 月，1–12。
    pub month: u8,
    /// 日，按月份与闰年取值。
    pub day: u8,
}

impl Date {
    /// 构造并校验一个日期。
    ///
    /// # Errors
    ///
    /// 月不在 1–12、或日超出该月（含闰年二月）天数时返回
    /// [`Jin10Error::Invalid`]。
    pub fn new(year: i16, month: u8, day: u8) -> Jin10Result<Self> {
        let date = Self { year, month, day };
        date.validate()?;
        Ok(date)
    }

    /// 严格解析 `YYYY-MM-DD`。
    ///
    /// # Examples
    ///
    /// ```
    /// use jin10x::{Date, Jin10ErrorKind};
    ///
    /// let date = Date::parse("2026-09-18")?;
    /// assert_eq!((date.year, date.month, date.day), (2026, 9, 18));
    /// assert_eq!(
    ///     Date::parse("2026-9-18").unwrap_err().kind(),
    ///     Jin10ErrorKind::Invalid
    /// );
    /// # Ok::<(), jin10x::Jin10Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// 非 `YYYY-MM-DD` 形态或字段越界时返回 [`Jin10Error::Invalid`]。
    pub fn parse(input: &str) -> Jin10Result<Self> {
        let bytes = input.as_bytes();
        if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
            return Err(Jin10Error::Invalid(format!(
                "日期须为严格 YYYY-MM-DD 形态，收到 {input:?}"
            )));
        }
        let year = parse_fixed_digits(&bytes[0..4], "年")?;
        let month = parse_fixed_digits(&bytes[5..7], "月")?;
        let day = parse_fixed_digits(&bytes[8..10], "日")?;
        let month =
            u8::try_from(month).map_err(|_| Jin10Error::Invalid("月超出取值域".to_owned()))?;
        let day = u8::try_from(day).map_err(|_| Jin10Error::Invalid("日超出取值域".to_owned()))?;
        Self::new(year, month, day)
    }

    /// 校验月范围、日范围与闰年。
    ///
    /// # Errors
    ///
    /// 月不在 1–12、日不在 1–该月天数（含闰年二月）时返回 [`Jin10Error::Invalid`]。
    pub fn validate(&self) -> Jin10Result<()> {
        if self.month < 1 || self.month > 12 {
            return Err(Jin10Error::Invalid(format!(
                "月须在 1–12，收到 {}",
                self.month
            )));
        }
        let max = days_in_month(self.year, self.month);
        if self.day < 1 || self.day > max {
            return Err(Jin10Error::Invalid(format!(
                "日须在 1–{max}（{} 年 {} 月），收到 {}",
                self.year, self.month, self.day
            )));
        }
        Ok(())
    }

    /// 是否闰年（公历规则：4 年一闰，100 年不闰，400 年再闰）。
    #[must_use]
    pub fn is_leap_year(year: i16) -> bool {
        (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
    }
}

impl std::fmt::Display for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// 取两位定长十进制数字（**要求补零**）。
fn parse_fixed_digits(bytes: &[u8], label: &str) -> Jin10Result<i16> {
    let mut value: i16 = 0;
    for &b in bytes {
        if !b.is_ascii_digit() {
            return Err(Jin10Error::Invalid(format!(
                "{label}须为两位补零十进制数字"
            )));
        }
        value = value * 10 + i16::from(b - b'0');
    }
    Ok(value)
}

/// 某年某月的天数。
fn days_in_month(year: i16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if Date::is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// 业务期间。**不得**用 `String` 顶替本类型。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Period {
    /// 日粒度：`YYYY-MM-DD`。
    Day(Date),
    /// 月粒度。
    Month {
        /// 年。
        year: i16,
        /// 月，1–12。
        month: u8,
    },
    /// 季粒度。
    Quarter {
        /// 年。
        year: i16,
        /// 季，1–4。
        quarter: u8,
    },
    /// 年粒度。
    Year(i16),
    /// 事件期间：以事件发生日标识，**不**代表日频序列。
    Event {
        /// 事件发生日。
        date: Date,
    },
}

impl Period {
    /// 校验期间取值域（月 1–12、季 1–4、日按月份与闰年）。
    ///
    /// # Errors
    ///
    /// 任一字段越界时返回 [`Jin10Error::Invalid`]。
    pub fn validate(&self) -> Jin10Result<()> {
        match *self {
            Self::Day(date) | Self::Event { date } => date.validate(),
            Self::Month { month, .. } if (1..=12).contains(&month) => Ok(()),
            Self::Month { month, .. } => {
                Err(Jin10Error::Invalid(format!("月须在 1–12，收到 {month}")))
            }
            Self::Quarter { quarter, .. } if (1..=4).contains(&quarter) => Ok(()),
            Self::Quarter { quarter, .. } => {
                Err(Jin10Error::Invalid(format!("季须在 1–4，收到 {quarter}")))
            }
            Self::Year(_) => Ok(()),
        }
    }
}

/// 频率。取值为契约固定的七个。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Frequency {
    /// 日频。
    Daily,
    /// 周频。
    Weekly,
    /// 月频。
    Monthly,
    /// 季频。
    Quarterly,
    /// 年频。
    Annual,
    /// 事件驱动。
    Event,
    /// 不规则 / 混合。
    Irregular,
}

impl Frequency {
    /// 稳定字符串形式（用于文档与跨库对齐）。
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Monthly => "monthly",
            Self::Quarterly => "quarterly",
            Self::Annual => "annual",
            Self::Event => "event",
            Self::Irregular => "irregular",
        }
    }

    /// 解析稳定字符串形式。
    ///
    /// # Errors
    ///
    /// 不在七个取值内时返回 [`Jin10Error::NotApplicable`]（**不**做近义推断）。
    pub fn parse(input: &str) -> Jin10Result<Self> {
        let lowered = input.to_ascii_lowercase();
        match lowered.as_str() {
            "daily" => Ok(Self::Daily),
            "weekly" => Ok(Self::Weekly),
            "monthly" => Ok(Self::Monthly),
            "quarterly" => Ok(Self::Quarterly),
            "annual" => Ok(Self::Annual),
            "event" => Ok(Self::Event),
            "irregular" => Ok(Self::Irregular),
            _ => Err(Jin10Error::NotApplicable(format!(
                "频率 {input:?} 不在契约的七个取值内"
            ))),
        }
    }
}

/// 源侧单位。
///
/// 清单**未固定**各载荷的单位 ⇒ 本层只做保留与最小具名表达，
/// **不推断**、**不换算**（换算归下游 Normalize）。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Jin10Unit {
    /// 源未声明单位（本层不推断）。
    Unspecified,
    /// 百分比：仅用于字段名自带百分比口径者（如 `change_pct`）。
    Percent,
}

#[cfg(test)]
mod tests {
    use super::{Date, Frequency, Jin10Unit, Period};
    use crate::error::Jin10ErrorKind;

    #[test]
    fn date_parse_is_strict_and_checks_the_calendar() {
        assert_eq!(Date::parse("2026-09-18").expect("合法").day, 18);
        for bad in ["2026-9-18", "2026/09/18", "2026-02-30"] {
            assert_eq!(
                Date::parse(bad).expect_err(bad).kind(),
                Jin10ErrorKind::Invalid
            );
        }
        assert!(Date::is_leap_year(2000) && !Date::is_leap_year(1900));
    }

    #[test]
    fn period_and_frequency_are_typed_not_strings() {
        assert!(Period::Month {
            year: 2026,
            month: 12
        }
        .validate()
        .is_ok());
        assert!(Period::Quarter {
            year: 2026,
            quarter: 5
        }
        .validate()
        .is_err());
        for frequency in [Frequency::Daily, Frequency::Event, Frequency::Irregular] {
            assert_eq!(
                Frequency::parse(frequency.as_str()).expect("可回读"),
                frequency
            );
        }
    }

    #[test]
    fn unit_has_exactly_the_two_declared_forms() {
        assert_ne!(Jin10Unit::Unspecified, Jin10Unit::Percent);
    }
}
