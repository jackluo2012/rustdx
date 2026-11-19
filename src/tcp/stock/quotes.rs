use crate::bytes_helper::{u16_from_le_bytes, u32_from_le_bytes};
use crate::tcp::Tdx;

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
        // 批量口径：协议单包上限 80；pytdx/mootdx/tdxrs 保守口径 60——
        // 部分服务器对 >60 截断甚至计入限流。高频轮询场景建议 ≤60。
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
        0x0c, 0x01, // H: 0x010c = 268 (2 bytes)
        0x20, 0x63, 0x00, 0x02, // I: 0x02006320 (4 bytes)
        0x00, 0x00, // H: pkg_len1 (占位符, 2 bytes)
        0x00, 0x00, // H: pkg_len2 (占位符, 2 bytes)
        0x3e, 0x05, 0x05, 0x00, // I: 0x0005053e (4 bytes)
        0x00, 0x00, 0x00, 0x00, // I: 0 (4 bytes)
        0x00, 0x00, // H: 0 (2 bytes)
        0x01, 0x00, // H: stock_count (占位符，默认1, 2 bytes)
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
    /// - 之后每只股票的字段见内部函数 `parse_quote`。
    fn parse(&mut self, v: Vec<u8>) {
        // The legacy infallible entry point rejects the whole invalid batch.
        let _ = self.try_parse(v);
    }

    fn try_parse(&mut self, v: Vec<u8>) -> std::io::Result<()> {
        self.data.clear();
        self.response = v;
        let mut pos = 0;
        let mut cursor = QuoteCursor {
            data: &self.response,
            pos: &mut pos,
        };
        cursor.take(2)?;
        let count = cursor.u16()? as usize;
        if count > self.stocks.len() {
            return Err(invalid_quote("unexpected quote count"));
        }
        let mut parsed = Vec::with_capacity(count);
        let mut remaining = self.stocks.clone();
        for _ in 0..count {
            let market = *self
                .response
                .get(pos)
                .ok_or_else(|| invalid_quote("missing market"))? as u16;
            let quote = parse_quote(&self.response, &mut pos)?;
            let Some(index) = remaining
                .iter()
                .position(|(m, code)| *m == market && *code == quote.code)
            else {
                // 请求里有无效代码（退市、写错）时，服务器回一条全零的占位记录
                // （身份是别的证券）。丢弃它，不能让一个无效代码拖垮整批。
                if quote.is_placeholder() {
                    continue;
                }
                return Err(invalid_quote("unexpected quote identity or multiplicity"));
            };
            remaining.swap_remove(index);
            parsed.push(quote);
        }
        self.data = parsed;
        Ok(())
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
fn invalid_quote(message: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message)
}

struct QuoteCursor<'a, 'b> {
    data: &'a [u8],
    pos: &'b mut usize,
}

impl<'a> QuoteCursor<'a, '_> {
    fn take(&mut self, len: usize) -> std::io::Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(len)
            .ok_or_else(|| invalid_quote("quote offset overflow"))?;
        let bytes = self
            .data
            .get(*self.pos..end)
            .ok_or_else(|| invalid_quote("truncated quote"))?;
        *self.pos = end;
        Ok(bytes)
    }

    fn u16(&mut self) -> std::io::Result<u16> {
        Ok(u16_from_le_bytes(self.take(2)?, 0))
    }

    fn u32(&mut self) -> std::io::Result<u32> {
        Ok(u32_from_le_bytes(self.take(4)?, 0))
    }

    fn price(&mut self) -> std::io::Result<i32> {
        crate::tcp::helper::price_checked(self.data, self.pos)
            .ok_or_else(|| invalid_quote("truncated or overflowing quote integer"))
    }
}

fn parse_quote(data: &[u8], pos: &mut usize) -> std::io::Result<QuoteData> {
    let mut cursor = QuoteCursor { data, pos };
    // market：调用方还会用它校验身份；这里用于判定报价精度
    let market = u16::from(cursor.take(1)?[0]);
    let code_bytes = cursor.take(6)?;
    if !code_bytes.iter().all(u8::is_ascii_digit) {
        return Err(invalid_quote("invalid stock code"));
    }
    let code = std::str::from_utf8(code_bytes)
        .map_err(|_| invalid_quote("invalid UTF-8 stock code"))?
        .to_owned();
    cursor.u16()?; // active1

    // 价格与差值（变长编码；price 为绝对价×100，其余字段为相对 price 的差值）
    let price_raw = cursor.price()?;
    let last_close_diff = cursor.price()?;
    let open_diff = cursor.price()?;
    let high_diff = cursor.price()?;
    let low_diff = cursor.price()?;

    // reversed_bytes0：行情时间，相对 price 的差值（见 [`servertime_string`]）
    let reversed_bytes0 = cursor.price()?;
    // reversed_bytes1：实测恒为 -price
    let _reversed_bytes1 = cursor.price()?;

    // vol、cur_vol：累计成交量与现量，单位：手（原始值，不做 /100）
    let vol = cursor.price()?;
    let cur_vol = cursor.price()?;

    // amount (4字节 u32，TDX 浮点编码)
    let amount_raw = cursor.u32()?;
    let amount = crate::tcp::helper::vol_amount(amount_raw as i32);

    // s_vol、b_vol：内盘（主动卖）、外盘（主动买），单位手
    let s_vol = cursor.price()?;
    let b_vol = cursor.price()?;

    // reversed_bytes2-3
    let _reversed_bytes2 = cursor.price()?;
    let _reversed_bytes3 = cursor.price()?;

    // 买一到买五、卖一到卖五：价格（相对差值）及其成交量（手）
    let bid1 = cursor.price()?;
    let ask1 = cursor.price()?;
    let bid1_vol = cursor.price()?;
    let ask1_vol = cursor.price()?;

    let bid2 = cursor.price()?;
    let ask2 = cursor.price()?;
    let bid2_vol = cursor.price()?;
    let ask2_vol = cursor.price()?;

    let bid3 = cursor.price()?;
    let ask3 = cursor.price()?;
    let bid3_vol = cursor.price()?;
    let ask3_vol = cursor.price()?;

    let bid4 = cursor.price()?;
    let ask4 = cursor.price()?;
    let bid4_vol = cursor.price()?;
    let ask4_vol = cursor.price()?;

    let bid5 = cursor.price()?;
    let ask5 = cursor.price()?;
    let bid5_vol = cursor.price()?;
    let ask5_vol = cursor.price()?;

    // reversed_bytes4 (u16)
    let _reversed_bytes4 = cursor.u16()?;
    // reversed_bytes5-8
    let _reversed_bytes5 = cursor.price()?;
    let _reversed_bytes6 = cursor.price()?;
    let _reversed_bytes7 = cursor.price()?;
    let _reversed_bytes8 = cursor.price()?;
    // reversed_bytes9（涨速×100，有符号 i16）+ active2（u16）
    let reversed_bytes9 = cursor.u16()? as i16;
    let _active2 = cursor.u16()?;

    // 按 pytdx _cal_price 公式还原真实价格：`(price_raw + diff) / 100`
    // 报价精度随品种不同（基金三位小数、债券更细），见 [`super::price_kind`]
    let div = super::price_kind(market, &code).quote_divisor();
    let base = price_raw as f64;
    let price = base / div;
    let last_close = (base + last_close_diff as f64) / div;
    let open = (base + open_diff as f64) / div;
    let high = (base + high_diff as f64) / div;
    let low = (base + low_diff as f64) / div;

    // 涨跌额与涨跌幅自行计算（协议不直接提供）
    // 现价为 0 表示还没有成交（开盘前、集合竞价未出价、全天停牌），此时没有
    // 涨跌可言：不能拿 0 去减昨收，否则盘前一扫全是 -100%。
    let has_price = price_raw != 0;
    let change = if has_price { price - last_close } else { 0.0 };
    let change_percent = if has_price && last_close > 0.0 {
        change / last_close * 100.0
    } else {
        0.0
    };

    Ok(QuoteData {
        market,
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
        servertime: servertime_string(
            i64::from(reversed_bytes0) + i64::from(price_raw),
            (div / 100.0) as i64,
        ),
        servertime_raw: reversed_bytes0,
        bid1: (base + bid1 as f64) / div,
        ask1: (base + ask1 as f64) / div,
        bid1_vol: bid1_vol as f64,
        ask1_vol: ask1_vol as f64,
        bid2: (base + bid2 as f64) / div,
        ask2: (base + ask2 as f64) / div,
        bid2_vol: bid2_vol as f64,
        ask2_vol: ask2_vol as f64,
        bid3: (base + bid3 as f64) / div,
        ask3: (base + ask3 as f64) / div,
        bid3_vol: bid3_vol as f64,
        ask3_vol: ask3_vol as f64,
        bid4: (base + bid4 as f64) / div,
        ask4: (base + ask4 as f64) / div,
        bid4_vol: bid4_vol as f64,
        ask4_vol: ask4_vol as f64,
        bid5: (base + bid5 as f64) / div,
        ask5: (base + ask5 as f64) / div,
        bid5_vol: bid5_vol as f64,
        ask5_vol: ask5_vol as f64,
    })
}

/// 将行情时间整数解码为字符串 `HH:MM:SS.mmm`。
///
/// - `sum`：**已加回现价**的值。协议字段 reversed_bytes0 与开/高/低/昨收一样，
///   是相对现价的差值，`时间整数 = reversed_bytes0 + price_raw`；
/// - `scale`：该品种报价精度相对股票的倍数（股票 1、基金 10、债券 100）。
///   时间整数与价格用同一把尺子放大，除以它才是 `HHMMSScc`（7 位为 `HMMSScc`），
///   末两位是百分之一秒。
///
/// 该值在服务端是单精度浮点，放大后的低位是舍入噪声（如债券 08:56:18.00 实测
/// 为 `856179968` 而非 `856180000`），所以先四舍五入到百分之一秒。此前把基金
/// 9 位值的末三位当作毫秒，那三位其实就是这种噪声。
///
/// 依据：2026-11-17 实测 40 只股票/ETF/指数/B 股、2026-11-19 开盘前实测可转债，
/// 还原后全部是合法时间；各服务器同一时刻返回值一致。pytdx `_format_time`
/// 没有加回现价，结果与实测不符（价格越高偏差越大）。
///
/// 放大后的舍入噪声可以超过半个百分之一秒：债券的值在 10 亿量级，单精度的
/// 间隔是 128，即 1.28 个百分之一秒。2026-11-19 实测 13:29:00.00 的债券是
/// `1328999936`（真值 `1329000000`），四舍五入得 `13:28:99.99`。所以放大过的
/// 品种在四舍五入结果非法时，再试相邻的百分之一秒。
///
/// 值异常（无法解码出合法时间）时返回空字符串。
fn servertime_string(sum: i64, scale: i64) -> String {
    if sum <= 0 || scale <= 0 {
        return String::new();
    }
    let rounded = (sum + scale / 2) / scale;
    let neighbours: &[i64] = if scale > 1 { &[0, 1, -1] } else { &[0] };
    neighbours
        .iter()
        .find_map(|d| format_time_integer(rounded + d))
        .unwrap_or_default()
}

/// 把时间整数 `HHMMSScc`（7 位为 `HMMSScc`）格式化为 `HH:MM:SS.mmm`；非法返回 `None`。
fn format_time_integer(time: i64) -> Option<String> {
    let s = time.to_string();
    let hour_len = match s.len() {
        7 => 1,
        8 => 2,
        _ => return None,
    };
    let field = |from: usize, to: usize| s[from..to].parse::<u32>().unwrap_or(u32::MAX);
    let (hour, minute) = (field(0, hour_len), field(hour_len, hour_len + 2));
    let (second, centis) = (
        field(hour_len + 2, hour_len + 4),
        field(hour_len + 4, s.len()),
    );
    if hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    Some(format!(
        "{hour:02}:{minute:02}:{second:02}.{:03}",
        centis * 10
    ))
}

/// 集合竞价阶段的虚拟撮合信息，见 [`QuoteData::auction`]。
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub struct AuctionMatch {
    /// 虚拟撮合价（元）：按此刻的委托撮合将得到的开盘价。
    pub price: f64,
    /// 撮合量（手）：按撮合价可成交的数量。
    pub volume: f64,
    /// 买方未匹配量（手）：撮合价上买方多出来的委托。
    pub unmatched_buy: f64,
    /// 卖方未匹配量（手）。与买方未匹配量至多一个非零。
    pub unmatched_sell: f64,
}

/// 指数成分股的涨跌统计，见 [`QuoteData::breadth`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct IndexBreadth {
    /// 上涨家数。
    pub up: u32,
    /// 下跌家数。
    pub down: u32,
    /// 涨停家数（现价等于涨停价，不区分是否封死）。
    pub limit_up: u32,
    /// 跌停家数（现价等于跌停价，不区分是否封死）。
    pub limit_down: u32,
}

impl QuoteData {
    /// 集合竞价阶段的虚拟撮合信息：开盘竞价（09:15~09:25 定价前）与**收盘竞价**
    /// （14:57~15:00 定价前）；其它时候返回 `None`。
    ///
    /// 竞价阶段撮合信息借用五档字段传递（2026-11-19 早盘与尾盘实测沪深主板、
    /// 创业板、科创板、ETF、北交所一致）：买一价 = 卖一价 = 撮合价，
    /// 买一量 = 卖一量 = 撮合量，买二量 / 卖二量 = 买方 / 卖方未匹配量，
    /// 买二价、卖二价为 0。定价后恢复为正常的现价与五档。
    ///
    /// - 开盘竞价时没有现价（`price`、`open`、`vol` 都是 0）；收盘竞价时
    ///   `price` 停在 14:57 前的最后一笔成交价，**不是**收盘价，收盘价以撮合价
    ///   成交后（实测 15:00:02~15:00:04）才出现在 `price` 里；
    /// - 买卖委托还撮合不上时（常见于成交稀疏的股票、竞价刚开始），服务器给的是
    ///   普通的买一、卖一报价或全 0，返回 `None`；
    /// - 沪市可转债没有收盘竞价，14:57 后仍是连续交易。
    pub fn auction(&self) -> Option<AuctionMatch> {
        let matched = self.bid1 > 0.0
            && self.bid1 == self.ask1
            && self.bid1_vol == self.ask1_vol
            && self.bid2 == 0.0
            && self.ask2 == 0.0;
        matched.then_some(AuctionMatch {
            price: self.bid1,
            volume: self.bid1_vol,
            unmatched_buy: self.bid2_vol,
            unmatched_sell: self.ask2_vol,
        })
    }

    /// 指数的涨跌家数与涨跌停家数；不是指数或服务器没给统计时返回 `None`。
    ///
    /// 指数没有盘口，行情里的五档量字段装的是成分股统计（2026-11-19 盘中实测，
    /// 与逐只成分股行情统计的结果一致，差异只来自两次请求之间的变动）：
    /// 买一量 / 卖一量 = 上涨 / 下跌家数，买二量 / 卖二量 = 涨停 / 跌停家数。
    ///
    /// - 宽基指数（上证指数、深证成指、沪深 300、科创 50、北证 50 等）与
    ///   通达信板块指数（880xxx，见 [`Block::index_code`](super::Block)）都适用，
    ///   所以一次行情请求就能拿到几十个题材的涨跌家数与涨停家数；
    /// - 涨停 / 跌停按「现价等于涨跌停价」统计，不区分是否封死；
    /// - 按代码识别指数：沪市 `000`/`88`/`999` 开头、深市 `399` 开头、北交所
    ///   `899` 开头；
    /// - 已停止维护的指数（如 `399005`）四个数全为 0，返回 `None`；
    /// - `880005`（涨跌家数指数）的字段另有含义，返回 `None`。
    pub fn breadth(&self) -> Option<IndexBreadth> {
        let is_index = match self.market {
            0 => self.code.starts_with("399"),
            1 => {
                self.code != "880005"
                    && ["000", "88", "999"]
                        .iter()
                        .any(|p| self.code.starts_with(p))
            }
            2 => self.code.starts_with("899"),
            _ => false,
        };
        let count = |v: f64| (v >= 0.0).then_some(v as u32);
        let breadth = IndexBreadth {
            up: count(self.bid1_vol)?,
            down: count(self.ask1_vol)?,
            limit_up: count(self.bid2_vol)?,
            limit_down: count(self.ask2_vol)?,
        };
        (is_index && breadth.up + breadth.down > 0).then_some(breadth)
    }

    /// 服务器为无效代码返回的全零占位记录。
    fn is_placeholder(&self) -> bool {
        self.price == 0.0 && self.last_close == 0.0 && self.vol == 0.0 && self.servertime_raw == 0
    }
}

/// 股票实时行情数据（完整五档买卖盘，字段语义与 pytdx get_security_quotes 对齐）。
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct QuoteData {
    /// 市场：0=深市、1=沪市、2=北交所。同一个 6 位代码可能在不同市场各有一只证券
    /// （`000001` 既是深市的平安银行，也是沪市的上证指数），必须与代码一起才能
    /// 唯一确定一只证券。
    pub market: u16,
    /// 股票代码（6位）
    pub code: String,
    /// 当前价（元）。**为 0 表示今天还没有成交**（开盘前、集合竞价未定价、全天
    /// 停牌）：此时开/高/低、成交量也是 0，涨跌额与涨跌幅按 0 给出；集合竞价的
    /// 撮合价见 [`QuoteData::auction`]。
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
    /// **内盘**（手）：主动卖出成交的量（s = sell）。
    ///
    /// 2026-11-19 盘中实测 5 只股票：与当日逐笔里卖盘（方向 1）的合计相差不到 5%，
    /// 差额是集合竞价的中性盘，大致对半分给了内外盘；`s_vol + b_vol == vol`。
    /// 此前文档把内外盘写反了。指数的这两个字段是成分股内、外盘的合计。
    pub s_vol: f64,
    /// **外盘**（手）：主动买入成交的量（b = buy）。见 [`QuoteData::s_vol`]。
    pub b_vol: f64,
    /// 涨跌额（元）
    pub change: f64,
    /// 涨跌幅（%）
    pub change_percent: f64,
    /// 涨速（%，协议字段 reversed_bytes9，有符号）
    pub speed: f64,
    /// 行情时间（`HH:MM:SS.mmm`，如 `14:43:47.750`；无法解码时为空）。
    ///
    /// 该证券本次快照的时间，由协议字段加回现价后解码（见
    /// [`QuoteData::servertime_raw`]）。**与 pytdx 的 `servertime` 不同**：
    /// pytdx 没有加回现价，价格越高偏差越大。
    pub servertime: String,
    /// 行情时间的协议原始值（reversed_bytes0）。
    ///
    /// 它是相对现价的差值，且与价格同步放大：`(servertime_raw + 现价整数) ÷ 倍数`
    /// 才是时间整数 `HHMMSScc`（倍数：股票 1、基金 10、债券 100）。
    /// 一般直接用 [`QuoteData::servertime`]。
    pub servertime_raw: i32,
    /// 买一价（元）。
    ///
    /// **指数没有盘口**：指数行情的五档字段装的是成分股统计，价格类字段
    /// （`bid1` 等）不是报价，不要当盘口用。涨跌家数与涨跌停家数用
    /// [`QuoteData::breadth`] 取；其余字段含义未确认。
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
        assert_eq!((q.market, q.code.as_str()), (0, "000001"));
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
        // 行情时间：原始值 11298811 是相对现价的差值，加回现价 1190（分）得
        // 11300001，即 11:30:00.01（抓包时刻 11:29~11:30；pytdx 未加回现价，
        // 对同一值输出 11:29:52.866）
        assert_eq!(q.servertime_raw, 11298811);
        assert_eq!(q.servertime, "11:30:00.010");
    }

    #[test]
    fn truncated_quote_never_panics_or_returns_partial_data() {
        let hex = "0114010000303030303031fe08a61242440a45bb9fe30ae612a9853d0d31160e4e91981e98ed1e00bfaf0700019f12074102a724ab3b4203911295164304a8288a234405a33296457501000000000000fe08";
        let bytes: Vec<u8> = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect();
        for end in 0..bytes.len() {
            let outcome = std::panic::catch_unwind(|| {
                let mut quotes = SecurityQuotes::default();
                quotes.parse(bytes[..end].to_vec());
                assert!(quotes.data.is_empty(), "partial quote must not escape");
            });
            assert!(outcome.is_ok(), "panic at truncation {end}");
        }
    }

    #[test]
    fn same_packets_match_actual_pytdx_parser() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("fixtures/pytdx_quotes.json")).unwrap();
        assert_eq!(
            fixture["commit"],
            "2857fdad08534533610bd2aeca387f760c4baa42"
        );
        for (index, case) in fixture["fixtures"].as_array().unwrap().iter().enumerate() {
            let hex = case["body_hex"].as_str().unwrap();
            let bytes = (0..hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
                .collect();
            let stocks: Vec<_> = case["stocks"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| (s[0].as_u64().unwrap() as u16, s[1].as_str().unwrap()))
                .collect();
            let mut parser = SecurityQuotes::new(stocks);
            let request = parser
                .send()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            assert_eq!(request, case["request_hex"].as_str().unwrap());
            parser.try_parse(bytes).unwrap();
            assert_eq!(parser.data.len(), case["quotes"].as_array().unwrap().len());
            for (quote, expected) in parser.data.iter().zip(case["quotes"].as_array().unwrap()) {
                let actual = serde_json::to_value(quote).unwrap();
                for (field, value) in expected.as_object().unwrap() {
                    // servertime 有意不与 pytdx 对齐（pytdx 未加回现价，
                    // 见 servertime_string），由 test_servertime_string 单独覆盖
                    if field == "servertime" {
                        continue;
                    }
                    if let Some(want) = value.as_f64() {
                        let got = actual[field].as_f64().unwrap();
                        assert!(
                            (got - want).abs() <= want.abs() * 1e-12 + 1e-12,
                            "fixture {index}, {field}: got {got}, want {want}"
                        );
                    } else {
                        assert_eq!(&actual[field], value, "fixture {index}, {field}");
                    }
                }
            }
        }
    }

    #[test]
    fn duplicate_requests_preserve_pytdx_multiplicity() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("fixtures/pytdx_quotes.json")).unwrap();
        let hex = fixture["fixtures"][2]["body_hex"].as_str().unwrap();
        let bytes: Vec<u8> = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect();
        let mut twice = vec![0, 0, 2, 0];
        twice.extend_from_slice(&bytes[4..]);
        twice.extend_from_slice(&bytes[4..]);
        let mut requested_twice = SecurityQuotes::new(vec![(0, "000001"); 2]);
        requested_twice.try_parse(twice.clone()).unwrap();
        assert_eq!(requested_twice.data.len(), 2);
        let mut requested_once = SecurityQuotes::new(vec![(0, "000001"), (1, "600000")]);
        assert!(requested_once.try_parse(twice).is_err());
        assert!(requested_once.data.is_empty());
    }

    #[test]
    fn malformed_quotes_return_errors_without_partial_rows() {
        let mut parser = SecurityQuotes::default();
        for body in [
            vec![],
            vec![0, 0, 1, 0],
            vec![0, 0, 2, 0],
            vec![0, 0, 1, 0, 0, 255, 255, 255, 255, 255, 255, 0, 0],
            vec![
                0, 0, 1, 0, 0, b'0', b'0', b'0', b'0', b'0', b'1', 0, 0, 128, 128, 128, 128, 128, 0,
            ],
        ] {
            assert_eq!(
                parser.try_parse(body).unwrap_err().kind(),
                std::io::ErrorKind::InvalidData
            );
            assert!(parser.data.is_empty());
        }
        let mut seed = 20261117u64;
        for len in 0..256 {
            let mut body = vec![0, 0, 1, 0, 0, b'0', b'0', b'0', b'0', b'0', b'1', 0, 0];
            for _ in 0..len {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                body.push((seed >> 32) as u8);
            }
            // Any byte sequence is safe; successfully parsed unknown tail fields are allowed.
            let _ = parser.try_parse(body);
        }
    }

    /// 按给定字段拼一条行情记录（其余字段为 0）。
    fn quote_record(market: u8, code: &str, price: i32, last_close_diff: i32) -> Vec<u8> {
        fn varint(n: i32) -> Vec<u8> {
            let mut v = n.unsigned_abs();
            let mut out = vec![(v & 63) as u8 | if n < 0 { 64 } else { 0 }];
            v >>= 6;
            while v > 0 {
                *out.last_mut().unwrap() |= 128;
                out.push((v & 127) as u8);
                v >>= 7;
            }
            out
        }
        let mut b = vec![market];
        b.extend_from_slice(code.as_bytes());
        b.extend_from_slice(&[0, 0]); // active1
        for n in [price, last_close_diff, 0, 0, 0, 0, -price, 0, 0] {
            b.extend(varint(n));
        }
        b.extend_from_slice(&[0; 4]); // amount
        b.extend(std::iter::repeat_n(0u8, 4 + 20)); // s_vol/b_vol/rev2/rev3 + 五档
        b.extend_from_slice(&[0; 2]); // rev4
        b.extend_from_slice(&[0; 4]); // rev5~8
        b.extend_from_slice(&[0; 4]); // rev9 + active2
        b
    }

    fn quote_body(records: &[Vec<u8>]) -> Vec<u8> {
        let mut body = vec![0, 0];
        body.extend_from_slice(&(records.len() as u16).to_le_bytes());
        records.iter().for_each(|r| body.extend_from_slice(r));
        body
    }

    /// 同一个代码在不同市场是不同的证券，结果里带市场才能区分。
    #[test]
    fn quote_carries_market() {
        let body = quote_body(&[
            quote_record(0, "000001", 1225, -5),    // 平安银行
            quote_record(1, "000001", 358883, -30), // 上证指数
        ]);
        let mut quotes = SecurityQuotes::new(vec![(0, "000001"), (1, "000001")]);
        quotes.try_parse(body).unwrap();
        let got: Vec<_> = quotes.data.iter().map(|q| (q.market, q.price)).collect();
        assert_eq!(got, [(0, 12.25), (1, 3588.83)]);
    }

    /// 报价精度随品种不同：同样的原始整数，基金是三位小数，债券更细。
    /// 数值取自 2026-11-18 实盘（K 线收盘价为基准）。
    #[test]
    fn price_scale_by_security_kind() {
        let cases = [
            (1u8, "600000", 1074, 10.74),    // 股票
            (1, "510020", 3750, 3.75),       // ETF
            (0, "159001", 99999, 99.999),    // 货币基金
            (1, "900901", 506, 0.506),       // 沪市 B 股
            (0, "200012", 92, 0.92),         // 深市 B 股（两位小数）
            (1, "113042", 1127820, 112.782), // 可转债
            (1, "204001", 12750, 1.275),     // 逆回购
            (1, "000001", 358883, 3588.83),  // 上证指数
        ];
        for (market, code, raw, want) in cases {
            let mut quotes = SecurityQuotes::new(vec![(market as u16, code)]);
            quotes
                .try_parse(quote_body(&[quote_record(market, code, raw, -10)]))
                .unwrap();
            let q = &quotes.data[0];
            assert!((q.price - want).abs() < 1e-9, "{code}: {}", q.price);
            // 差值字段用同一除数：昨收 = (现价 - 10) / 除数
            let step = q.price - q.last_close;
            assert!((step * raw as f64 / q.price - 10.0).abs() < 1e-6, "{code}");
        }
    }

    /// 无效代码的全零占位记录被丢弃，不影响同批其它证券。
    #[test]
    fn placeholder_for_invalid_code_is_skipped() {
        let body = quote_body(&[
            quote_record(1, "600000", 1074, -12),
            quote_record(1, "600839", 0, 0), // 服务器对无效代码 123456 的回应
        ]);
        let mut quotes = SecurityQuotes::new(vec![(1, "600000"), (0, "123456")]);
        quotes.try_parse(body).unwrap();
        assert_eq!(quotes.data.len(), 1);
        assert_eq!(quotes.data[0].code, "600000");

        // 身份不符但有真实数据的记录仍然是错误
        let body = quote_body(&[quote_record(1, "600839", 500, -1)]);
        let mut quotes = SecurityQuotes::new(vec![(1, "600000")]);
        assert!(quotes.try_parse(body).is_err());
        assert!(quotes.data.is_empty());
    }

    /// 指数的涨跌家数取自五档量字段；个股、停止维护的指数、880005 不给。
    #[test]
    fn breadth_only_for_live_indexes() {
        let quote = |market: u16, code: &str, vols: [f64; 4]| QuoteData {
            market,
            code: code.into(),
            bid1_vol: vols[0],
            ask1_vol: vols[1],
            bid2_vol: vols[2],
            ask2_vol: vols[3],
            ..Default::default()
        };
        // 2026-11-19 盘中实测：上证指数、板块指数「昨日涨停」、北证 50
        let expect = |up, down, limit_up, limit_down| {
            Some(IndexBreadth {
                up,
                down,
                limit_up,
                limit_down,
            })
        };
        let sample = [345.0, 1977.0, 13.0, 25.0];
        assert_eq!(
            quote(1, "000001", sample).breadth(),
            expect(345, 1977, 13, 25)
        );
        assert_eq!(
            quote(1, "880864", [25.0, 18.0, 6.0, 3.0]).breadth(),
            expect(25, 18, 6, 3)
        );
        assert_eq!(
            quote(2, "899050", [15.0, 338.0, 0.0, 0.0]).breadth(),
            expect(15, 338, 0, 0)
        );
        assert_eq!(
            quote(0, "399001", sample).breadth(),
            expect(345, 1977, 13, 25)
        );
        // 同代码的深市平安银行是个股：这些字段是真盘口
        assert_eq!(quote(0, "000001", sample).breadth(), None);
        assert_eq!(quote(1, "600519", sample).breadth(), None);
        assert_eq!(quote(0, "399005", [0.0; 4]).breadth(), None);
        assert_eq!(quote(1, "880005", sample).breadth(), None);
    }

    /// 入参为「原始值 + 现价」与报价精度倍数。样本来自实盘（注释为 原始值 + 现价）。
    #[test]
    fn test_servertime_string() {
        // 股票、指数：倍数 1
        assert_eq!(servertime_string(15121501, 1), "15:12:15.010"); // 000001: 15120276 + 1225
        // 600519：不加现价时看似落后约 15 分钟
        assert_eq!(servertime_string(15115401, 1), "15:11:54.010"); // 14969631 + 145770
        assert_eq!(servertime_string(15124500, 1), "15:12:45.000"); // 399001: 14060648 + 1063852
        // 盘中：000001 相隔 3 秒的两次快照（现价 1225），跨整分
        assert_eq!(servertime_string(14434475 + 1225, 1), "14:43:57.000");
        assert_eq!(servertime_string(14438775 + 1225, 1), "14:44:00.000");
        // 10 点前为 7 位
        assert_eq!(servertime_string(9300501, 1), "09:30:05.010");
        // 基金、沪市 B 股：倍数 10
        assert_eq!(servertime_string(151253958 + 4042, 10), "15:12:58.000"); // 510300
        assert_eq!(servertime_string(149999502 + 498, 10), "15:00:00.000"); // 900901
        assert_eq!(servertime_string(85618000, 10), "08:56:18.000"); // 开盘前，现价为 0
        // 单精度舍入噪声：144344952 + 4043 = 144348995 → 14:43:49.00
        assert_eq!(servertime_string(144344952 + 4043, 10), "14:43:49.000");
        // 债券：倍数 100，08:56:18.00 实测为 856179968（开盘前现价为 0）
        assert_eq!(servertime_string(856179968, 100), "08:56:18.000");
        // 债券：13:29:00.00 实测为 1328999936，四舍五入是非法的 13:28:99.99
        assert_eq!(servertime_string(1328999936, 100), "13:29:00.000");
        // 未放大的品种不做相邻修正
        assert_eq!(servertime_string(13289999, 1), "");
        // 异常值：非正数、位数不对、分/秒越界
        assert_eq!(servertime_string(0, 1), "");
        assert_eq!(servertime_string(-1, 1), "");
        assert_eq!(servertime_string(123, 1), "");
        assert_eq!(servertime_string(15126501, 1), ""); // 秒 65
        assert_eq!(servertime_string(15601501, 1), ""); // 分 60
        assert_eq!(servertime_string(24000000, 1), ""); // 时 24
        assert_eq!(servertime_string(151258000, 1), ""); // 9 位却按股票倍数
    }

    /// 集合竞价的撮合信息借用五档字段传递。数值取自 2026-11-19 早盘实盘。
    #[test]
    fn auction_match_from_bid_ask_fields() {
        let q = |price, b1: f64, b1v: f64, a1: f64, a1v: f64, b2v: f64, a2v: f64| QuoteData {
            price,
            bid1: b1,
            bid1_vol: b1v,
            ask1: a1,
            ask1_vol: a1v,
            bid2_vol: b2v,
            ask2_vol: a2v,
            ..Default::default()
        };
        // 600519 09:22：撮合价 1467.99、撮合 567 手、卖方未匹配 11 手
        let a = q(0.0, 1467.99, 567., 1467.99, 567., 0., 11.)
            .auction()
            .unwrap();
        assert_eq!(
            (a.price, a.volume, a.unmatched_buy, a.unmatched_sell),
            (1467.99, 567., 0., 11.)
        );
        // 000001 09:22：买方未匹配 1256 手
        let a = q(0.0, 12.13, 2266., 12.13, 2266., 1256., 0.)
            .auction()
            .unwrap();
        assert_eq!((a.unmatched_buy, a.unmatched_sell), (1256., 0.));
        // 开盘前无委托、09:25 定价后、连续竞价中：都不是竞价撮合状态
        assert!(q(0.0, 0., 0., 0., 0., 0., 0.).auction().is_none());
        assert!(
            q(1442.0, 1441.98, 1., 1442.0, 16., 1., 1.)
                .auction()
                .is_none()
        );
        // 撮合不上时是普通报价（920799 09:22：买一 24.86x36、卖一 25.00x0）
        assert!(q(0.0, 24.86, 36., 25.0, 0., 0., 0.).auction().is_none());
        // 收盘竞价：现价停在 14:57 前的最后一笔，撮合信息同样在五档里
        // 600519 14:59:58：现价 1460.21、撮合价 1466.16、撮合 1634 手、卖方未匹配 3 手
        let a = q(1460.21, 1466.16, 1634., 1466.16, 1634., 0., 3.)
            .auction()
            .unwrap();
        assert_eq!((a.price, a.volume, a.unmatched_sell), (1466.16, 1634., 3.));
        // 北交所 920799 14:57:52：撮合价 23.85、撮合 37 手、买方未匹配 32 手
        let a = q(23.88, 23.85, 37., 23.85, 37., 32., 0.).auction().unwrap();
        assert_eq!((a.price, a.unmatched_buy), (23.85, 32.));
        // 指数的买一价、卖一价可能相等，但买二价不为 0，不是竞价
        let index = QuoteData {
            bid2: 2304.6,
            ask2: 2304.6,
            ..q(2304.6, 2304.6, 154., 2304.6, 154., 3., 0.)
        };
        assert!(index.auction().is_none());
    }

    /// 开盘前现价为 0：涨跌额与涨跌幅是 0，不是 -100%。
    #[test]
    fn no_change_before_first_trade() {
        let mut quotes = SecurityQuotes::new(vec![(1, "600519")]);
        quotes
            .try_parse(quote_body(&[quote_record(1, "600519", 0, 144266)]))
            .unwrap();
        let q = &quotes.data[0];
        assert_eq!((q.price, q.last_close), (0.0, 1442.66));
        assert_eq!((q.change, q.change_percent), (0.0, 0.0));
    }
}
