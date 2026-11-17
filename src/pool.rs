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
use std::sync::{Arc, Mutex, MutexGuard};
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
            max_idle: 600,      // 10分钟
            max_lifetime: 3600, // 1小时
            health_check_timeout: 5,
        }
    }
}

/// 空闲连接
#[derive(Debug)]
struct IdleConn {
    /// TCP连接
    tcp: Tcp,
    /// 创建时间
    created_at: Instant,
    /// 最后使用时间
    last_used: Instant,
}

impl IdleConn {
    /// 健康检查
    fn is_healthy(&self, config: &PoolConfig) -> bool {
        // 未超过最大生命周期，且未超过最大空闲时间
        !lifetime_expired(self.created_at, config)
            && self.last_used.elapsed() <= Duration::from_secs(config.max_idle)
    }
}

/// 连接是否已超过最大生命周期
fn lifetime_expired(created_at: Instant, config: &PoolConfig) -> bool {
    created_at.elapsed() > Duration::from_secs(config.max_lifetime)
}

/// 连接池内部状态
///
/// 不使用下标定位连接：借出的连接由 [`PooledConn`] 自己持有，
/// 池里只记录空闲连接和借出数量，清理空闲连接不会影响归还。
#[derive(Debug, Default)]
struct PoolState {
    /// 空闲连接（后进先出）
    idle: Vec<IdleConn>,
    /// 已借出的连接数（包含正在建立、已预留名额的连接）
    active: usize,
    /// 代数：每次 `close()` 加一，旧代的连接归还时直接丢弃
    generation: u64,
}

/// TCP连接池
///
/// 管理多个TCP连接，提供连接复用功能
#[derive(Debug, Clone)]
pub struct ConnectionPool {
    /// 内部状态
    state: Arc<Mutex<PoolState>>,
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
        Self::with_config(PoolConfig {
            max_size,
            ..Default::default()
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
            state: Arc::new(Mutex::new(PoolState::default())),
            config,
        })
    }

    /// 加锁获取内部状态
    ///
    /// 持锁期间只做简单的状态更新，即使锁被毒化状态也是一致的，
    /// 因此直接恢复使用（也避免在 `Drop` 中 panic）。
    fn lock(&self) -> MutexGuard<'_, PoolState> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// 从池中获取连接
    ///
    /// 优先复用空闲连接；没有空闲连接且未达上限时新建连接。
    /// 新建连接（网络握手）在锁外进行，不会阻塞其他线程。
    ///
    /// # 返回
    ///
    /// - `Ok(PooledConn)`: 获取成功
    /// - `Err(std::io::Error)`: 获取失败（池已满时为 `WouldBlock`）
    ///
    /// # 示例
    ///
    /// ```rust,no_run
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
        self.get_connection_with(Tcp::new)
    }

    /// 获取连接，需要新建时调用 `connect`（便于不联网测试）
    fn get_connection_with(
        &self,
        connect: impl FnOnce() -> Result<Tcp, std::io::Error>,
    ) -> Result<PooledConn, std::io::Error> {
        let generation = {
            let mut state = self.lock();

            // 清理过期的空闲连接
            state.idle.retain(|conn| conn.is_healthy(&self.config));

            // 复用空闲连接
            if let Some(conn) = state.idle.pop() {
                state.active += 1;
                return Ok(PooledConn {
                    pool: self.clone(),
                    tcp: Some(conn.tcp),
                    created_at: conn.created_at,
                    generation: state.generation,
                });
            }

            // 池已满，返回错误
            if state.idle.len() + state.active >= self.config.max_size {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::WouldBlock,
                    "Connection pool exhausted",
                ));
            }

            // 先预留名额，释放锁后再建立连接
            state.active += 1;
            state.generation
        };

        match connect() {
            Ok(tcp) => Ok(PooledConn {
                pool: self.clone(),
                tcp: Some(tcp),
                created_at: Instant::now(),
                generation,
            }),
            Err(e) => {
                // 建连失败，退还预留的名额
                self.release(generation);
                Err(e)
            }
        }
    }

    /// 退还一个借出名额；返回 `false` 表示池已被 `close()`，名额早已清零
    fn release(&self, generation: u64) -> bool {
        let mut state = self.lock();
        if state.generation != generation {
            return false;
        }
        state.active -= 1;
        true
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
        let state = self.lock();

        PoolStats {
            total: state.idle.len() + state.active,
            active: state.active,
            idle: state.idle.len(),
            max_size: self.config.max_size,
        }
    }

    /// 关闭连接池
    ///
    /// 关闭所有空闲连接并清零统计；仍被借出的连接在归还时直接关闭，
    /// 不会再回到池中。
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
        let mut state = self.lock();
        state.idle.clear();
        state.active = 0;
        state.generation += 1;
    }
}

/// 池化连接包装器
///
/// 当离开作用域时，自动归还连接到池中
///
/// # 示例
///
/// ```rust,no_run
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
    /// 直接持有TCP连接；仅在 `Drop` 归还时被取走
    tcp: Option<Tcp>,
    /// 连接创建时间
    created_at: Instant,
    /// 借出时连接池的代数
    generation: u64,
}

impl PooledConn {
    /// 获取内部TCP连接的可变引用
    ///
    /// # 示例
    ///
    /// ```rust,no_run
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
        self.tcp.as_mut().expect("连接仅在 Drop 时被取走")
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
    /// ```rust,no_run
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
        let tcp = self.tcp.take();
        let mut state = self.pool.lock();

        // 池已被 close()：名额已清零，连接直接丢弃
        if state.generation != self.generation {
            return;
        }
        state.active -= 1;

        // 归还到空闲列表；超过最大生命周期的连接直接丢弃
        if let Some(tcp) = tcp
            && !lifetime_expired(self.created_at, &self.pool.config)
        {
            state.idle.push(IdleConn {
                tcp,
                created_at: self.created_at,
                last_used: Instant::now(),
            });
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
    use crate::tcp::{HANDSHAKE, TcpConfig};
    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpListener};
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// 设置了 `RUSTDX_SKIP_INTEGRATION_TESTS` 时跳过联网测试
    fn skip_network() -> bool {
        if std::env::var("RUSTDX_SKIP_INTEGRATION_TESTS").is_ok() {
            println!("⚠️  跳过集成测试 (RUSTDX_SKIP_INTEGRATION_TESTS 已设置)");
            return true;
        }
        false
    }

    /// 本地假行情服务器：接受任意数量的连接，完成握手后保持连接不断开
    fn fake_server() -> SocketAddr {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            let mut alive = Vec::new();
            for stream in listener.incoming() {
                let mut stream = stream.unwrap();
                for &pack in HANDSHAKE {
                    let mut request = vec![0; pack.len()];
                    stream.read_exact(&mut request).unwrap();
                    stream.write_all(&[0; 16]).unwrap();
                }
                alive.push(stream);
            }
        });
        addr
    }

    /// 连接本地假服务器
    fn connect_fake(addr: SocketAddr) -> Result<Tcp, std::io::Error> {
        let config = TcpConfig {
            timeout: Duration::from_secs(2),
            ..Default::default()
        };
        Tcp::with_servers(&config, &[addr])
    }

    /// 连接的本地端口，用于区分不同的 Tcp
    fn local_port(conn: &mut PooledConn) -> u16 {
        conn.get_mut().get_ref().0.local_addr().unwrap().port()
    }

    fn assert_stats(pool: &ConnectionPool, total: usize, active: usize, idle: usize) {
        let stats = pool.stats();
        assert_eq!(
            (stats.total, stats.active, stats.idle),
            (total, active, idle)
        );
    }

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
        if skip_network() {
            return;
        }
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
        if skip_network() {
            return;
        }
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
        if skip_network() {
            return;
        }
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
        if skip_network() {
            return;
        }
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
        if skip_network() {
            return;
        }
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

    // ------------------------------------------------------------------
    // 以下测试使用本地假服务器，不依赖外网
    // ------------------------------------------------------------------

    /// 回归：清理过期空闲连接后，仍被借出的连接归还不能错位，名额不能泄漏
    #[test]
    fn test_return_after_idle_cleanup_keeps_accounting() {
        let addr = fake_server();
        let pool = ConnectionPool::with_config(PoolConfig {
            max_size: 2,
            max_idle: 0, // 空闲连接立即过期
            ..Default::default()
        })
        .unwrap();
        let connect = || connect_fake(addr);

        let a = pool.get_connection_with(connect).unwrap();
        let mut b = pool.get_connection_with(connect).unwrap();
        let b_port = local_port(&mut b);
        drop(a);
        assert_stats(&pool, 2, 1, 1);
        std::thread::sleep(Duration::from_millis(5));

        // a 已过期被清理，c 是新建连接；此时 b 仍被借出
        let mut c = pool.get_connection_with(connect).unwrap();
        let c_port = local_port(&mut c);
        assert_ne!(b_port, c_port);
        assert_stats(&pool, 2, 2, 0);

        // 归还 b：不影响 c，b 自己进入空闲列表
        drop(b);
        assert_stats(&pool, 2, 1, 1);
        assert_eq!(local_port(&mut c), c_port);
        drop(c);
        assert_stats(&pool, 2, 0, 2);

        // 名额没有泄漏：过期连接被清理后仍能借满 max_size 个
        std::thread::sleep(Duration::from_millis(5));
        let _d = pool.get_connection_with(connect).unwrap();
        let _e = pool.get_connection_with(connect).unwrap();
        assert_stats(&pool, 2, 2, 0);
        let err = pool.get_connection_with(connect).err().unwrap();
        assert_eq!(err.kind(), std::io::ErrorKind::WouldBlock);
    }

    /// 复用空闲连接时不新建连接，拿到的是同一个 Tcp
    #[test]
    fn test_idle_connection_is_reused_without_connecting() {
        let addr = fake_server();
        let pool = ConnectionPool::new(2).unwrap();
        let connects = AtomicUsize::new(0);
        let connect = || {
            connects.fetch_add(1, Ordering::SeqCst);
            connect_fake(addr)
        };

        let mut first = pool.get_connection_with(connect).unwrap();
        let port = local_port(&mut first);
        drop(first);
        assert_stats(&pool, 1, 0, 1);

        let mut second = pool.get_connection_with(connect).unwrap();
        assert_eq!(local_port(&mut second), port);
        assert_eq!(connects.load(Ordering::SeqCst), 1);
        assert_stats(&pool, 1, 1, 0);
        assert_eq!(second.execute(|_tcp| 42), 42);
    }

    /// 建连期间不持有池锁，且名额已预留
    #[test]
    fn test_connect_runs_without_pool_lock() {
        let addr = fake_server();
        let pool = ConnectionPool::new(1).unwrap();

        let conn = pool
            .get_connection_with(|| {
                // 若建连时仍持有锁，这里会死锁
                assert_stats(&pool, 1, 1, 0);
                // 名额已被预留，并发的获取请求应立即得到"池已满"
                let err = pool
                    .get_connection_with(|| unreachable!("池已满时不应建连"))
                    .err()
                    .unwrap();
                assert_eq!(err.kind(), std::io::ErrorKind::WouldBlock);
                connect_fake(addr)
            })
            .unwrap();
        assert_stats(&pool, 1, 1, 0);
        drop(conn);
        assert_stats(&pool, 1, 0, 1);
    }

    /// 建连失败时退还预留名额
    #[test]
    fn test_connect_failure_releases_reservation() {
        let addr = fake_server();
        let pool = ConnectionPool::new(1).unwrap();

        let err = pool
            .get_connection_with(|| Err(std::io::ErrorKind::ConnectionRefused.into()))
            .err()
            .unwrap();
        assert_eq!(err.kind(), std::io::ErrorKind::ConnectionRefused);
        assert_stats(&pool, 0, 0, 0);

        let _conn = pool.get_connection_with(|| connect_fake(addr)).unwrap();
        assert_stats(&pool, 1, 1, 0);
    }

    /// close() 之后归还的连接被丢弃，统计不会变成负数或回到池中
    #[test]
    fn test_close_discards_borrowed_connections() {
        let addr = fake_server();
        let pool = ConnectionPool::new(2).unwrap();
        let connect = || connect_fake(addr);

        let borrowed = pool.get_connection_with(connect).unwrap();
        drop(pool.get_connection_with(connect).unwrap());
        assert_stats(&pool, 2, 1, 1);

        pool.close();
        assert_stats(&pool, 0, 0, 0);
        drop(borrowed);
        assert_stats(&pool, 0, 0, 0);

        // 关闭后池仍可继续使用
        let _conn = pool.get_connection_with(connect).unwrap();
        assert_stats(&pool, 1, 1, 0);
    }

    /// 超过最大生命周期的连接归还时直接丢弃
    #[test]
    fn test_expired_lifetime_connection_is_dropped_on_return() {
        let addr = fake_server();
        let pool = ConnectionPool::with_config(PoolConfig {
            max_size: 1,
            max_lifetime: 0,
            ..Default::default()
        })
        .unwrap();

        let conn = pool.get_connection_with(|| connect_fake(addr)).unwrap();
        std::thread::sleep(Duration::from_millis(5));
        drop(conn);
        assert_stats(&pool, 0, 0, 0);
    }
}
