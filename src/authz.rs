//! jin10x 的授权判定（**fail-closed**）。
//!
//! 权威依据：`specs/005-macro-data-source-crates/contracts/module-roster.json` 的
//! `jin10` 条目 —— `authorization = unknown`，
//! `authorization_evidence = "无 Owner 签核文件；REST/WSS/MCP 全 UNKNOWN"`。
//!
//! 因此本模块的判定**默认拒绝**：证据缺失、过期、签署者不明、覆盖范围不明，
//! 一律返回 [`Jin10Authorization::Denied`]。判定是**只读**结论，
//! **不改变**名册的登记值，也不表示本库生产就绪
//! （`production_decision = NO-GO` 与「某范围是否被授权访问」是两件事）。

use crate::error::Jin10Result;
use crate::value::Date;

/// 名册登记的授权状态（只读引用，不参与判定）。
pub const AUTHORIZATION_STATUS: &str = "unknown";

/// 名册登记的授权证据说明（只读引用，不参与判定）。
pub const AUTHORIZATION_EVIDENCE: &str = "无 Owner 签核文件；REST/WSS/MCP 全 UNKNOWN";

/// 授权判定结果。
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Jin10Authorization {
    /// 证据有效，且覆盖本次请求的范围与有效期。
    Authorized {
        /// 被覆盖的源 / 模式 / 用途（可读描述）。
        scope: String,
    },
    /// 证据缺失 / 过期 / 签署者不明 / 覆盖范围不明。
    Denied {
        /// 可读的拒绝理由。
        reason: String,
    },
}

/// 授权证据的**形状**。
///
/// 本源当前**没有**任何签核文件（见 [`AUTHORIZATION_EVIDENCE`]），
/// 故此类型只用于表达判定所需的形状；实际调用中传入 `None`
/// 即得到 fail-closed 的 [`Jin10Authorization::Denied`]。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Jin10AuthorizationEvidence {
    /// 被覆盖的源 / 模式 / 用途。
    pub scope: String,
    /// 签署者。
    pub signer: String,
    /// 是否经 Owner 签核。
    pub owner_signed: bool,
    /// 生效日（含）。
    pub valid_from: Date,
    /// 失效日（含）。
    pub valid_until: Date,
}

/// fail-closed 授权判定。
///
/// 判定顺序（任一不满足即拒绝，**不**默认放行）：
///
/// 日期分量先复验：当前日、生效日或失效日非法即 `Denied`。
///
/// 1. 证据缺失 → `Denied`；
/// 2. 覆盖范围为空 / 全空白 → `Denied`（范围不明）；
/// 3. 签署者为空 / 全空白 → `Denied`（签署者不明）；
/// 4. 未获 Owner 签核 → `Denied`；
/// 5. `as_of` 早于 `valid_from` 或晚于 `valid_until` → `Denied`（未生效 / 已过期）。
#[must_use]
pub fn authorize(evidence: Option<&Jin10AuthorizationEvidence>, as_of: Date) -> Jin10Authorization {
    let Some(evidence) = evidence else {
        return Jin10Authorization::Denied {
            reason: format!("证据缺失：{AUTHORIZATION_EVIDENCE}"),
        };
    };
    if as_of.validate().is_err()
        || evidence.valid_from.validate().is_err()
        || evidence.valid_until.validate().is_err()
    {
        return Jin10Authorization::Denied {
            reason: "授权日期非法：须为有效日历日期".to_owned(),
        };
    }
    if evidence.scope.trim().is_empty() {
        return Jin10Authorization::Denied {
            reason: "覆盖范围不明：scope 为空".to_owned(),
        };
    }
    if evidence.signer.trim().is_empty() {
        return Jin10Authorization::Denied {
            reason: "签署者不明：signer 为空".to_owned(),
        };
    }
    if !evidence.owner_signed {
        return Jin10Authorization::Denied {
            reason: "未获 Owner 签核".to_owned(),
        };
    }
    if as_of < evidence.valid_from {
        return Jin10Authorization::Denied {
            reason: format!("证据尚未生效（生效日 {}）", evidence.valid_from),
        };
    }
    if as_of > evidence.valid_until {
        return Jin10Authorization::Denied {
            reason: format!("证据已过期（失效日 {}）", evidence.valid_until),
        };
    }
    Jin10Authorization::Authorized {
        scope: evidence.scope.clone(),
    }
}

/// 判定为「必须拒绝」时返回类型化错误，便于调用方直接 `?`。
///
/// # Errors
///
/// 未获授权时返回
/// [`AuthorizationDenied`](crate::Jin10ErrorKind::AuthorizationDenied)。
pub fn ensure_authorized(
    evidence: Option<&Jin10AuthorizationEvidence>,
    as_of: Date,
) -> Jin10Result<()> {
    match authorize(evidence, as_of) {
        Jin10Authorization::Authorized { .. } => Ok(()),
        Jin10Authorization::Denied { reason } => {
            Err(crate::Jin10Error::AuthorizationDenied(reason))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        authorize, ensure_authorized, Jin10Authorization, Jin10AuthorizationEvidence,
        AUTHORIZATION_STATUS,
    };
    use crate::error::Jin10ErrorKind;
    use crate::value::Date;

    fn date(text: &str) -> Date {
        Date::parse(text).expect("测试用日期合法")
    }

    fn complete() -> Jin10AuthorizationEvidence {
        Jin10AuthorizationEvidence {
            scope: "offline fixture parse".to_owned(),
            signer: "Owner".to_owned(),
            owner_signed: true,
            valid_from: date("2026-01-01"),
            valid_until: date("2026-12-31"),
        }
    }

    #[test]
    fn missing_evidence_is_denied() {
        let verdict = authorize(None, date("2026-09-22"));
        assert!(matches!(verdict, Jin10Authorization::Denied { .. }));
        assert_eq!(AUTHORIZATION_STATUS, "unknown");
    }

    #[test]
    fn unknown_scope_is_denied() {
        let mut evidence = complete();
        evidence.scope = "   ".to_owned();
        match authorize(Some(&evidence), date("2026-09-22")) {
            Jin10Authorization::Denied { reason } => assert!(reason.contains("范围")),
            Jin10Authorization::Authorized { .. } => panic!("空白范围不得放行"),
        }
    }

    #[test]
    fn unsigned_evidence_is_denied() {
        let mut evidence = complete();
        evidence.owner_signed = false;
        assert!(matches!(
            authorize(Some(&evidence), date("2026-09-22")),
            Jin10Authorization::Denied { .. }
        ));
    }

    #[test]
    fn expired_and_not_yet_valid_evidence_are_denied() {
        let evidence = complete();
        assert!(matches!(
            authorize(Some(&evidence), date("2027-01-01")),
            Jin10Authorization::Denied { .. }
        ));
        assert!(matches!(
            authorize(Some(&evidence), date("2025-12-31")),
            Jin10Authorization::Denied { .. }
        ));
    }

    #[test]
    fn complete_evidence_within_window_is_authorized() {
        let verdict = authorize(Some(&complete()), date("2026-09-22"));
        assert_eq!(
            verdict,
            Jin10Authorization::Authorized {
                scope: "offline fixture parse".to_owned()
            }
        );
    }

    #[test]
    fn ensure_authorized_maps_to_typed_error() {
        assert!(ensure_authorized(None, date("2026-09-22")).is_err());
        assert_eq!(
            ensure_authorized(None, date("2026-09-22"))
                .expect_err("证据缺失")
                .kind(),
            Jin10ErrorKind::AuthorizationDenied
        );
        assert!(ensure_authorized(Some(&complete()), date("2026-09-22")).is_ok());
    }

    #[test]
    fn adversarial_authorization_dates_are_revalidated() {
        let bad = Date {
            year: 2026,
            month: 9,
            day: 99,
        };
        let mut evidence = complete();
        assert!(matches!(
            authorize(Some(&evidence), bad),
            Jin10Authorization::Denied { .. }
        ));
        evidence.valid_from = Date {
            year: 2026,
            month: 0,
            day: 1,
        };
        assert!(matches!(
            authorize(Some(&evidence), date("2026-09-22")),
            Jin10Authorization::Denied { .. }
        ));
        evidence = complete();
        evidence.valid_until = bad;
        assert!(matches!(
            authorize(Some(&evidence), date("2026-09-22")),
            Jin10Authorization::Denied { .. }
        ));
    }
}
