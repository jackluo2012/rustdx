use crate::bytes_helper::u16_from_le_bytes;
use log::trace;
use std::{
    io::{BufReader, Read, Result, Write},
    net::{SocketAddr, TcpStream},
    time::{Duration, Instant},
};

#[macro_use]
#[cfg(test)]
pub(crate) mod tests;

#[cfg(test)]
mod resilience_tests;

#[cfg(test)]
mod deadline_tests;

mod basic;
pub use basic::*;

pub mod helper;
pub mod ip;

pub mod stock;

/// 用于缓冲读取 TcpStream 的数据。
pub type BufTcp = BufReader<TcpStream>;

/// 连接配置：超时与指定服务器。
///
/// - [`Tcp::new`] 使用默认配置（5 秒超时，按 [`ip::STOCK_IP`] 列表故障转移）；
/// - [`Tcp::with_config`] 可指定服务器与超时（`ip = None` 时依然按列表故障转移）。
#[derive(Debug, Clone)]
pub struct TcpConfig {
    /// 连接与读写的超时时间。
    pub timeout: Duration,
    /// 指定服务器地址。`None` 表示使用 [`ip::STOCK_IP`] 列表自动选择。
    pub ip: Option<SocketAddr>,
    /// 请求总尝试次数（含首次）；`0` 和 `1` 均不重试，`2` 表示失败后重连重发一次。
    /// 保留既有计数语义；与 pytdx 的四次重试策略不同。
    ///
    /// 默认 `2`：行情请求幂等，失败（断线、超时、响应畸形）后换下一台服务器
    /// 重试一次——严格解析会把异常响应报成错误，默认不重试的话调用方要自己
    /// 处理换服务器。需要旧行为（失败即返回）时显式设为 `0`。
    pub auto_reconnect: usize,
    /// 重试前的退避间隔基数（毫秒）。第 `n` 次失败后等待
    /// `retry_delay_ms × n` 再重连重试（线性退避）。
    ///
    /// 代理/高并发环境下「立即重试」常连续失败（实测 TUN fake-IP 环境
    /// 首连易 `EAGAIN`），建议 200~400。默认 `0` = 立即重试（旧行为）。
    pub retry_delay_ms: u64,
    /// K 线区间拉取（[`crate::tcp::stock::Client::k`] /
    /// [`crate::tcp::stock::Client::bars_range`]）空结果二次确认。
    ///
    /// 历史上观察到服务端会**静默返回空响应**（不报错），与停牌股
    /// 「窗口内本就无 K 线」的真空无法从返回值区分。其中多数后来查明是握手
    /// 发送 `PACK3` 导致的权限降级（已修复，见 [`HANDSHAKE`]），且 K 线的异常
    /// 响应现在会报错而非返回空；本选项保留作为兜底。开启后：空结果时
    /// 等待 300ms 并重连，重拉一次，仍空才返回空——以停牌股约 300ms/只
    /// 的代价换取全市场扫描不被静默限流污染。默认 `false`（旧行为）。
    pub recheck_empty: bool,
}

impl Default for TcpConfig {
    fn default() -> Self {
        Self {
            timeout: TIMEOUT,
            ip: None,
            auto_reconnect: 2,
            retry_delay_ms: 0,
            recheck_empty: false,
        }
    }
}

impl TcpConfig {
    /// 使用列表中第 `index` 个服务器（越界时回退到第一个）。
    pub fn with_index(index: usize, timeout: Duration) -> Self {
        Self {
            timeout,
            ip: Some(ip::STOCK_IP.get(index).copied().unwrap_or(ip::STOCK_IP[0])),
            ..Default::default()
        }
    }
}

/// Tcp 链接的一层封装。
#[derive(Debug)]
pub struct Tcp {
    stream: TcpStream,
    buffer: BufTcp,
    recv: [u8; RECV_SIZE],
    /// 创建本连接所用的配置，[`Tcp::reconnect`] 时使用。
    config: TcpConfig,
    /// Optional ordered failover pool; explicit `with_config(ip=Some)` stays pinned.
    servers: Vec<SocketAddr>,
    deadline: Option<Instant>,
    needs_reconnect: bool,
}

impl Tcp {
    /// 连接到行情服务器并完成握手（已发送 [`HANDSHAKE`] 握手包）。
    ///
    /// 按 [`ip::STOCK_IP`] 列表顺序依次尝试（故障转移），直到某台连接成功；
    /// 全部失败、或整轮超过 3 倍 `timeout`（默认 15 秒）时返回最后一个错误。
    ///
    /// 超时默认 5 秒，可用 [`Tcp::with_config`] 自定义。
    pub fn new() -> Result<Self> {
        Self::with_config(&TcpConfig::default())
    }

    /// 以指定配置连接服务器并完成握手（已发送 [`HANDSHAKE`] 握手包）。
    ///
    /// - `config.ip` 为 `Some(addr)` 时只连接该地址；
    /// - 为 `None` 时按 [`ip::STOCK_IP`] 列表顺序故障转移——TCP 连接与协议握手
    ///   都成功才算可用（部分服务器 TCP 可连但不响应行情协议）。
    pub fn with_config(config: &TcpConfig) -> Result<Self> {
        Self::with_config_inner(config, None)
    }

    /// Connect and handshake within one absolute deadline, shared by all servers.
    pub fn with_config_until(config: &TcpConfig, deadline: Instant) -> Result<Self> {
        Self::with_config_inner(config, Some(deadline))
    }

    fn with_config_inner(config: &TcpConfig, deadline: Option<Instant>) -> Result<Self> {
        match config.ip {
            Some(addr) => Self::connect_addr(&addr, config, deadline),
            None => Self::with_servers_inner(config, &ip::preferred_first(), deadline),
        }
    }

    /// Connect using an ordered server pool; reconnect rotates after the current server.
    /// `config.ip` is ignored; use `with_config` for a pinned connection.
    pub fn with_servers(config: &TcpConfig, servers: &[SocketAddr]) -> Result<Self> {
        Self::with_servers_inner(config, servers, None)
    }

    /// Connect using an ordered pool within one absolute deadline. Reconnect,
    /// handshake, request I/O, retries and backoff retain this same deadline.
    pub fn with_servers_until(
        config: &TcpConfig,
        servers: &[SocketAddr],
        deadline: Instant,
    ) -> Result<Self> {
        Self::with_servers_inner(config, servers, Some(deadline))
    }

    fn with_servers_inner(
        config: &TcpConfig,
        servers: &[SocketAddr],
        deadline: Option<Instant>,
    ) -> Result<Self> {
        // 没有显式截止时间时，整轮故障转移也要有上限：否则网络黑洞（连接不被
        // 拒绝、只是不回应）下要把每台服务器都等满一个超时，40 台就是几分钟。
        let scan_deadline = deadline.or_else(|| {
            Instant::now().checked_add(config.timeout.saturating_mul(FAILOVER_BUDGET_TIMEOUTS))
        });
        let mut last_err = None;
        for addr in servers {
            if let Err(e) = operation_timeout(config.timeout, scan_deadline) {
                if deadline.is_some() || last_err.is_none() {
                    return Err(e); // 显式截止时间已到，或超时配置本身非法
                }
                break; // 隐式总时限用尽：返回最后一台的连接错误
            }
            let mut candidate = config.clone();
            candidate.ip = Some(*addr);
            match Self::connect_addr(addr, &candidate, scan_deadline) {
                Ok(mut tcp) => {
                    ip::remember(addr);
                    tcp.servers = servers.to_vec();
                    tcp.deadline = deadline; // 隐式总时限只约束建连，不带入后续请求
                    return Ok(tcp);
                }
                Err(e) => last_err = Some(e),
            }
        }
        operation_timeout(config.timeout, deadline)?;
        Err(last_err.unwrap_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "empty server pool")
        }))
    }

    fn connect_addr(
        addr: &SocketAddr,
        config: &TcpConfig,
        deadline: Option<Instant>,
    ) -> Result<Self> {
        let timeout = operation_timeout(config.timeout, deadline)?;
        let (stream, buffer, recv) = tcpstream_ip_with_timeout(addr, timeout)?;
        let mut tcp = Self {
            stream,
            buffer,
            recv,
            config: config.clone(),
            servers: Vec::new(),
            deadline,
            needs_reconnect: false,
        };
        // Never recursively retry a failed handshake.
        tcp.config.auto_reconnect = 0;
        send_packs(&mut tcp, false)?;
        tcp.config = config.clone();
        tcp.check_deadline()?;
        Ok(tcp)
    }

    /// Set the budget for the next synchronous operation. Use a fresh absolute
    /// deadline each polling round; None restores per-I/O timeouts only.
    /// An interrupted frame is discarded and reconnected before the next request.
    pub fn set_deadline(&mut self, deadline: Option<Instant>) {
        self.deadline = deadline;
    }

    fn invalidate(&mut self) {
        self.needs_reconnect = true;
        let _ = self.stream.shutdown(std::net::Shutdown::Both);
    }

    fn check_deadline(&mut self) -> Result<()> {
        if self.deadline.is_some_and(|at| Instant::now() >= at) {
            self.invalidate();
            return Err(deadline_error());
        }
        Ok(())
    }

    fn io_timeout(&mut self) -> Result<Duration> {
        self.check_deadline()?;
        operation_timeout(self.config.timeout, self.deadline)
    }

    pub(crate) fn sleep_with_deadline(&mut self, duration: Duration) -> Result<()> {
        self.check_deadline()?;
        let sleep = self
            .deadline
            .map(|at| duration.min(at.saturating_duration_since(Instant::now())))
            .unwrap_or(duration);
        std::thread::sleep(sleep);
        self.check_deadline()
    }

    fn read_exact_until(&mut self, output: &mut [u8]) -> Result<()> {
        let result = (|| {
            let mut offset = 0;
            while offset < output.len() {
                let timeout = self.io_timeout()?;
                self.buffer.get_ref().set_read_timeout(Some(timeout))?;
                match self.buffer.read(&mut output[offset..]) {
                    Ok(0) => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "TDX response closed",
                        ));
                    }
                    Ok(n) => offset += n,
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e),
                }
            }
            self.check_deadline()
        })();
        if result.is_err() {
            self.invalidate();
        }
        result
    }

    fn write_all_until(&mut self, input: &[u8]) -> Result<()> {
        let result = (|| {
            let mut offset = 0;
            while offset < input.len() {
                let timeout = self.io_timeout()?;
                self.stream.set_write_timeout(Some(timeout))?;
                match self.stream.write(&input[offset..]) {
                    Ok(0) => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::WriteZero,
                            "TDX request write returned zero",
                        ));
                    }
                    Ok(n) => offset += n,
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e),
                }
            }
            self.check_deadline()
        })();
        if result.is_err() {
            self.invalidate();
        }
        result
    }

    /// 当前连接配置（[`TcpConfig::recheck_empty`] 等高级选项由此读取）。
    pub fn config(&self) -> &TcpConfig {
        &self.config
    }

    /// 重新建立连接（按创建时的配置），并重发握手包。原连接被丢弃。
    ///
    /// ## 示例
    /// ```ignore
    /// tcp.reconnect()?; // 断线后重新连接
    /// ```
    pub fn reconnect(&mut self) -> Result<()> {
        self.check_deadline()?;
        if self.servers.is_empty() {
            *self = Self::with_config_inner(&self.config, self.deadline)?;
        } else {
            let mut servers = self.servers.clone();
            if let Some(index) = servers
                .iter()
                .position(|addr| Some(*addr) == self.config.ip)
            {
                let len = servers.len();
                servers.rotate_left((index + 1) % len);
            }
            *self = Self::with_servers_inner(&self.config, &servers, self.deadline)?;
        }
        Ok(())
    }

    /// 发送心跳包保持连接活跃（对应 mootdx 的 `heartbeat`）。
    ///
    /// 通达信服务器会关闭长时间空闲的连接；在空闲场景下定期调用即可。
    /// 返回心跳响应中的深市证券数量。
    pub fn heartbeat(&mut self) -> Result<u16> {
        let mut hb = SecurityCount::new(0);
        Ok(*hb.recv_parsed(self)?)
    }

    /// 执行请求，失败时自动重连并重试（对应 mootdx 的 `auto_retry`）。
    ///
    /// ## 示例
    /// ```ignore
    /// let quotes = tcp.retry(|tcp| quotes.recv_parsed(tcp).copied(), 3)?;
    /// ```
    pub fn retry<T>(
        &mut self,
        mut f: impl FnMut(&mut Tcp) -> Result<T>,
        attempts: usize,
    ) -> Result<T> {
        if attempts == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "attempts must be positive",
            ));
        }
        let mut last_err = None;
        for attempt in 0..attempts {
            self.check_deadline()?;
            match f(self) {
                Ok(v) => return Ok(v),
                Err(e) => {
                    if attempt + 1 < attempts {
                        self.backoff(attempt as u64 + 1)?;
                        self.reconnect()?;
                    }
                    last_err = Some(e);
                }
            }
        }
        Err(last_err.unwrap())
    }

    fn without_auto_retry<T>(&mut self, f: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        let configured = self.config.auto_reconnect;
        self.config.auto_reconnect = 0;
        // Preserve configuration even if a legacy parser panics; rethrow the panic.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(self)));
        self.config.auto_reconnect = configured;
        match result {
            Ok(result) => result,
            Err(payload) => std::panic::resume_unwind(payload),
        }
    }

    /// 重试退避：第 `n` 次失败后按 `retry_delay_ms × n` 等待（配置为 0 时不等待）。
    fn backoff(&mut self, failed: u64) -> Result<()> {
        self.sleep_with_deadline(Duration::from_millis(
            self.config.retry_delay_ms.saturating_mul(failed),
        ))
    }

    /// 发送并接收字节。需要对接收的字节进行解析（参考 [`Tdx::parse`] 的实现）。
    ///
    /// 方法返回发送和读取的字节数。
    ///
    /// 注意是读取而不是接收的字节数。
    /// 由于每次接收先读取 16 字节，所以返回的元组中，第二个数字应为 16。
    ///
    /// 配置了 [`TcpConfig::auto_reconnect`] 时，请求失败会自动重连并按原请求
    /// 字节重发（行情查询请求幂等，可安全重试）。
    pub fn send_recv(&mut self, send: &[u8]) -> Result<(usize, usize)> {
        let attempts = self.config.auto_reconnect.max(1);
        self.retry(|tcp| tcp.send_recv_once(send), attempts)
    }

    fn send_recv_once(&mut self, send: &[u8]) -> Result<(usize, usize)> {
        self.check_deadline()?;
        if self.needs_reconnect {
            self.reconnect()?;
        }
        self.write_all_until(send)?;
        let mut header = [0; RECV_SIZE];
        self.read_exact_until(&mut header)?;
        self.recv = header;
        trace!("send: {:?}\nrecv[16B]: {:?}", send, self);
        Ok((send.len(), RECV_SIZE))
    }

    pub fn into_inner(self) -> (TcpStream, BufTcp, [u8; RECV_SIZE]) {
        (self.stream, self.buffer, self.recv)
    }

    pub fn get_ref(&self) -> (&TcpStream, &BufTcp, &[u8]) {
        (&self.stream, &self.buffer, &self.recv)
    }

    pub fn get_ref_recv(&self) -> &[u8] {
        &self.recv
    }
}

pub trait Tdx {
    /// 待发送的字节。所有发送请求的字节由两部分组成：
    /// 1. 固定的默认字节（基本为前半段字节）
    /// 2. 查询所需的变化的字节（基本为后半段字节）
    ///
    /// [`Tdx::SEND`] 的作用为默认有效的字节，即发送字节之后会正常响应且数据可以解析，
    /// 但是并不保证每次响应的数据完全一致。
    /// 比如请求日线时发送字节在当天收盘后是不变的，次日交易日请求得到的数据则可能改变。
    ///
    /// 如果发送请求的字节有误，则无法得到响应
    /// （比如设置了读取超时，无响应情况下会得到：
    /// [`WouldBlock` error][`std::io::ErrorKind::WouldBlock`]）。
    ///
    /// 字节具体的含义见 Implementor 的 Tdx trait 部分的 `SEND` 文档。
    const SEND: &'static [u8];
    /// 描述此次 tcp 连接的用途，目前用于记录日志。
    const TAG: &'static str;
    /// 发送的字节的长度。每种请求所发送的字节长度是已知的。默认为 [`Tdx::SEND`] 的长度。
    const LEN: usize = Self::SEND.len();
    type Item: ?Sized;

    /// 真正发送的字节。
    fn send(&mut self) -> &[u8];

    /// 得到响应的字节。响应的字节长度无法预测。
    fn recv(&mut self, tcp: &mut Tcp) -> Result<Vec<u8>> {
        send_recv_decompress(tcp, self.send(), Self::TAG)
    }

    /// 解析响应的字节。
    fn parse(&mut self, response: Vec<u8>);

    /// 得到和解析响应的字节，并返回解析的数据。
    fn recv_parsed(&mut self, tcp: &mut Tcp) -> Result<&Self::Item> {
        let attempts = tcp.config.auto_reconnect.max(1);
        tcp.retry(
            |tcp| {
                // Like pytdx's API decorator, retry receive + decode + fallible parse
                // as one operation. Suppress inner transport retries to avoid N² attempts.
                tcp.without_auto_retry(|tcp| {
                    let response = self.recv(tcp)?;
                    self.try_parse(response)?;
                    tcp.check_deadline()
                })
            },
            attempts,
        )?;
        Ok(self.result())
    }

    /// Fallible parser hook. Implementations handling untrusted data can reject
    /// malformed responses without panicking or returning partial data.
    fn try_parse(&mut self, response: Vec<u8>) -> Result<()> {
        self.parse(response);
        Ok(())
    }

    /// 解析后的结果。可以是引用，也可以是 owned type 。
    fn result(&self) -> &Self::Item;
}

/// 此函数完成以下事情：
/// 1. 接收响应的字节，并且在 debug 模式下验证有效数据的长度
/// 2. 根据响应信息的解压前后长度，进行数据解压
/// 3. 消耗缓冲区的字节（否则下次 read 的内容是上次未读/未消耗的字节），返回有效数据
///
/// 有效数据：包含实际有用信息的数据。
pub fn send_recv_decompress(tcp: &mut Tcp, send: &[u8], tag: &str) -> Result<Vec<u8>> {
    let attempts = tcp.config.auto_reconnect.max(1);
    tcp.retry(
        |tcp| {
            let (buf, deflate_size, inflate_size) = read_response_once(tcp, send, tag)?;
            let decoded = decode_response(buf, deflate_size, inflate_size)?;
            tcp.check_deadline()?;
            Ok(decoded)
        },
        attempts,
    )
}

fn decode_response(buf: Vec<u8>, deflate_size: u16, inflate_size: u16) -> Result<Vec<u8>> {
    let invalid = |message| std::io::Error::new(std::io::ErrorKind::InvalidData, message);
    let decoded = if deflate_size != inflate_size {
        // Bound allocation by the declared (u16) protocol size as well as handling
        // corrupt zlib without unwinding through a caller's connection mutex.
        miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(&buf, inflate_size as usize)
            .map_err(|_| invalid("invalid or oversized compressed TDX response"))?
    } else {
        buf
    };
    if decoded.len() != inflate_size as usize {
        return Err(invalid("TDX response length mismatch"));
    }
    Ok(decoded)
}

// 由于只读取了前 16 字节（TCP_RECV_SIZE），
// 剩下的未读字节需要使用内部缓冲区消耗掉，
// 否则下次 read 的内容是上次的未读字节。
// 如果不使用 BufReader ，那么需要手动 read 剩余字节。
// 对于 TcpStream ，Write::flush 没有做任何事情，所以无需调用。
pub fn send_recv(tcp: &mut Tcp, send: &[u8], tag: &str) -> Result<(Vec<u8>, u16, u16)> {
    let attempts = tcp.config.auto_reconnect.max(1);
    tcp.retry(|tcp| read_response_once(tcp, send, tag), attempts)
}

fn read_response_once(tcp: &mut Tcp, send: &[u8], tag: &str) -> Result<(Vec<u8>, u16, u16)> {
    tcp.send_recv_once(send)?;
    trace!("{tag}\nsend: {:?}\nrecv[16B]: {:?}", send, tcp);

    let deflate_size = u16_from_le_bytes(&tcp.recv, 12); // 响应信息中的待解压长度
    let mut buf = vec![0; deflate_size as usize];
    tcp.read_exact_until(&mut buf)?;

    let inflate_size = u16_from_le_bytes(&tcp.recv, 14); // 响应信息中的解压后长度
    #[rustfmt::skip]
    trace!("\n解压前：#{:?}# -> {}，解压后：#{:?}# -> {}\n剩余数据（即解压前）：{:x?}\n",
           &tcp.recv[12..14], deflate_size, &tcp.recv[14..16], inflate_size, buf);

    Ok((buf, deflate_size, inflate_size))
}

fn deadline_error() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::TimedOut,
        "TDX operation deadline exceeded",
    )
}

fn operation_timeout(timeout: Duration, deadline: Option<Instant>) -> Result<Duration> {
    if timeout.is_zero() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "timeout must be positive",
        ));
    }
    match deadline {
        Some(at) => {
            let remaining = at.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                Err(deadline_error())
            } else {
                Ok(timeout.min(remaining))
            }
        }
        None => Ok(timeout),
    }
}

/// 默认的超时值。
pub const TIMEOUT: Duration = Duration::from_secs(5);

/// 未指定截止时间时，一轮故障转移（建连 + 握手）的总时限，以 `timeout` 的倍数计。
/// 默认配置下即 15 秒：足够跳过两台不回应的服务器再连上第三台。
const FAILOVER_BUDGET_TIMEOUTS: u32 = 3;

/// 快速引入 tcpstream（按服务器列表故障转移），设置 [`TIMEOUT`] 超时。
pub fn tcpstream() -> Result<(TcpStream, BufTcp, [u8; RECV_SIZE])> {
    tcpstream_with_timeout(TIMEOUT)
}

/// 按服务器列表顺序尝试建立连接（故障转移），每个尝试使用 `timeout`。
/// 全部失败时返回最后一个错误。
pub fn tcpstream_with_timeout(timeout: Duration) -> Result<(TcpStream, BufTcp, [u8; RECV_SIZE])> {
    let mut last_err = None;
    for addr in ip::STOCK_IP.iter() {
        match tcpstream_ip_with_timeout(addr, timeout) {
            Ok(x) => return Ok(x),
            Err(e) => last_err = Some((addr, e)),
        }
    }
    let (addr, err) = last_err.expect("服务器列表不能为空");
    log::warn!("所有行情服务器连接失败，最后尝试: {addr}");
    Err(err)
}

/// 快速引入 tcpstream（指定服务器），设置 [`TIMEOUT`] 超时。
pub fn tcpstream_ip(ip: &SocketAddr) -> Result<(TcpStream, BufTcp, [u8; RECV_SIZE])> {
    tcpstream_ip_with_timeout(ip, TIMEOUT)
}

/// 快速引入 tcpstream（指定服务器与超时）。
pub fn tcpstream_ip_with_timeout(
    ip: &SocketAddr,
    timeout: Duration,
) -> Result<(TcpStream, BufTcp, [u8; RECV_SIZE])> {
    let stream = TcpStream::connect_timeout(ip, timeout)?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    let recv = [0; RECV_SIZE];
    let buffer = BufReader::new(stream.try_clone()?);
    Ok((stream, buffer, recv))
}
