use crate::bytes_helper::u16_from_le_bytes;
use crate::tcp::Tdx;

/// 获取股票分时数据。对应于 pytdx 中的 hq.get_minute_time_data、GetMinuteTimeDataCmd。
///
/// ## 注意
/// - 返回当天的分时成交数据（每分钟一个数据点）
/// - 通常返回240个数据点（4小时交易时间）
/// - market: 0=深市, 1=沪市
///
/// ## 示例
/// ```ignore
/// use rustdx::tcp::{Tcp, Tdx};
/// use rustdx::tcp::stock::MinuteTime;
///
/// let mut tcp = Tcp::new()?;
/// let mut minute = MinuteTime::new(0, "000001");
/// minute.recv_parsed(&mut tcp)?;
/// for data in minute.result().iter().take(10) {
///     println!("价格: {:.2}, 成交量: {}", data.price, data.vol);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct MinuteTime<'d> {
    pub send: Box<[u8]>,
    pub market: u16,
    pub code: &'d str,
    pub response: Vec<u8>,
    pub data: Vec<MinuteTimeData>,
}

impl<'d> MinuteTime<'d> {
    /// 创建一个新的分时数据请求。
    ///
    /// ## 参数
    /// - `market`: 市场代码（0=深市, 1=沪市）
    /// - `code`: 6位股票代码
    pub fn new(market: u16, code: &'d str) -> Self {
        assert_eq!(code.len(), 6, "股票代码必须是6位");

        let mut send = [0u8; Self::LEN];
        // 复制包头（12字节）
        send[0..12].copy_from_slice(Self::SEND);

        // 设置market（字节12-13）
        send[12..14].copy_from_slice(&market.to_le_bytes());
        // 设置code（字节14-19）
        send[14..20].copy_from_slice(code.as_bytes());
        // 字节20-23：设置为0

        Self {
            send: send.into(),
            market,
            code,
            response: Vec::new(),
            data: Vec::new(),
        }
    }
}

impl<'a> Tdx for MinuteTime<'a> {
    type Item = [MinuteTimeData];

    /// 获取分时数据的请求字节。
    ///
    /// ## 协议格式（基于pytdx源码分析）
    /// - 前12字节：固定包头
    /// - 字节12-13：market（市场代码）
    /// - 字节14-19：code（股票代码，6字节）
    /// - 字节20-23：0
    const SEND: &'static [u8] = &[
        0x0c, 0x1b, 0x08, 0x00, 0x01, 0x01, 0x0e, 0x00, 0x0e, 0x00, 0x1d,
        0x05, // 固定包头（12字节）
    ];

    const TAG: &'static str = "分时数据";
    const LEN: usize = 12 + 2 + 6 + 4; // 固定长度：包头12字节 + market(2) + code(6) + 0(4)

    fn send(&mut self) -> &[u8] {
        &self.send
    }

    /// 解析响应的字节。
    ///
    /// ## 响应格式（基于 pytdx GetMinuteTimeDataCmd.parseResponse）
    /// - 前2字节：数据点数量
    /// - 字节2-3：跳过
    /// - 之后每个数据点：price_raw / reversed1 / vol 三个变长编码字段
    ///
    /// ## ⚠️ 已知问题（2026 起）
    /// 部分通达信服务器的分时响应格式已发生变化（数据前新增了 market + code 头，
    /// 数据编码亦与旧格式不符），pytdx 同样无法解析。本方法内置了防御性校验：
    /// 解析结果异常（点数不齐、价格不合理、字节未耗尽）时返回**空数据**而非垃圾数据。
    /// 参见 <https://github.com/rainx/pytdx/issues/148>。
    fn parse(&mut self, v: Vec<u8>) {
        self.data = Vec::new();

        if v.len() < 4 {
            self.response = v;
            return;
        }

        // 读取数据点数量
        let num_points = u16_from_le_bytes(&v, 0);
        let mut pos = 4; // 跳过前4字节（数量 + 2字节跳过）

        let mut last_price = 0i32;

        for _ in 0..num_points {
            // 变长编码最多 5 字节，每点 3 个字段；数据不足时立即停止
            if v.len() - pos < 3 {
                break;
            }
            let Some(price_raw) = price_checked(&v, &mut pos) else {
                self.response = v;
                return; // 字节越界：格式不符，返回空数据
            };
            let Some(_reversed1) = price_checked(&v, &mut pos) else {
                self.response = v;
                return;
            };
            let Some(vol) = price_checked(&v, &mut pos) else {
                self.response = v;
                return;
            };

            // 累加计算实际价格
            last_price += price_raw;
            let price = last_price as f64 / 100.0;

            self.data.push(MinuteTimeData { price, vol });
        }

        // 防御性校验：
        // 1. 点数必须完整（协议变化时 varint 边界错位，点数几乎必然不齐）
        // 2. 响应字节应基本耗尽（允许少量尾部填充）
        // 3. 首点为开盘价，必须大于 0；所有价格需在 A 股合理范围内
        let points_complete = self.data.len() == num_points as usize;
        let bytes_consumed = points_complete && pos + 8 >= v.len();
        let prices_valid = self
            .data
            .iter()
            .all(|d| (0.01..=100000.0).contains(&d.price));

        if !points_complete || !bytes_consumed || !prices_valid {
            self.data = Vec::new(); // 协议不匹配，不输出垃圾数据
        }

        self.response = v;
    }

    fn result(&self) -> &Self::Item {
        &self.data
    }
}

/// 分时数据点。
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct MinuteTimeData {
    /// 价格（元）
    pub price: f64,
    /// 成交量（手）
    pub vol: i32,
}

/// [`price`] 的越界安全版本：字节不足时返回 `None`。
pub(crate) fn price_checked(arr: &[u8], pos: &mut usize) -> Option<i32> {
    let mut shl = 6;
    let mut bit = *arr.get(*pos)? as i32;
    let mut res = bit & 0x3f;
    let sign = (bit & 0x40) == 0;
    while (bit & 0x80) != 0 {
        *pos += 1;
        bit = *arr.get(*pos)? as i32;
        res += (bit & 0x7f) << shl;
        shl += 7;
    }
    *pos += 1;
    Some(if sign { res } else { -res })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minute_time_new() {
        let minute = MinuteTime::new(0, "000001");
        assert_eq!(minute.market, 0);
        assert_eq!(minute.code, "000001");
        assert_eq!(minute.send.len(), 24);
    }

    #[test]
    fn test_minute_time_new_shanghai() {
        let minute = MinuteTime::new(1, "600000");
        assert_eq!(minute.market, 1);
        assert_eq!(minute.code, "600000");
    }

    #[test]
    fn test_minute_time_send_bytes() {
        let minute = MinuteTime::new(0, "000001");
        // 验证包头
        assert_eq!(
            &minute.send[0..12],
            &[
                0x0c, 0x1b, 0x08, 0x00, 0x01, 0x01, 0x0e, 0x00, 0x0e, 0x00, 0x1d, 0x05
            ]
        );
        // 验证market
        assert_eq!(&minute.send[12..14], &[0x00, 0x00]);
        // 验证code
        assert_eq!(&minute.send[14..20], b"000001");
        // 验证最后的0
        assert_eq!(&minute.send[20..24], &[0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    #[should_panic(expected = "股票代码必须是6位")]
    fn test_minute_time_invalid_code() {
        MinuteTime::new(0, "00001");
    }

    #[test]
    fn test_connection() {
        // 跳过集成测试（需要实际网络连接）
        if std::env::var("RUSTDX_SKIP_INTEGRATION_TESTS").is_ok() {
            println!("⚠️  跳过集成测试 (RUSTDX_SKIP_INTEGRATION_TESTS 已设置)");
            return;
        }
        println!("⚠️  集成测试需要手动验证（需要实际TCP连接）");
    }

    /// 旧协议格式的正常解析：2 个点，价格累加还原。
    #[test]
    fn parse_legacy_format() {
        // num=2，跳过2字节；
        // 点1: price=+900(9.00元), rev1=0, vol=100；点2: price=+50, rev1=0, vol=80
        let mut v = vec![0x02, 0x00, 0x00, 0x00];
        v.extend_from_slice(&[0x84, 0x0e, 0x00, 0xa4, 0x01]); // +900, 0, 100
        v.extend_from_slice(&[0x32, 0x00, 0x90, 0x01]); // +50, 0, 80
        let mut mt = MinuteTime::new(0, "000001");
        mt.parse(v);
        assert_eq!(mt.result().len(), 2);
        assert_eq!(mt.result()[0].price, 9.0);
        assert_eq!(mt.result()[0].vol, 100);
        assert_eq!(mt.result()[1].price, 9.5);
        assert_eq!(mt.result()[1].vol, 80);
    }

    /// 2026-09-04 实测的新协议响应（服务器在数据前新增 market+code 头、编码已变），
    /// 旧解析逻辑产出垃圾数据。防御性校验应返回空数据而非垃圾。
    #[test]
    fn parse_new_protocol_returns_empty() {
        let hex = "1f000000003030303030319d02a81244460847a5f1c409e8128cf21cba0208a4864d84da0c89981000bfaf0700018c1a851b4102ae1b9e3142038f29a31543048d20861144058940ad16cd1700000000";
        let v: Vec<u8> = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect();
        let mut mt = MinuteTime::new(0, "000001");
        mt.parse(v);
        assert!(
            mt.result().is_empty(),
            "新协议响应应返回空数据，实际解析出 {} 个点",
            mt.result().len()
        );
    }

    /// 截断的响应：点数声明 10 但数据只有 1 点 → 返回空。
    #[test]
    fn parse_truncated_returns_empty() {
        let mut v = vec![0x0a, 0x00, 0x00, 0x00];
        v.extend_from_slice(&[0x84, 0x0e, 0x00, 0x64]); // 只有 1 个点
        let mut mt = MinuteTime::new(0, "000001");
        mt.parse(v);
        assert!(mt.result().is_empty());
    }

    /// 价格越界（>100000 元）→ 校验失败返回空。
    #[test]
    fn parse_invalid_price_returns_empty() {
        // price = 20_000_000（即 200000 元）超出合理范围：varint = 80 b4 89 13
        let mut v = vec![0x01, 0x00, 0x00, 0x00];
        v.extend_from_slice(&[0x80, 0xb4, 0x89, 0x13, 0x00, 0xa4, 0x01]);
        let mut mt = MinuteTime::new(0, "000001");
        mt.parse(v);
        assert!(mt.result().is_empty());
    }
}
