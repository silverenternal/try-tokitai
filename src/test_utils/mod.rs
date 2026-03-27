//! 测试工具模块
//!
//! 提供测试数据工厂、Mock 工具和通用测试辅助函数
//!
//! # 使用示例
//!
//! ```rust,ignore
//! #[cfg(test)]
//! mod tests {
//!     use crate::test_utils::{factories, fixtures};
//!     
//!     #[test]
//!     fn test_something() {
//!         let tensor = factories::create_test_tensor(&[2, 3]);
//!         let temp_dir = fixtures::temp_project_dir();
//!     }
//! }
//! ```

pub mod factories;
pub mod fixtures;
pub mod mocks;

/// 测试辅助宏
#[macro_export]
macro_rules! assert_approx_eq {
    ($a:expr, $b:expr, $eps:expr $(,)?) => {{
        let (a, b, eps) = (&$a, &$b, $eps);
        assert!(
            (a - b).abs() < eps,
            "assertion failed: `(left != right)` \
             (left: `{:?}`, right: `{:?}`, expect diff: `{:?}`)",
            a,
            b,
            eps
        );
    }};
    ($a:expr, $b:expr $(,)?) => {{
        $crate::assert_approx_eq!($a, $b, 1e-6);
    }};
}

/// 断言结果包含特定错误消息的宏
#[macro_export]
macro_rules! assert_err_contains {
    ($result:expr, $msg:expr $(,)?) => {{
        let result: anyhow::Result<_> = $result;
        assert!(result.is_err(), "Expected error, got Ok");
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains($msg),
            "Error message '{}' does not contain '{}'",
            err_msg,
            $msg
        );
    }};
}
