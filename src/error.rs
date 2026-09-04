//! 错误类型定义
//!
//! 提供分层的错误处理系统，为不同模块提供专门的错误类型
//!
//! # 设计原则
//!
//! - **向后兼容**：保留原有的 `Error` 枚举
//! - **分层处理**：不同模块有专门的错误类型
//! - **详细信息**：提供上下文信息，便于调试
//! - **类型安全**：使用 `thiserror` 简化错误处理
//!
//! # 使用示例
//!
//! ```rust
//! use rustdx_complete::{Error, Result};
//!
//! // io::Error 通过 From 自动转换为 Error::Io
//! fn connect_to_server() -> Result<rustdx_complete::tcp::Tcp> {
//!     let tcp = rustdx_complete::tcp::Tcp::new()?; // io::Error 自动转换
//!     Ok(tcp)
//! }
//! ```

use std::io;
use thiserror::Error;

// ============================================================================
// 通用错误类型（向后兼容）
// ============================================================================

/// 通用错误类型
///
/// 保留向后兼容性，涵盖所有可能的错误情况
#[derive(Error, Debug)]
pub enum Error {
    /// IO 错误
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    /// 无效值错误
    #[error("无效值 (期望: {expected:?}, 实际: {found:?})")]
    Invalid { expected: String, found: String },

    /// 自定义错误消息
    #[error("{0}")]
    Custom(&'static str),

    /// TCP 连接错误
    #[error("TCP 错误: {0}")]
    Tcp(#[from] TcpError),

    /// 数据验证错误
    #[error("数据验证错误: {0}")]
    Validation(#[from] ValidationError),

    /// 缓存错误
    #[error("缓存错误: {0}")]
    Cache(#[from] CacheError),

    /// 技术指标错误
    #[error("技术指标错误: {0}")]
    Indicator(#[from] IndicatorError),
}

/// 通用的 Result 类型
pub type Result<T> = std::result::Result<T, Error>;

// ============================================================================
// TCP 连接错误
// ============================================================================

/// TCP 连接相关错误
///
/// 涵盖连接、数据传输、解析等所有 TCP 相关错误
#[derive(Error, Debug)]
pub enum TcpError {
    /// 连接失败
    #[error("无法连接到服务器 {host}:{port} (原因: {reason})")]
    ConnectionFailed {
        host: String,
        port: u16,
        reason: String,
    },

    /// 连接超时
    #[error("连接超时 (超时时间: {timeout:?})")]
    Timeout { timeout: String },

    /// 发送数据失败
    #[error("发送数据失败 (已发送: {sent} 字节, 原因: {reason})")]
    SendFailed { sent: usize, reason: String },

    /// 接收数据失败
    #[error("接收数据失败 (期望: {expected} 字节, 原因: {reason})")]
    ReceiveFailed { expected: usize, reason: String },

    /// 数据解压失败
    #[error("数据解压失败 (解压前: {deflate} 字节, 解压后期望: {inflate} 字节)")]
    DecompressionFailed { deflate: usize, inflate: usize },

    /// 响应数据格式错误
    #[error("响应数据格式错误 (期望长度: {expected}, 实际: {actual})")]
    InvalidResponse { expected: usize, actual: usize },

    /// 数据解析失败
    #[error("数据解析失败 (字段: {field}, 原因: {reason})")]
    ParseFailed { field: String, reason: String },

    /// 底层 IO 错误
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
}

impl TcpError {
    /// 从 IO 错误创建连接失败错误
    pub fn connection_failed(host: String, port: u16, err: &io::Error) -> Self {
        TcpError::ConnectionFailed {
            host,
            port,
            reason: err.to_string(),
        }
    }

    /// 创建超时错误
    pub fn timeout(duration: std::time::Duration) -> Self {
        TcpError::Timeout {
            timeout: format!("{:.2}s", duration.as_secs_f64()),
        }
    }
}

// ============================================================================
// 数据验证错误
// ============================================================================

/// 数据验证相关错误
///
/// 涵盖数据完整性、一致性、异常检测等验证错误
#[derive(Error, Debug)]
pub enum ValidationError {
    /// K线数据不连续
    #[error("K线数据不连续 (代码: {code}, 缺失日期: {missing:?})")]
    KlineDiscontinuity {
        code: String,
        missing: Vec<String>,
    },

    /// K线数据异常
    #[error("K线数据异常 (代码: {code}, 日期: {date}, 类型: {anomaly_type})")]
    KlineAnomaly {
        code: String,
        date: String,
        anomaly_type: String,
    },

    /// 财务数据不一致
    #[error("财务数据不一致 (字段: {field}, 原因: {reason})")]
    FinanceInconsistency { field: String, reason: String },

    /// 数据不足
    #[error("数据不足 (需要: {required}, 实际: {actual})")]
    InsufficientData { required: usize, actual: usize },

    /// 空数据集
    #[error("数据集为空 (上下文: {context})")]
    EmptyDataset { context: String },

    /// 验证警告（非致命错误）
    #[error("验证警告: {0}")]
    Warning(String),
}

impl ValidationError {
    /// 创建 K线不连续错误
    pub fn kline_discontinuity(code: String, missing: Vec<String>) -> Self {
        ValidationError::KlineDiscontinuity { code, missing }
    }

    /// 创建数据异常错误
    pub fn kline_anomaly(code: String, date: String, anomaly_type: String) -> Self {
        ValidationError::KlineAnomaly {
            code,
            date,
            anomaly_type,
        }
    }

    /// 创建数据不足错误
    pub fn insufficient_data(required: usize, actual: usize) -> Self {
        ValidationError::InsufficientData { required, actual }
    }
}

// ============================================================================
// 缓存错误
// ============================================================================

/// 缓存操作相关错误
///
/// 涵盖缓存读写、文件操作、序列化等错误
#[derive(Error, Debug)]
pub enum CacheError {
    /// 文件操作失败
    #[error("缓存文件操作失败 (路径: {path}, 原因: {reason})")]
    FileOperationFailed { path: String, reason: String },

    /// 缓存数据损坏
    #[error("缓存数据损坏 (键: {key}, 原因: {reason})")]
    CorruptedData { key: String, reason: String },

    /// 缓存已过期
    #[error("缓存已过期 (键: {key}, TTL: {ttl:?})")]
    Expired { key: String, ttl: String },

    /// 序列化失败
    #[error("序列化失败 (类型: {type_name}, 原因: {reason})")]
    SerializationFailed { type_name: String, reason: String },

    /// 反序列化失败
    #[error("反序列化失败 (类型: {type_name}, 原因: {reason})")]
    DeserializationFailed { type_name: String, reason: String },

    /// 底层 IO 错误
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
}

impl CacheError {
    /// 创建文件操作失败错误
    pub fn file_operation_failed(path: String, err: &io::Error) -> Self {
        CacheError::FileOperationFailed {
            path,
            reason: err.to_string(),
        }
    }
}

// ============================================================================
// 技术指标错误
// ============================================================================

/// 技术指标计算相关错误
///
/// 涵盖参数验证、数据量检查等错误
#[derive(Error, Debug)]
pub enum IndicatorError {
    /// 参数无效
    #[error("参数无效 (指标: {indicator}, 参数: {parameter}, 值: {value})")]
    InvalidParameter {
        indicator: String,
        parameter: String,
        value: String,
    },

    /// 数据不足
    #[error("数据不足 (指标: {indicator}, 最小需求: {min_required}, 实际: {actual})")]
    InsufficientData {
        indicator: String,
        min_required: usize,
        actual: usize,
    },

    /// 计算溢出
    #[error("计算溢出 (指标: {indicator}, 原因: {reason})")]
    Overflow { indicator: String, reason: String },

    /// 除零错误
    #[error("除零错误 (指标: {indicator}, 上下文: {context})")]
    DivisionByZero { indicator: String, context: String },
}

impl IndicatorError {
    /// 创建数据不足错误
    pub fn insufficient_data(indicator: String, min_required: usize, actual: usize) -> Self {
        IndicatorError::InsufficientData {
            indicator,
            min_required,
            actual,
        }
    }

    /// 创建参数无效错误
    pub fn invalid_parameter(indicator: String, parameter: String, value: String) -> Self {
        IndicatorError::InvalidParameter {
            indicator,
            parameter,
            value,
        }
    }
}

// ============================================================================
// 辅助 Trait
// ============================================================================

/// 错误上下文扩展
///
/// 为错误添加更多上下文信息
pub trait ErrorContext<T> {
    /// 添加上下文信息
    fn with_context(self, context: &str) -> Result<T>;

    /// 转换为 TCP 错误
    fn map_tcp(self) -> Result<T>;

    /// 转换为验证错误
    fn map_validation(self) -> Result<T>;
}

impl<T, E> ErrorContext<T> for std::result::Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn with_context(self, context: &str) -> Result<T> {
        // 使用 Box::leak 将字符串转换为 'static 生命周期
        let static_msg: &'static str = Box::leak(context.to_string().into_boxed_str());
        self.map_err(|_e| Error::Custom(static_msg))
    }

    fn map_tcp(self) -> Result<T> {
        self.map_err(|e| TcpError::Io(io::Error::other(e)).into())
    }

    fn map_validation(self) -> Result<T> {
        self.map_err(|e| ValidationError::Warning(e.to_string()).into())
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::Custom("测试错误");
        assert_eq!(err.to_string(), "测试错误");
    }

    #[test]
    fn test_tcp_error_display() {
        let err = TcpError::timeout(std::time::Duration::from_secs(5));
        assert!(err.to_string().contains("连接超时"));
    }

    #[test]
    fn test_validation_error_display() {
        let err = ValidationError::insufficient_data(10, 5);
        assert!(err.to_string().contains("数据不足"));
        assert!(err.to_string().contains("10"));
        assert!(err.to_string().contains("5"));
    }

    #[test]
    fn test_indicator_error_display() {
        let err = IndicatorError::insufficient_data("SMA".to_string(), 20, 10);
        assert!(err.to_string().contains("SMA"));
        assert!(err.to_string().contains("20"));
        assert!(err.to_string().contains("10"));
    }

    #[test]
    fn test_error_conversion() {
        let io_err = io::Error::new(io::ErrorKind::ConnectionRefused, "连接被拒绝");
        let tcp_err = TcpError::from(io_err);
        assert!(matches!(tcp_err, TcpError::Io(_)));
    }

    #[test]
    fn test_error_chain() {
        let err = Error::Tcp(TcpError::timeout(std::time::Duration::from_secs(5)));
        assert!(err.to_string().contains("TCP 错误"));
    }
}
