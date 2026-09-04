use crate::bytes_helper::{u16_from_le_bytes, u32_from_le_bytes};
use crate::tcp::{helper::price, Tdx};

/// 获取股票实时行情快照。对应于 pytdx 中的 hq.get_security_quotes、GetSecurityQuotesCmd。
///
/// ## 注意
/// - 可以一次获取多只股票的实时行情信息（建议不超过80只）
/// - 返回字段：当前价、昨收、开高低、成交量（手）、成交额、买卖五档等
/// - 协议中价格字段为「绝对价 + 差值」编码，本模块已按 pytdx `_cal_price` 公式还原真实价格
///
/// ## 示例
/// ```ignore
/// use rustdx_complete::tcp::{Tcp, Tdx};
/// use rustdx_complete::tcp::stock::SecurityQuotes;
///
/// let mut tcp = Tcp::new()?;
/// let mut quotes = SecurityQuotes::new(vec![(0, "000001"), (1, "600000")]);
/// let data = quotes.recv_parsed(&mut tcp)?;
/// for quote in data {
///     println!("{}: {} ({:+.2}%)", quote.code, quote.price, quote.change_percent);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct SecurityQuotes<'d> {
    pub send: Box<[u8]>,
    pub stocks: Vec<(u16, &'d str)>,
    pub response: Vec<u8>,
    pub data: Vec<QuoteData>,
}

impl<'d> Default for SecurityQuotes<'d> {
    fn default() -> Self {
        Self::new(vec![(0, "000001")])
    }
}

impl<'d> SecurityQuotes<'d> {
    /// 创建一个新的股票行情请求。
    ///
    /// ## 参数
    /// - `stocks`: 股票列表，格式为 `[(market, code), ...]`
    ///   - market: 0=深市, 1=沪市
    ///   - code: 6位股票代码
    ///
    /// ## panic
    /// 当任何股票代码的长度不是6时，程序会panic。
    pub fn new(stocks: Vec<(u16, &'d str)>) -> Self {
        let count = stocks.len();
        assert!(count > 0 && count <= 80, "股票数量必须在1-80之间");
        for (_, code) in &stocks {
            assert_eq!(code.len(), 6, "股票代码必须是6位");
        }

        // 计算包长度: stock_count * 7 + 12（注意：这里是整个包的数据长度）
        let pkg_len = (count * 7 + 12) as u16;

        let mut send = [0u8; Self::LEN];
        // 复制整个包头（22字节）
        send[0..22].copy_from_slice(Self::SEND);

        // 设置包长度（字节6-7，第一个pkg_len，u16）
        send[6..8].copy_from_slice(&pkg_len.to_le_bytes());
        // 设置包长度重复（字节8-9，第二个pkg_len，u16）
        send[8..10].copy_from_slice(&pkg_len.to_le_bytes());

        // 设置股票数量（字节20-21）
        send[20..22].copy_from_slice(&(count as u16).to_le_bytes());

        // 填充每只股票的信息（每只7字节: 1字节market + 6字节code）
        let mut pos = 22;
        for (market, code) in &stocks {
            send[pos] = *market as u8;
            send[pos + 1..pos + 7].copy_from_slice(code.as_bytes());
            pos += 7;
        }

        Self {
            send: send.into(),
            stocks,
            response: Vec::new(),
            data: Vec::with_capacity(count),
        }
    }
}

impl<'a> Tdx for SecurityQuotes<'a> {
    type Item = [QuoteData];

    /// 获取股票行情的请求字节。
    ///
    /// ## 协议格式（与 pytdx GetSecurityQuotesCmd.setParams 一致）
    /// - 前22字节：固定包头（struct.pack("<HIHHIIHH", ...)）
    ///   - 0-1: H (0x010c = 268)
    ///   - 2-5: I (0x02006320)
    ///   - 6-7: H (pkg_len1)
    ///   - 8-9: H (pkg_len2)
    ///   - 10-13: I (0x5053e)
    ///   - 14-17: I (0)
    ///   - 18-19: H (0)
    ///   - 20-21: H (stock_count)
    /// - 之后每7字节：一只股票 (1字节market + 6字节code)
    const SEND: &'static [u8] = &[
        0x0c, 0x01,                   // H: 0x010c = 268 (2 bytes)
        0x20, 0x63, 0x00, 0x02,       // I: 0x02006320 (4 bytes)
        0x00, 0x00,                   // H: pkg_len1 (占位符, 2 bytes)
        0x00, 0x00,                   // H: pkg_len2 (占位符, 2 bytes)
        0x3e, 0x05, 0x05, 0x00,       // I: 0x0005053e (4 bytes)
        0x00, 0x00, 0x00, 0x00,       // I: 0 (4 bytes)
        0x00, 0x00,                   // H: 0 (2 bytes)
        0x01, 0x00,                   // H: stock_count (占位符，默认1, 2 bytes)
    ];

    const TAG: &'static str = "股票行情快照";
    const LEN: usize = 22 + 80 * 7; // 固定长度：包头22字节 + 最多80只股票

    fn send(&mut self) -> &[u8] {
        // 只返回实际需要发送的字节数：包头22字节 + 每只股票7字节
        let actual_len = 22 + self.stocks.len() * 7;
        &self.send[..actual_len]
    }

    /// 解析响应的字节。
    ///
    /// ## 响应格式（与 pytdx GetSecurityQuotesCmd.parseResponse 一致）
    /// - 前2字节：跳过
    /// - 接下来2字节：股票数量（小端 u16）
    /// - 之后每只股票的字段见 [`parse_quote`]。
    fn parse(&mut self, v: Vec<u8>) {
        // 检查最小长度：至少需要 4 字节（2跳过 + 2数量）
        if v.len() < 4 {
            self.response = v;
            self.data = Vec::new();
            return;
        }

        let mut pos = 0;

        // 跳过前2字节
        pos += 2;

        // 读取股票数量
        let num_stocks = u16_from_le_bytes(&v, pos);
        pos += 2;

        self.data = Vec::with_capacity(num_stocks as usize);

        for i in 0..num_stocks {
            // 剩余数据不足以包含一只股票的最小头部（market+code+active1 = 9 字节）时停止
            if pos + 9 > v.len() {
                debug_assert!(false, "行情数据不完整，只解析了 {i}/{num_stocks} 只股票");
                break;
            }
            // 解析每只股票数据
            let quote = parse_quote(&v, &mut pos);
            self.data.push(quote);
        }

        self.response = v;
    }

    fn result(&self) -> &Self::Item {
        &self.data
    }
}

/// 解析单只股票的行情数据。
///
/// 字段顺序与 pytdx `GetSecurityQuotesCmd.parseResponse` 完全一致：
/// price, last_close_diff, open_diff, high_diff, low_diff, rev0(服务器时间), rev1,
/// vol, cur_vol, amount(u32), s_vol, b_vol, rev2, rev3,
/// bid1..bid5/ask1..ask5 及其成交量（均为「相对当前价的差值」），
/// rev4(u16), rev5..rev8, rev9(i16, 涨速×100) + active2(u16)。
///
/// 价格还原公式（pytdx `_cal_price`）：`真实价格 = (price_raw + diff) / 100`。
fn parse_quote(data: &[u8], pos: &mut usize) -> QuoteData {
    // market (1字节) + code (6字节) + active1 (2字节)
    let _market = data[*pos] as u16;
    *pos += 1;
    let code_bytes = &data[*pos..*pos + 6];
    *pos += 6;
    let code = unsafe { std::str::from_utf8_unchecked(code_bytes) };
    let code = String::from(code); // 转换为拥有所有权的String
    let _active1 = u16_from_le_bytes(data, *pos);
    *pos += 2;

    // 价格与差值（变长编码；price 为绝对价×100，其余字段为相对 price 的差值）
    let price_raw = price(data, pos);
    let last_close_diff = price(data, pos);
    let open_diff = price(data, pos);
    let high_diff = price(data, pos);
    let low_diff = price(data, pos);

    // reversed_bytes0：服务器时间，编码为 HHMM+SSMM 的十进制拼接
    // （如 11298811 = 11:29:52.866，见 [`servertime_string`]）
    let reversed_bytes0 = price(data, pos);
    // reversed_bytes1：实测恒为 -price
    let _reversed_bytes1 = price(data, pos);

    // vol、cur_vol：累计成交量与现量，单位：手（原始值，不做 /100）
    let vol = price(data, pos);
    let cur_vol = price(data, pos);

    // amount (4字节 u32，TDX 浮点编码)
    let amount_raw = u32_from_le_bytes(data, *pos);
    *pos += 4;
    let amount = crate::tcp::helper::vol_amount(amount_raw as i32);

    // s_vol、b_vol：外盘、内盘（手）
    let s_vol = price(data, pos);
    let b_vol = price(data, pos);

    // reversed_bytes2-3
    let _reversed_bytes2 = price(data, pos);
    let _reversed_bytes3 = price(data, pos);

    // 买一到买五、卖一到卖五：价格（相对差值）及其成交量（手）
    let bid1 = price(data, pos);
    let ask1 = price(data, pos);
    let bid1_vol = price(data, pos);
    let ask1_vol = price(data, pos);

    let bid2 = price(data, pos);
    let ask2 = price(data, pos);
    let bid2_vol = price(data, pos);
    let ask2_vol = price(data, pos);

    let bid3 = price(data, pos);
    let ask3 = price(data, pos);
    let bid3_vol = price(data, pos);
    let ask3_vol = price(data, pos);

    let bid4 = price(data, pos);
    let ask4 = price(data, pos);
    let bid4_vol = price(data, pos);
    let ask4_vol = price(data, pos);

    let bid5 = price(data, pos);
    let ask5 = price(data, pos);
    let bid5_vol = price(data, pos);
    let ask5_vol = price(data, pos);

    // reversed_bytes4 (u16)
    let _reversed_bytes4 = u16_from_le_bytes(data, *pos);
    *pos += 2;
    // reversed_bytes5-8
    let _reversed_bytes5 = price(data, pos);
    let _reversed_bytes6 = price(data, pos);
    let _reversed_bytes7 = price(data, pos);
    let _reversed_bytes8 = price(data, pos);
    // reversed_bytes9（涨速×100，有符号 i16）+ active2（u16）
    let reversed_bytes9 = u16_from_le_bytes(data, *pos) as i16;
    let _active2 = u16_from_le_bytes(data, *pos + 2);
    *pos += 4;

    // 按 pytdx _cal_price 公式还原真实价格：`(price_raw + diff) / 100`
    let base = price_raw as f64;
    let price = base / 100.0;
    let last_close = (base + last_close_diff as f64) / 100.0;
    let open = (base + open_diff as f64) / 100.0;
    let high = (base + high_diff as f64) / 100.0;
    let low = (base + low_diff as f64) / 100.0;

    // 涨跌额与涨跌幅自行计算（协议不直接提供）
    let change = price - last_close;
    let change_percent = if last_close > 0.0 {
        change / last_close * 100.0
    } else {
        0.0
    };

    QuoteData {
        code,
        price,
        last_close,
        open,
        high,
        low,
        vol: vol as f64,
        cur_vol: cur_vol as f64,
        amount,
        s_vol: s_vol as f64,
        b_vol: b_vol as f64,
        change,
        change_percent,
        speed: reversed_bytes9 as f64 / 100.0,
        servertime: servertime_string(reversed_bytes0),
        bid1: (base + bid1 as f64) / 100.0,
        ask1: (base + ask1 as f64) / 100.0,
        bid1_vol: bid1_vol as f64,
        ask1_vol: ask1_vol as f64,
        bid2: (base + bid2 as f64) / 100.0,
        ask2: (base + ask2 as f64) / 100.0,
        bid2_vol: bid2_vol as f64,
        ask2_vol: ask2_vol as f64,
        bid3: (base + bid3 as f64) / 100.0,
        ask3: (base + ask3 as f64) / 100.0,
        bid3_vol: bid3_vol as f64,
        ask3_vol: ask3_vol as f64,
        bid4: (base + bid4 as f64) / 100.0,
        ask4: (base + ask4 as f64) / 100.0,
        bid4_vol: bid4_vol as f64,
        ask4_vol: ask4_vol as f64,
        bid5: (base + bid5 as f64) / 100.0,
        ask5: (base + ask5 as f64) / 100.0,
        bid5_vol: bid5_vol as f64,
        ask5_vol: ask5_vol as f64,
    }
}

/// 将 reversed_bytes0 解码为服务器时间字符串。
///
/// 编码为十进制位的拼接：`HH` + `MM` + 秒的千分位展开（见 pytdx
/// `GetSecurityQuotesCmd._format_time`，来源 <https://github.com/rainx/pytdx/issues/187>）。
/// 例如 `11298811`：时=11、分=29、`8811 * 60 / 10000 = 52.866` 秒 → `11:29:52.866`。
///
/// 值异常（无法解码出合理时间）时返回空字符串。
fn servertime_string(v: i32) -> String {
    if v <= 0 {
        return String::new();
    }
    let s = v.to_string();
    if s.len() < 7 {
        return String::new();
    }
    let (head, tail) = s.split_at(s.len() - 6);
    let hour: u32 = head.parse().unwrap_or(u32::MAX);
    let minute: u32 = tail[..2].parse().unwrap_or(u32::MAX);
    let sec_raw: f64 = tail[2..].parse().unwrap_or(f64::NAN);
    // pytdx 公式：秒 = 原始值 * 60 / 10000（如 8811 → 52.866 秒）
    let seconds = sec_raw * 60.0 / 10000.0;
    if hour > 23 || minute > 59 || !(0.0..60.0).contains(&seconds) {
        return String::new();
    }
    format!("{hour:02}:{minute:02}:{seconds:06.3}")
}

/// 股票实时行情数据（完整五档买卖盘，字段语义与 pytdx get_security_quotes 对齐）。
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct QuoteData {
    /// 股票代码（6位）
    pub code: String,
    /// 当前价（元）
    pub price: f64,
    /// 昨收价（元）。协议中为相对当前价的差值，已还原。
    pub last_close: f64,
    /// 开盘价（元）。协议中为相对当前价的差值，已还原。
    pub open: f64,
    /// 最高价（元）。协议中为相对当前价的差值，已还原。
    pub high: f64,
    /// 最低价（元）。协议中为相对当前价的差值，已还原。
    pub low: f64,
    /// 累计成交量（手）
    pub vol: f64,
    /// 现量（手）
    pub cur_vol: f64,
    /// 成交额（元）
    pub amount: f64,
    /// 外盘（手）
    pub s_vol: f64,
    /// 内盘（手）
    pub b_vol: f64,
    /// 涨跌额（元）
    pub change: f64,
    /// 涨跌幅（%）
    pub change_percent: f64,
    /// 涨速（%，协议字段 reversed_bytes9，有符号）
    pub speed: f64,
    /// 服务器时间（如 `11:29:52.866`；无法解码时为空）
    pub servertime: String,
    /// 买一价（元）
    pub bid1: f64,
    /// 卖一价（元）
    pub ask1: f64,
    /// 买一量（手）
    pub bid1_vol: f64,
    /// 卖一量（手）
    pub ask1_vol: f64,
    /// 买二价（元）
    pub bid2: f64,
    /// 卖二价（元）
    pub ask2: f64,
    /// 买二量（手）
    pub bid2_vol: f64,
    /// 卖二量（手）
    pub ask2_vol: f64,
    /// 买三价（元）
    pub bid3: f64,
    /// 卖三价（元）
    pub ask3: f64,
    /// 买三量（手）
    pub bid3_vol: f64,
    /// 卖三量（手）
    pub ask3_vol: f64,
    /// 买四价（元）
    pub bid4: f64,
    /// 卖四价（元）
    pub ask4: f64,
    /// 买四量（手）
    pub bid4_vol: f64,
    /// 卖四量（手）
    pub ask4_vol: f64,
    /// 买五价（元）
    pub bid5: f64,
    /// 卖五价（元）
    pub ask5: f64,
    /// 买五量（手）
    pub bid5_vol: f64,
    /// 卖五量（手）
    pub ask5_vol: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_quotes_default() {
        let quotes = SecurityQuotes::default();
        assert_eq!(quotes.stocks.len(), 1);
        assert_eq!(quotes.stocks[0].0, 0);
        assert_eq!(quotes.stocks[0].1, "000001");
    }

    #[test]
    fn test_security_quotes_new() {
        let stocks = vec![(0, "000001"), (1, "600000")];
        let quotes = SecurityQuotes::new(stocks);
        assert_eq!(quotes.stocks.len(), 2);
    }

    #[test]
    #[should_panic(expected = "股票数量必须在1-80之间")]
    fn test_security_quotes_empty() {
        SecurityQuotes::new(vec![]);
    }

    /// 完整真实响应的解析测试：2026-09-04 11:29 抓取（body hex 来自 pytdx 抓包，
    /// 同时段 pytdx get_security_quotes 输出见各断言注释）。
    #[test]
    fn parse_full_response() {
        let hex = "0114010000303030303031fe08a61242440a45bb9fe30ae612a9853d0d31160e4e91981e98ed1e00bfaf0700019f12074102a724ab3b4203911295164304a8288a234405a33296457501000000000000fe08";
        let v: Vec<u8> = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect();

        let mut quotes = SecurityQuotes::default();
        quotes.parse(v);

        assert_eq!(quotes.data.len(), 1);
        let q = &quotes.data[0];
        assert_eq!(q.code, "000001");
        // 与同时段 pytdx get_security_quotes 输出逐字段一致
        assert_eq!(q.price, 11.9);
        assert_eq!(q.last_close, 11.88);
        assert_eq!(q.open, 11.86);
        assert_eq!(q.high, 12.0);
        assert_eq!(q.low, 11.85);
        assert_eq!(q.vol, 500073.0); // 手
        assert_eq!(q.cur_vol, 13.0);
        assert_eq!(q.s_vol, 247313.0);
        assert_eq!(q.b_vol, 252760.0);
        assert_eq!(q.amount, 595954752.0);
        // 五档：bid1=+0, ask1=+1, bid5=-4, ask5=+5（差值 + 绝对价还原）
        assert_eq!(q.bid1, 11.9);
        assert_eq!(q.ask1, 11.91);
        assert_eq!(q.bid2, 11.89);
        assert_eq!(q.ask2, 11.92);
        assert_eq!(q.bid3, 11.88);
        assert_eq!(q.ask3, 11.93);
        assert_eq!(q.bid4, 11.87);
        assert_eq!(q.ask4, 11.94);
        assert_eq!(q.bid5, 11.86);
        assert_eq!(q.ask5, 11.95);
        assert_eq!(q.bid1_vol, 1183.0);
        assert_eq!(q.ask1_vol, 7.0);
        // 涨跌：change = 11.90 - 11.88 = 0.02
        assert!((q.change - 0.02).abs() < 1e-9);
        assert!((q.change_percent - 0.1684).abs() < 1e-4);
        // 服务器时间
        assert_eq!(q.servertime, "11:29:52.866");
    }

    #[test]
    fn test_servertime_string() {
        assert_eq!(servertime_string(11298811), "11:29:52.866");
        assert_eq!(servertime_string(9561810), "09:56:10.860"); // 1810*60/10000 = 10.86
        assert_eq!(servertime_string(0), "");
        assert_eq!(servertime_string(-1), "");
        assert_eq!(servertime_string(123), ""); // 过短
    }
}
