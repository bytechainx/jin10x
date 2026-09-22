//! jin10x 的 publication 语义：时间精度 + 可得性证据层 + 正式 PIT 资格。
//!
//! 本源**只给日期、不给可信时刻**，且**没有**经签批的发布日历或官方 vintage 面，
//! 因此三元组恒为
//! [`Date`](TimePrecision::Date) + [`Inferred`](AvailabilityEvidence::Inferred) +
//! [`NotEligible`](PitEligibility::NotEligible)。
//!
//! **禁则**：不得补造 `00:00 UTC` / `21:00 UTC` 之类时刻把 `Date` 伪装成 `Instant`；
//! 日后若接入官方 vintage 或经签批的发布日历，须先改清单与契约，**禁止静默升格**。

/// 时间精度：源只给日期还是给出时刻。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimePrecision {
    /// 只有日期。
    Date,
    /// 有明确时刻。
    Instant,
}

/// 可得性证据层：官方字段 > 日历 > 推断。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvailabilityEvidence {
    /// 官方字段。
    Official,
    /// 发布日历。
    Calendar,
    /// 推断。
    Inferred,
}

/// 正式 PIT 资格。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PitEligibility {
    /// 可进正式 PIT。
    Formal,
    /// 不可进正式 PIT。
    NotEligible,
}

/// jin10x 的 publication 三元组，**恒为** `(Date, Inferred, NotEligible)`。
#[must_use]
pub fn publication_semantics() -> (TimePrecision, AvailabilityEvidence, PitEligibility) {
    (
        TimePrecision::Date,
        AvailabilityEvidence::Inferred,
        PitEligibility::NotEligible,
    )
}

/// 是否具备正式 PIT 资格。本层恒为 `false`。
#[must_use]
pub fn is_formal_pit_eligible() -> bool {
    matches!(publication_semantics().2, PitEligibility::Formal)
}

#[cfg(test)]
mod tests {
    use super::{
        is_formal_pit_eligible, publication_semantics, AvailabilityEvidence, PitEligibility,
        TimePrecision,
    };

    #[test]
    fn publication_triple_is_pinned_to_date_inferred_not_eligible() {
        let (precision, evidence, eligibility) = publication_semantics();
        assert_eq!(precision, TimePrecision::Date);
        assert_eq!(evidence, AvailabilityEvidence::Inferred);
        assert_eq!(eligibility, PitEligibility::NotEligible);
    }

    #[test]
    fn formal_pit_is_never_eligible_here() {
        assert!(!is_formal_pit_eligible());
        assert_ne!(publication_semantics().2, PitEligibility::Formal);
    }

    #[test]
    fn no_instant_and_no_official_evidence_are_claimed() {
        let (precision, evidence, _) = publication_semantics();
        assert_ne!(precision, TimePrecision::Instant, "不得把日期伪装成时刻");
        assert_ne!(evidence, AvailabilityEvidence::Official);
        assert_ne!(evidence, AvailabilityEvidence::Calendar);
    }
}
