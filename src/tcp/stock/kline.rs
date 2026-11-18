use crate::tcp::{Tdx, helper::DateTime};

// ['获取股票行情', '参数：市场代码， 股票代码， 如： 0,000001 或 1,  600300',
// get_security_quotes, '0,000001']),          (2, ['获取k线', '''category-> K线种类  0
// 5分钟K线 1 15分钟K线 2 30分钟K线 3 1小时K线 4 日K线  5 周K线   6 月K线
//   7 1分钟
//   8 1分钟K线 9 日K线
//   10 季K线
//   11 年K线
//   market -> 市场代码 0:深圳，1:上海
//   stockcode -> 证券代码;
//   start -> 指定的范围开始位置;
//   count -> 用户要请求的 K 线数目，最大值为 800
//
//   如： 9,0,000001,0,100''', get_security_bars, '9,0,000001,0,100']),
/// 查询股票日线。对应于 pytdx 中的 hq.get_security_bars、GetSecurityBarsCmd。
/// ## 注意
/// 只修改字段并不会更改待发送字节的内容。
/// 如果你需要修改查询条件，请使用相应的方法。
/// 比如修改股票代码调用 [`Kline::code`]，修改查询数量调用 [`Kline::count`]。
#[derive(Debug, Clone)]
pub struct Kline<'d> {
    pub send: Box<[u8]>,
    pub market: u16,
    pub code: &'d str,
    pub category: u16,
    pub start: u16,
    pub count: u16,
    pub response: Vec<u8>,
    pub data: Vec<KlineData<'d>>,
}

/// 为了对应 [`Kline::SEND`] 的含义，以下默认值值得注意：
/// 1. category 默认为 9 （日线）；
/// 2. code 默认为 `000001`；
/// 3. count 默认为 3；
/// 4. KlineData.dt.hour 默认小时数为 15。
#[rustfmt::skip]
impl<'d> Default for Kline<'d> {
    fn default() -> Self {
        Self { market: 0, code: "000001", category: 9, start: 0, count: 3,
               send:     { let mut v = [0; Self::LEN]; v.copy_from_slice(Self::SEND); v.into() },
               response: Vec::new(),
               data:     vec![KlineData::default(); 3], }
    }
}

impl<'d> Kline<'d> {
    /// 0 代表深市；1 代表沪市。
    ///
    /// ## panic
    /// 当 code 的字节长度不是 6 时，程序会 panic。
    #[rustfmt::skip]
    pub fn new(market: u16, code: &'d str, category: u16, start: u16, count: u16) -> Self {
        Self { market, code, category, start, count,
               send: {
                   let mut arr = [0; Self::LEN];
                   arr.copy_from_slice(Self::SEND);
                   arr[12..14].copy_from_slice(&market.to_le_bytes());
                   arr[14..20].copy_from_slice(code.as_bytes());
                   arr[20..22].copy_from_slice(&category.to_le_bytes());
                   arr[24..26].copy_from_slice(&start.to_le_bytes());
                   arr[26..28].copy_from_slice(&count.to_le_bytes());
                   arr.into()
               },
               response: Vec::new(),
               data: vec![KlineData::default(); count as usize] }
    }

    /// 修改市场。
    pub fn market(&mut self, market: u16) -> &mut Self {
        self.market = market;
        self.send[12..14].copy_from_slice(&market.to_le_bytes());
        self
    }

    /// 修改股票。当代码不正确时，不能正常得到响应。
    ///
    /// ## panic
    /// 当 code 的字节长度不是 6 时，程序会 panic。
    pub fn code(&mut self, code: &'d str) -> &mut Self {
        self.code = code;
        self.send[14..20].copy_from_slice(code.as_bytes());
        self
    }

    /// 修改 K 线类型。
    pub fn category(&mut self, category: u16) -> &mut Self {
        self.category = category;
        self.send[20..22].copy_from_slice(&category.to_le_bytes());
        self
    }

    /// 修改起始位置。
    pub fn start(&mut self, start: u16) -> &mut Self {
        self.start = start;
        self.send[24..26].copy_from_slice(&start.to_le_bytes());
        self
    }

    /// 修改查询数量。
    ///
    /// 注意：此方法调用了 [`Vec::resize_with`]
    /// 1. 如果此次修改的查询数量大于修改前的查询数量，
    ///    则 data 字段除了保持已有的数据，还会会增加默认值。
    /// 2. 如果此次修改的查询数量小于或等于修改前的查询数量，
    ///    则 data 字段截断已有的数据。
    /// 3. 无论以上哪种情况，目的都是保证查询数量与 data 字段的 Vec 长度一致。
    pub fn count(&mut self, count: u16) -> &mut Self {
        self.data.resize_with(count as usize, Default::default);
        self.count = count;
        self.send[26..28].copy_from_slice(&count.to_le_bytes());
        self
    }
}

impl<'a> Tdx for Kline<'a> {
    type Item = [KlineData<'a>];

    /// #sz000001# 最近三天日线的请求字节。长度为 38。
    /// ```python
    /// struct.pack("<HIHHHH6sHHHHIIH", bytes) # python 中的解读方式
    /// ```
    const SEND: &'static [u8] = &[
        0x0c, 0x01, 0x08, 0x64, 0x01, 0x01, 0x1c, 0x00, 0x1c, 0x00, 0x2d, 0x05, 0x00, 0x00, 0x30,
        0x30, 0x30, 0x30, 0x30, 0x31, 0x09, 0x00, 0x01, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    const TAG: &'static str = "日线";

    fn send(&mut self) -> &[u8] {
        &self.send
    }

    /// 旧入口（不可失败）：等价于忽略 [`Tdx::try_parse`] 的错误。
    ///
    /// 畸形/截断的响应不会 panic，也不会留下部分数据：此时 `data` 为空、
    /// `response` 仍保存原始字节。需要区分「真的没有数据」与「响应异常」时，
    /// 请使用 [`Tdx::try_parse`] 或 [`Tdx::recv_parsed`]。
    fn parse(&mut self, v: Vec<u8>) {
        let _ = self.try_parse(v);
    }

    /// 严格解析：畸形响应返回 [`std::io::ErrorKind::InvalidData`]，
    /// 由连接层按配置重连/换服务器重试。
    ///
    /// 指数代码也可以走本命令：服务器按指数格式回应时会自动识别（结果不含
    /// 涨跌家数，需要时用 [`IndexKline`](super::IndexKline)）。
    ///
    /// - 响应体不足 2 字节（连记录数都没有）→ 错误；
    /// - 实际解析出的记录数 < min(响应头声明数, 请求数)（记录被截断，或声明
    ///   有数据却没有数据体，实盘出现过只有 `20 03` 两字节的响应）→ 错误；
    /// - 声明数为 0（停牌/无历史/翻页到尽头，体为 `00 00`）→ 成功且结果为空。
    ///
    /// 出错时 `data` 被清空（不返回部分数据），`response` 始终保存原始字节。
    #[rustfmt::skip]
    fn try_parse(&mut self, v: Vec<u8>) -> std::io::Result<()> {
        use crate::tcp::helper::vol_amount;

        self.data.clear();
        self.response = v;
        let v = &self.response;
        let declared = declared_count(v, Self::TAG)?;
        // 服务器可能返回少于请求数（历史尽头 / 长期停牌等），以声明数为准；
        // 声明数多于请求数时只取请求的根数。
        let expected = declared.min(self.count as usize);

        // 指数代码走本命令时，服务器按指数格式回应：每条记录多 4 字节（涨跌家数）。
        // 按股票格式读会从第二条起错位，得到看似合理的错误数字。判别依据：
        // 正确的格式恰好读完整个响应（实测股票/基金/债券/指数无一例外）。
        let (stock, stock_end) = read_bars(v, self.category, declared, 0);
        let exact = |bars: &[BarHead], end: usize| bars.len() == declared && end == v.len();
        let (mut bars, index_layout) = if exact(&stock, stock_end) {
            (stock, false)
        } else {
            let (index, index_end) = read_bars(v, self.category, declared, 4);
            if exact(&index, index_end) { (index, true) } else { (stock, false) }
        };
        bars.truncate(expected);
        let kind = super::price_kind(self.market, self.code);

        let mut base = 0i64;
        let mut data = Vec::with_capacity(expected);
        for bar in bars {
            let [open, close, high, low] = bar.prices;
            base += open;
            data.push(KlineData { dt: bar.dt, code: self.code,
                                  open:   base as f64 / 1000.,
                                  close:  real_price(close, base),
                                  high:   real_price(high, base),
                                  low:    real_price(low, base),
                                  vol:    vol_in_hands(vol_amount(bar.vol as i32), self.category, index_layout, kind),
                                  amount: vol_amount(bar.amount as i32) });
            base += close;
        }
        if data.len() < expected {
            return Err(incomplete(Self::TAG, declared, self.count, data.len()));
        }
        // 升序契约：实测服务端返回顺序与早前相反（2026-09 起，疑与当日分时
        // 协议变更同期调整），解析层统一排序，消费方不再各自兜底。
        data.sort_by_key(|b| b.dt);
        self.data = data;
        Ok(())
    }

    fn result(&self) -> &Self::Item {
        &self.data
    }
}

#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct KlineData<'d> {
    pub dt: DateTime,
    pub code: &'d str,
    pub open: f64,
    pub close: f64,
    pub high: f64,
    pub low: f64,
    /// 成交量，单位：手，与行情快照的累计成交量口径一致。协议里的原始单位随
    /// 周期和品种变化（分钟线是股、指数与债券的日线是百手），解析时已统一换算。
    /// 例外：**指数的分钟线没有成交量**，此字段是成交额的百分之一。
    /// 日线不含盘后成交。
    pub vol: f64,
    /// 成交额，单位：元
    pub amount: f64,
}

// impl<'d> KlineData<'d> {
//     pub fn parse(v: &'d [u8], mut pos: usize, mut base: i32, code: &'d str, category: u16)
//                  -> (usize, i32, KlineData<'d>) {
//         use crate::{
//             bytes_helper::u32_from_le_bytes,
//             tcp::helper::{datetime, price, vol_amount},
//         };
//         let dt = datetime(&v[pos..pos + 4], category);
//         pos += 4;
//         let open = price(&v, &mut pos);
//         let close = price(&v, &mut pos);
//
//         #[rustfmt::skip]
//         let kline = Self { dt, code,
//                            open: { base += open; base as f64 / 1000.  },
//                            close: real_price(close, base),
//                            high: real_price(price(&v, &mut pos), base),
//                            low: real_price(price(&v, &mut pos), base),
//                            vol: { pos += 4; vol_amount(u32_from_le_bytes(&v, pos - 4) as
// i32) },                            amount: { pos += 4; vol_amount(u32_from_le_bytes(&v, pos
// - 4) as i32) } };
//
//         base += close;
//         (pos, base, kline)
//     }
// }

/// 价格以 i64 累加：畸形数据也不会让差值累加溢出（合法数据下结果与 i32 相同）。
#[inline]
fn real_price(p: i64, base: i64) -> f64 {
    (p + base) as f64 / 1000.
}

/// 股票 K 线与指数 K 线每条记录的公共前缀：
/// datetime(4) + 开/收/高/低(varint 差值) + vol(u32) + amount(u32)。
pub(super) struct BarHead {
    pub dt: DateTime,
    /// 开/收/高/低 的原始差值（尚未累加、未除以 1000）。
    pub prices: [i64; 4],
    pub vol: u32,
    pub amount: u32,
}

/// 读取响应头的记录数（前 2 字节）；不足 2 字节视为畸形响应。
pub(super) fn declared_count(v: &[u8], tag: &str) -> std::io::Result<usize> {
    match v {
        [lo, hi, ..] => Ok(u16::from_le_bytes([*lo, *hi]) as usize),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("{tag}响应体过短：{} 字节，缺少记录数", v.len()),
        )),
    }
}

/// 记录数不足（截断，或声明有数据却无数据体）时的错误。
pub(super) fn incomplete(
    tag: &str,
    declared: usize,
    requested: u16,
    parsed: usize,
) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!("{tag}响应不完整：声明 {declared} 根，请求 {requested} 根，实际解析 {parsed} 根"),
    )
}

/// 越界安全地读取 4 字节小端 u32；数据不足返回 None。
pub(super) fn read_u32(v: &[u8], pos: &mut usize) -> Option<u32> {
    let bytes = v.get(*pos..pos.checked_add(4)?)?;
    *pos += 4;
    Some(u32::from_le_bytes(bytes.try_into().ok()?))
}

/// 把协议里的原始成交量统一换算为手（与行情快照、分时、逐笔一致）。
///
/// 原始单位随周期与品种变化（2026-11-18 盘中实测：各周期当日成交量之和
/// 对比行情快照的累计成交量）：
///
/// | 周期 | 股票、基金 | 债券 | 指数 |
/// |---|---|---|---|
/// | 日/周/月/季/年（9、5、6、10、11） | 手 | 百手 | 百手 |
/// | 分钟线（0~3、7、8）与周期 4 | 股 | 手 | 见下 |
///
/// 指数的分钟线没有成交量：该字段在协议里是**成交额的百分之一**，无法换算，
/// 原样返回；指数的周期 4 是手。月/季/年按日线同族处理，未单独实测。
pub(super) fn vol_in_hands(
    raw: f64,
    category: u16,
    index_layout: bool,
    kind: super::PriceKind,
) -> f64 {
    let day_family = matches!(category, 5 | 6 | 9 | 10 | 11);
    let bond = kind == super::PriceKind::Bond;
    match (day_family, index_layout || bond) {
        (true, true) => raw * 100., // 百手 → 手
        (true, false) => raw,
        (false, true) => raw, // 债券已是手；指数分钟线无成交量，原样返回
        (false, false) => raw / 100., // 股 → 手
    }
}

/// 从响应体第 2 字节起读取最多 `n` 条记录，每条记录尾部跳过 `extra` 字节
/// （股票 0、指数 4）。返回已读出的记录与结束位置；截断时提前停止。
pub(super) fn read_bars(v: &[u8], category: u16, n: usize, extra: usize) -> (Vec<BarHead>, usize) {
    let mut pos = 2usize;
    let mut bars = Vec::with_capacity(n.min(800));
    while bars.len() < n {
        let mut next = pos;
        let Some(bar) = bar_head(v, &mut next, category) else {
            break;
        };
        let Some(end) = next.checked_add(extra).filter(|end| *end <= v.len()) else {
            break;
        };
        pos = end;
        bars.push(bar);
    }
    (bars, pos)
}

/// 逐字段安全读取一条记录的公共前缀；截断或 varint 溢出时返回 None。
pub(super) fn bar_head(v: &[u8], pos: &mut usize, category: u16) -> Option<BarHead> {
    use crate::tcp::helper::{datetime, price_checked};

    let dt = datetime(v.get(*pos..pos.checked_add(4)?)?, category);
    *pos += 4;
    let mut prices = [0i64; 4];
    for p in &mut prices {
        *p = i64::from(price_checked(v, pos)?);
    }
    let vol = read_u32(v, pos)?;
    let amount = read_u32(v, pos)?;
    Some(BarHead {
        dt,
        prices,
        vol,
        amount,
    })
}

#[test]
fn day_new_modify() {
    let day = Kline::new(0, "000001", 9, 0, 3);
    let mut day2 = Kline::new(1, "000000", 0, 1, 1);
    day2.market(0).code("000001").category(9).start(0).count(3);
    compare!(Kline::default(), day, day2);
}

#[test]
fn connection() -> std::io::Result<()> {
    crate::tcp::tests::connection(Kline::default())
}

#[test]
fn parse() {
    let mut day = Kline::default();
    let arr = vec![
        0x03, 0x00, 0xeb, 0x64, 0x34, 0x01, 0xb4, 0x9a, 0x02, 0xe4, 0x06, 0x9c, 0x03, 0xc2, 0x07,
        0xe8, 0x6f, 0xa8, 0x49, 0x59, 0xf7, 0x12, 0x4f, 0xec, 0x64, 0x34, 0x01, 0xd0, 0x01, 0xfa,
        0x03, 0x90, 0x01, 0xc4, 0x04, 0x00, 0x81, 0x9a, 0x49, 0xb7, 0xb1, 0x03, 0x4f, 0xef, 0x64,
        0x34, 0x01, 0xcc, 0x02, 0xa8, 0x05, 0x96, 0x07, 0xd6, 0x02, 0xd8, 0x3d, 0x8b, 0x49, 0x4b,
        0xf0, 0xeb, 0x4e,
    ];
    let res = [
        KlineData {
            dt: DateTime {
                year: 2021,
                month: 9,
                day: 23,
                hour: 15,
                minute: 0,
            },
            code: "000001",
            open: 18.1,
            close: 17.68,
            high: 18.32,
            low: 17.65,
            vol: 1379837.0,
            amount: 2465683712.0,
        },
        KlineData {
            dt: DateTime {
                year: 2021,
                month: 9,
                day: 24,
                hour: 15,
                minute: 0,
            },
            code: "000001",
            open: 17.6,
            close: 17.35,
            high: 17.68,
            low: 17.34,
            vol: 1265696.0,
            amount: 2209462016.0,
        },
        KlineData {
            dt: DateTime {
                year: 2021,
                month: 9,
                day: 27,
                hour: 15,
                minute: 0,
            },
            code: "000001",
            open: 17.21,
            close: 17.57,
            high: 17.68,
            low: 17.06,
            vol: 1140667.0,
            amount: 1979196800.0,
        },
    ];
    day.parse(arr);
    compare!(res, day.data.as_slice());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;

    /// 一条完整合法的日线记录（2021-09-23，#sz000001#）。
    const BAR: [u8; 21] = [
        0xeb, 0x64, 0x34, 0x01, 0xb4, 0x9a, 0x02, 0xe4, 0x06, 0x9c, 0x03, 0xc2, 0x07, 0xe8, 0x6f,
        0xa8, 0x49, 0x59, 0xf7, 0x12, 0x4f,
    ];

    fn body(count: u16, bars: &[&[u8]]) -> Vec<u8> {
        let mut v = count.to_le_bytes().to_vec();
        bars.iter().for_each(|b| v.extend_from_slice(b));
        v
    }

    /// 实盘出现过的异常响应：声明 800 根却没有任何记录，不能当成「历史尽头」。
    #[test]
    fn declared_without_body_is_invalid() {
        let mut day = Kline::new(0, "000001", 9, 0, 5);
        let err = day.try_parse(vec![0x20, 0x03]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);
        let msg = err.to_string();
        assert!(msg.contains("800") && msg.contains("实际解析 0"), "{msg}");
        assert!(day.data.is_empty());
        assert_eq!(day.response, [0x20, 0x03]);

        // 旧入口不 panic，同样不留下数据。
        day.parse(vec![0x20, 0x03]);
        assert!(day.data.is_empty());
        assert_eq!(day.response, [0x20, 0x03]);
    }

    /// 声明 0 根（停牌/无历史）是合法的「真空」。
    #[test]
    fn declared_zero_is_ok_and_empty() {
        let mut day = Kline::new(0, "000001", 9, 0, 5);
        day.try_parse(vec![0x00, 0x00]).unwrap();
        assert!(day.data.is_empty());
        assert_eq!(day.response, [0x00, 0x00]);
    }

    #[test]
    fn too_short_body_is_invalid() {
        for v in [vec![], vec![0x01]] {
            let mut day = Kline::new(0, "000001", 9, 0, 5);
            assert_eq!(
                day.try_parse(v.clone()).unwrap_err().kind(),
                ErrorKind::InvalidData
            );
            assert!(day.data.is_empty());
            assert_eq!(day.response, v);
        }
    }

    /// 在任意位置截断都返回错误且不留下部分数据。
    #[test]
    fn truncated_record_is_invalid() {
        let full = body(2, &[&BAR, &BAR]);
        for len in 2..full.len() {
            let mut day = Kline::new(0, "000001", 9, 0, 2);
            let err = day.try_parse(full[..len].to_vec()).unwrap_err();
            assert_eq!(err.kind(), ErrorKind::InvalidData, "len={len}");
            assert!(day.data.is_empty(), "len={len}");
            assert_eq!(day.response.len(), len);
            day.parse(full[..len].to_vec());
            assert!(day.data.is_empty(), "len={len}");
        }
    }

    /// 超长 varint（连续的续位字节）不能 panic（debug 下曾移位溢出）。
    #[test]
    fn overlong_varint_does_not_panic() {
        for filler in [0x80u8, 0xff] {
            let mut v = body(1, &[&BAR[..4]]);
            v.extend_from_slice(&[filler; 64]);
            let mut day = Kline::new(0, "000001", 9, 0, 1);
            assert_eq!(
                day.try_parse(v.clone()).unwrap_err().kind(),
                ErrorKind::InvalidData
            );
            day.parse(v);
            assert!(day.data.is_empty());
        }
    }

    #[test]
    fn single_valid_record() {
        let mut day = Kline::new(0, "000001", 9, 0, 1);
        day.try_parse(body(1, &[&BAR])).unwrap();
        assert_eq!(day.data.len(), 1);
        let bar = &day.data[0];
        assert_eq!(
            bar.dt,
            DateTime {
                year: 2021,
                month: 9,
                day: 23,
                hour: 15,
                minute: 0
            }
        );
        assert_eq!(bar.code, "000001");
        assert_eq!(
            (bar.open, bar.close, bar.high, bar.low),
            (18.1, 17.68, 18.32, 17.65)
        );
        assert_eq!((bar.vol, bar.amount), (1379837.0, 2465683712.0));
    }

    /// 服务器返回少于请求数（历史尽头）是合法的；多于请求数时只取请求的根数。
    #[test]
    fn fewer_or_more_than_requested() {
        let mut day = Kline::new(0, "000001", 9, 0, 5);
        day.try_parse(body(1, &[&BAR])).unwrap();
        assert_eq!(day.data.len(), 1);
        // 同一对象复用：上一次结果较少不影响下一次解析的根数。
        day.try_parse(body(2, &[&BAR, &BAR])).unwrap();
        assert_eq!(day.data.len(), 2);

        let mut day = Kline::new(0, "000001", 9, 0, 1);
        day.try_parse(body(2, &[&BAR, &BAR])).unwrap();
        assert_eq!(day.data.len(), 1);
    }

    /// 指数代码走股票 K 线命令时，服务器按指数格式回应（每条多 4 字节）。
    /// 必须识别出来：否则从第二条起错位，得到看似合理的错误数字。
    #[test]
    fn index_layout_is_detected_by_exact_consumption() {
        const REC: [u8; 21] = [
            0xeb, 0x64, 0x34, 0x01, 0xb4, 0x9a, 0x02, 0xe4, 0x06, 0x9c, 0x03, 0xc2, 0x07, 0xe8,
            0x6f, 0xa8, 0x49, 0x59, 0xf7, 0x12, 0x4f,
        ];
        let mut later = REC;
        later[0] = 0xec; // 次日，价格差值相同
        let build = |extra: &[u8]| {
            let mut v = vec![2, 0];
            for rec in [&REC, &later] {
                v.extend_from_slice(rec);
                v.extend_from_slice(extra);
            }
            v
        };

        // 股票格式：成交量不变
        let mut stock = Kline::new(1, "600000", 9, 0, 2);
        stock.try_parse(build(&[])).unwrap();
        // 指数格式：两条记录的价格与股票格式逐项相同，成交量由百手换算为手
        let mut index = Kline::new(1, "000001", 9, 0, 2);
        index.try_parse(build(&[0x2c, 0x01, 0xc8, 0x00])).unwrap();
        assert_eq!(index.data.len(), 2);
        for (a, b) in stock.data.iter().zip(&index.data) {
            assert_eq!(a.dt, b.dt);
            assert_eq!(
                (a.open, a.close, a.high, a.low),
                (b.open, b.close, b.high, b.low)
            );
            assert_eq!(a.amount, b.amount);
            assert_eq!(a.vol * 100., b.vol);
        }
        assert_eq!(stock.data[0].vol, 1379837.0);
        // 第二条没有错位：开盘 = 前一条 (open + close 差值) 再加自身 open 差值
        assert_eq!(index.data[1].open, (18100 - 420 + 18100) as f64 / 1000.);

        // 债券按股票格式，成交量同样由百手换算为手
        let mut bond = Kline::new(1, "113042", 9, 0, 2);
        bond.try_parse(build(&[])).unwrap();
        assert_eq!(bond.data[0].vol, 137983700.0);
    }

    /// 原始成交量单位随周期与品种变化，统一换算为手。
    #[test]
    fn vol_units_are_unified_to_hands() {
        use crate::tcp::stock::PriceKind::{Bond, Fund, Stock};
        // 日线同族：股票/基金是手，债券与指数是百手
        for cat in [9, 5, 6, 10, 11] {
            assert_eq!(vol_in_hands(500., cat, false, Stock), 500.);
            assert_eq!(vol_in_hands(500., cat, false, Fund), 500.);
            assert_eq!(vol_in_hands(500., cat, false, Bond), 50000.);
            assert_eq!(vol_in_hands(500., cat, true, Stock), 50000.);
        }
        // 分钟线与周期 4：股票/基金是股，债券是手；指数原样返回
        for cat in [0, 1, 2, 3, 7, 8, 4] {
            assert_eq!(vol_in_hands(50451600., cat, false, Stock), 504516.);
            assert_eq!(vol_in_hands(50451600., cat, false, Fund), 504516.);
            assert_eq!(vol_in_hands(500., cat, false, Bond), 500.);
            assert_eq!(vol_in_hands(500., cat, true, Stock), 500.);
        }
    }
}
