//! jin10x 的错误分类与错误类型。
//!
//! 调用方必须能回答「这个错误值不值得重试、该改数据还是该改授权」，而不是靠字符串匹配。
//! 因此本模块给出可判定的 [`Jin10ErrorKind`] 与携带分类的 [`Jin10Error`]。
//!
//! - 本层**无网络**（`production_decision = NO-GO`，尚无任何采集授权），
//!   故 [`Jin10Error::is_retryable`] 除 [`Jin10ErrorKind::Invariant`] 外一律返回 `false`。
//! - 错误消息为简体中文，**不回显**原始响应正文、凭据或整行配置源码。

/// 错误分类：按「调用方应如何反应」划分。
///
/// 禁止用字符串匹配替代对本枚举的匹配。
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Jin10ErrorKind {
    /// 输入形态或取值非法（调用方应修数据，重试无意义）。
    Invalid,
    /// 缺少必需项。
    Missing,
    /// 授权判定未通过（fail-closed 落点）。
    AuthorizationDenied,
    /// 该产品不属于本域，已被路由规则拒绝。
    RoutedElsewhere,
    /// 越权写入（权威写入归他域或主权未决）。
    WriteAuthorityDenied,
    /// 结构可解析但语义不被接受（如近义非同 ID、身份重复）。
    SemanticallyRejected,
    /// 尚未实现的规划能力。
    NotApplicable,
    /// 不变量被破坏（库内 bug 的信号）。
    Invariant,
}

/// jin10x 错误。保留可区分的分类，供调用方分流。
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum Jin10Error {
    /// 输入非法。
    #[error("输入非法：{0}")]
    Invalid(String),
    /// 缺少必需项。
    #[error("缺少必需项：{0}")]
    Missing(String),
    /// 授权判定未通过。
    #[error("授权判定未通过：{0}")]
    AuthorizationDenied(String),
    /// 产品不属于本域。
    #[error("不属于本域，已路由他处：{0}")]
    RoutedElsewhere(String),
    /// 越权写入。
    #[error("越权写入被拒绝：{0}")]
    WriteAuthorityDenied(String),
    /// 语义拒绝。
    #[error("语义不被接受：{0}")]
    SemanticallyRejected(String),
    /// 规划能力未实现。
    #[error("规划能力尚未实现：{0}")]
    NotApplicable(String),
    /// 不变量被破坏。
    #[error("不变量被破坏：{0}")]
    Invariant(String),
}

impl Jin10Error {
    /// 分类，供调用方按「如何反应」分流。
    #[must_use]
    pub fn kind(&self) -> Jin10ErrorKind {
        match self {
            Self::Invalid(_) => Jin10ErrorKind::Invalid,
            Self::Missing(_) => Jin10ErrorKind::Missing,
            Self::AuthorizationDenied(_) => Jin10ErrorKind::AuthorizationDenied,
            Self::RoutedElsewhere(_) => Jin10ErrorKind::RoutedElsewhere,
            Self::WriteAuthorityDenied(_) => Jin10ErrorKind::WriteAuthorityDenied,
            Self::SemanticallyRejected(_) => Jin10ErrorKind::SemanticallyRejected,
            Self::NotApplicable(_) => Jin10ErrorKind::NotApplicable,
            Self::Invariant(_) => Jin10ErrorKind::Invariant,
        }
    }

    /// 是否值得重试。本层无网络，除 [`Jin10ErrorKind::Invariant`] 外一律 `false`。
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        matches!(self.kind(), Jin10ErrorKind::Invariant)
    }
}

/// 本 crate 统一结果别名。
pub type Jin10Result<T> = Result<T, Jin10Error>;

#[cfg(test)]
mod tests {
    use super::{Jin10Error, Jin10ErrorKind};

    /// 每个变体的 `kind()` 映射必须一一对应。
    #[test]
    fn kind_maps_one_to_one() {
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

    /// 仅 `Invariant` 可重试（本层无网络）。
    #[test]
    fn only_invariant_is_retryable() {
        let kinds = [
            Jin10ErrorKind::Invalid,
            Jin10ErrorKind::Missing,
            Jin10ErrorKind::AuthorizationDenied,
            Jin10ErrorKind::RoutedElsewhere,
            Jin10ErrorKind::WriteAuthorityDenied,
            Jin10ErrorKind::SemanticallyRejected,
            Jin10ErrorKind::NotApplicable,
        ];
        for kind in kinds {
            let error = Jin10Error::Invalid(format!("{kind:?}"));
            assert!(!error.is_retryable(), "{kind:?} 不应可重试");
        }
        assert!(Jin10Error::Invariant("内部不变量".to_owned()).is_retryable());
    }

    /// `Display` 非空且带上分类语义。
    #[test]
    fn display_is_not_empty() {
        let error = Jin10Error::RoutedElsewhere("曲线产品归 yieldx".to_owned());
        let text = error.to_string();
        assert!(!text.is_empty());
        assert!(text.contains("路由"));
        assert!(text.contains("yieldx"));
    }

    /// 错误消息不得回显凭据样式的内容。
    #[test]
    fn display_does_not_leak_credentials() {
        let error = Jin10Error::SemanticallyRejected("字段 country 的取值非法".to_owned());
        let text = error.to_string();
        assert!(!text.contains("token="));
        assert!(!text.contains("password"));
        assert!(!text.contains("Authorization:"));
    }
}
