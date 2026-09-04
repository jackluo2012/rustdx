#![cfg_attr(feature = "docsrs", feature(doc_auto_cfg))]

pub mod bytes_helper;

pub mod file;

pub mod tcp;

pub mod indicators;

pub mod cache;

pub mod error;

pub mod calendar;

pub mod builder;

pub mod pool;

// 重新导出常用错误类型
pub use error::{Error, Result, TcpError, ValidationError, CacheError, IndicatorError};

// 重新导出Builder
pub use builder::{KlineBuilder};

// 重新导出连接池
pub use pool::{ConnectionPool, PoolConfig, PoolStats};
