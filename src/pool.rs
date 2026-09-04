//! TCP 连接池
//!
//! 提供连接复用功能，减少TCP连接开销，提升性能
//!
//! # 特性
//!
//! - ✅ 连接复用，减少握手开销
//! - ✅ 自动健康检查
//! - ✅ 线程安全
//! - ✅ 可配置池大小
//! - ✅ 优雅的错误处理
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use rustdx_complete::pool::ConnectionPool;
//!
//! // 创建连接池（最大3个连接）
//! let pool = ConnectionPool::new(3).unwrap();
//!
//! // 从池中获取连接
//! let mut conn = pool.get_connection().unwrap();
//!
//! // 使用连接执行查询
//! // ... 查询操作 ...
//!
//! // 连接自动归还到池中（当离开作用域时）
//! drop(conn);
//! ```

use crate::tcp::Tcp;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// 连接池配置
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// 最大连接数
    pub max_size: usize,
    /// 连接最大空闲时间（秒）
    pub max_idle: u64,
    /// 连接最大生命周期（秒）
    pub max_lifetime: u64,
    /// 健康检查超时（秒）
    pub health_check_timeout: u64,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_size: 3,
            max_idle: 600,        // 10分钟
            max_lifetime: 3600,   // 1小时
            health_check_timeout: 5,
        }
    }
}

/// 池化连接
#[derive(Debug)]
struct PooledConnection {
    /// TCP连接
    tcp: Option<Tcp>,
    /// 创建时间
    created_at: Instant,
    /// 最后使用时间
    last_used: Instant,
    /// 是否在使用中
    in_use: bool,
}

impl PooledConnection {
    /// 健康检查
    fn is_healthy(&self, config: &PoolConfig) -> bool {
        // 检查是否超过最大生命周期
        if self.created_at.elapsed() > Duration::from_secs(config.max_lifetime) {
            return false;
        }

        // 检查是否超过最大空闲时间
        if self.last_used.elapsed() > Duration::from_secs(config.max_idle) {
            return false;
        }

        true
    }

    /// 标记为使用中
    fn mark_used(&mut self) {
        self.in_use = true;
        self.last_used = Instant::now();
    }

    /// 标记为空闲
    fn mark_idle(&mut self) {
        self.in_use = false;
        self.last_used = Instant::now();
    }
}

/// TCP连接池
///
/// 管理多个TCP连接，提供连接复用功能
#[derive(Debug, Clone)]
pub struct ConnectionPool {
    /// 连接列表
    connections: Arc<Mutex<Vec<PooledConnection>>>,
    /// 配置
    config: PoolConfig,
}

impl ConnectionPool {
    /// 创建新的连接池
    ///
    /// # 参数
    ///
    /// - `max_size`: 最大连接数
    ///
    /// # 返回
    ///
    /// - `Ok(ConnectionPool)`: 创建成功
    /// - `Err(std::io::Error)`: 创建失败
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rustdx_complete::pool::ConnectionPool;
    ///
    /// let pool = ConnectionPool::new(3).unwrap();
    /// ```
    pub fn new(max_size: usize) -> Result<Self, std::io::Error> {
        let config = PoolConfig {
            max_size,
            ..Default::default()
        };

        Ok(Self {
            connections: Arc::new(Mutex::new(Vec::new())),
            config,
        })
    }

    /// 使用自定义配置创建连接池
    ///
    /// # 参数
    ///
    /// - `config`: 连接池配置
    ///
    /// # 返回
    ///
    /// - `Ok(ConnectionPool)`: 创建成功
    /// - `Err(std::io::Error)`: 创建失败
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rustdx_complete::pool::{ConnectionPool, PoolConfig};
    ///
    /// let config = PoolConfig {
    ///     max_size: 5,
    ///     max_idle: 300,
    ///     ..Default::default()
    /// };
    ///
    /// let pool = ConnectionPool::with_config(config).unwrap();
    /// ```
    pub fn with_config(config: PoolConfig) -> Result<Self, std::io::Error> {
        Ok(Self {
            connections: Arc::new(Mutex::new(Vec::new())),
            config,
        })
    }

    /// 从池中获取连接
    ///
    /// # 返回
    ///
    /// - `Ok(PooledConnection)`: 获取成功
    /// - `Err(std::io::Error)`: 获取失败
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rustdx_complete::pool::ConnectionPool;
    ///
    /// let pool = ConnectionPool::new(3).unwrap();
    /// let conn = pool.get_connection().unwrap();
    ///
    /// // 使用连接...
    ///
    /// // 连接自动归还到池中
    /// drop(conn);
    /// ```
    pub fn get_connection(&self) -> Result<PooledConn, std::io::Error> {
        let mut connections = self.connections.lock().unwrap();

        // 清理过期连接
        self.cleanup_idle_connections(&mut connections);

        // 查找空闲连接
        for (i, conn) in connections.iter_mut().enumerate() {
            if !conn.in_use && conn.is_healthy(&self.config) {
                conn.mark_used();
                // 取出TCP连接
                let tcp = conn.tcp.take().unwrap();
                return Ok(PooledConn {
                    pool: self.clone(),
                    index: i,
                    tcp: Some(tcp),
                });
            }
        }

        // 没有可用连接，创建新连接
        if connections.len() < self.config.max_size {
            let tcp = Tcp::new()?;
            let new_conn = PooledConnection {
                tcp: None,  // TCP会被PooledConn持有
                created_at: Instant::now(),
                last_used: Instant::now(),
                in_use: true,
            };
            connections.push(new_conn);

            let index = connections.len() - 1;
            return Ok(PooledConn {
                pool: self.clone(),
                index,
                tcp: Some(tcp),
            });
        }

        // 池已满，返回错误
        Err(std::io::Error::new(
            std::io::ErrorKind::WouldBlock,
            "Connection pool exhausted",
        ))
    }

    /// 清理空闲连接
    fn cleanup_idle_connections(&self, connections: &mut Vec<PooledConnection>) {
        connections.retain(|conn| {
            // 保留所有活跃连接
            if conn.in_use {
                return true;
            }

            // 移除不健康的连接
            conn.is_healthy(&self.config)
        });
    }

    /// 获取池统计信息
    ///
    /// # 返回
    ///
    /// 连接池统计信息
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rustdx_complete::pool::ConnectionPool;
    ///
    /// let pool = ConnectionPool::new(3).unwrap();
    /// let stats = pool.stats();
    ///
    /// println!("总连接数: {}", stats.total);
    /// println!("活跃连接数: {}", stats.active);
    /// println!("空闲连接数: {}", stats.idle);
    /// ```
    pub fn stats(&self) -> PoolStats {
        let connections = self.connections.lock().unwrap();

        let total = connections.len();
        let active = connections.iter().filter(|c| c.in_use).count();
        let idle = total - active;

        PoolStats {
            total,
            active,
            idle,
            max_size: self.config.max_size,
        }
    }

    /// 关闭连接池
    ///
    /// 关闭所有连接并释放资源
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rustdx_complete::pool::ConnectionPool;
    ///
    /// let pool = ConnectionPool::new(3).unwrap();
    /// pool.close();  // 关闭所有连接
    /// ```
    pub fn close(&self) {
        let mut connections = self.connections.lock().unwrap();
        connections.clear();
    }
}

/// 池化连接包装器
///
/// 当离开作用域时，自动归还连接到池中
///
/// # 示例
///
/// ```rust
/// use rustdx_complete::pool::ConnectionPool;
/// use rustdx_complete::tcp::Tdx;
///
/// let pool = ConnectionPool::new(3).unwrap();
/// let conn = pool.get_connection().unwrap();
///
/// // 使用连接执行查询
/// // 注意：PooledConn 是一个智能指针，会自动管理TCP连接
/// ```
pub struct PooledConn {
    pool: ConnectionPool,
    index: usize,
    /// 直接持有TCP连接，避免借用检查问题
    tcp: Option<Tcp>,
}

impl PooledConn {
    /// 获取内部TCP连接的可变引用
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rustdx_complete::pool::ConnectionPool;
    /// use rustdx_complete::tcp::Tdx;
    ///
    /// let pool = ConnectionPool::new(3).unwrap();
    /// let mut conn = pool.get_connection().unwrap();
    /// let tcp = conn.get_mut();
    ///
    /// // 使用tcp执行查询...
    /// ```
    pub fn get_mut(&mut self) -> &mut Tcp {
        self.tcp.as_mut().unwrap()
    }

    /// 执行操作并返回结果（便捷方法）
    ///
    /// # 参数
    ///
    /// - `f`: 操作函数
    ///
    /// # 返回
    ///
    /// 操作结果
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rustdx_complete::pool::ConnectionPool;
    ///
    /// let pool = ConnectionPool::new(3).unwrap();
    /// let mut conn = pool.get_connection().unwrap();
    ///
    /// let result = conn.execute(|_tcp| {
    ///     // 使用 tcp 执行查询，返回任意结果
    ///     42
    /// });
    /// assert_eq!(result, 42);
    /// ```
    pub fn execute<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Tcp) -> R,
    {
        f(self.get_mut())
    }
}

impl Drop for PooledConn {
    fn drop(&mut self) {
        // 将TCP连接归还到池中
        let mut connections = self.pool.connections.lock().unwrap();

        // 将连接放回池中的对应位置
        if let Some(pool_conn) = connections.get_mut(self.index) {
            pool_conn.tcp = self.tcp.take();
            pool_conn.mark_idle();
        }
    }
}

/// 连接池统计信息
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// 总连接数
    pub total: usize,
    /// 活跃连接数
    pub active: usize,
    /// 空闲连接数
    pub idle: usize,
    /// 最大连接数
    pub max_size: usize,
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_creation() {
        let pool = ConnectionPool::new(3).unwrap();
        let stats = pool.stats();

        assert_eq!(stats.total, 0);
        assert_eq!(stats.active, 0);
        assert_eq!(stats.max_size, 3);
    }

    #[test]
    fn test_pool_get_connection() {
        let pool = ConnectionPool::new(3).unwrap();

        // 获取连接
        let conn1 = pool.get_connection();
        assert!(conn1.is_ok());

        let stats = pool.stats();
        assert_eq!(stats.total, 1);
        assert_eq!(stats.active, 1);
        assert_eq!(stats.idle, 0);
    }

    #[test]
    fn test_pool_connection_return() {
        let pool = ConnectionPool::new(3).unwrap();

        {
            let _conn = pool.get_connection().unwrap();
            let stats = pool.stats();
            assert_eq!(stats.active, 1);
        }

        // 连接已归还
        let stats = pool.stats();
        assert_eq!(stats.active, 0);
        assert_eq!(stats.idle, 1);
    }

    #[test]
    fn test_pool_max_size() {
        let pool = ConnectionPool::new(2).unwrap();

        // 获取第一个连接
        let _conn1 = pool.get_connection().unwrap();
        let stats = pool.stats();
        assert_eq!(stats.total, 1);

        // 获取第二个连接
        let _conn2 = pool.get_connection().unwrap();
        let stats = pool.stats();
        assert_eq!(stats.total, 2);

        // 尝试获取第三个连接（应该失败）
        let result = pool.get_connection();
        assert!(result.is_err());
    }

    #[test]
    fn test_pool_connection_reuse() {
        let pool = ConnectionPool::new(2).unwrap();

        // 获取并归还连接
        {
            let _conn = pool.get_connection().unwrap();
        }

        let stats = pool.stats();
        assert_eq!(stats.total, 1);

        // 再次获取应该复用之前的连接
        let _conn2 = pool.get_connection().unwrap();
        let stats = pool.stats();
        assert_eq!(stats.total, 1); // 没有创建新连接
        assert_eq!(stats.active, 1);
    }

    #[test]
    fn test_pool_close() {
        let pool = ConnectionPool::new(3).unwrap();

        // 获取一些连接
        let _conn1 = pool.get_connection().unwrap();
        let _conn2 = pool.get_connection().unwrap();

        let stats = pool.stats();
        assert_eq!(stats.total, 2);

        // 关闭池
        pool.close();

        let stats = pool.stats();
        assert_eq!(stats.total, 0);
    }

    #[test]
    fn test_pool_config() {
        let config = PoolConfig {
            max_size: 5,
            max_idle: 300,
            max_lifetime: 1800,
            health_check_timeout: 10,
        };

        let pool = ConnectionPool::with_config(config).unwrap();
        let stats = pool.stats();

        assert_eq!(stats.max_size, 5);
    }
}
