use crate::tcp::Tdx;
use crate::bytes_helper::u16_from_le_bytes;

/// 获取历史分时数据。对应于 pytdx 的 hq.get_history_minute_time_data。
///
/// ## 注意
/// - 返回指定日期的分时成交数据（每分钟一个数据点）
/// - 只能查询近期交易日（服务器保留期约 1-2 周）
/// - ⚠️ 与当日分时（[`MinuteTime`](super::MinuteTime)）一样，部分服务器的
///   分时协议在 2026 年后发生变化，解析异常时返回空数据（内置防御性校验）
///
/// ## 示例
/// ```ignore
/// use rustdx_complete::tcp::{Tcp, Tdx};
/// use rustdx_complete::tcp::stock::HistoryMinuteTime;
///
/// let mut tcp = Tcp::new()?;
/// let mut mt = HistoryMinuteTime::new(0, "000001", 20260901);
/// mt.recv_parsed(&mut tcp)?;
/// for data in mt.result() {
///     println!("价格={} 成交量={}", data.price, data.vol);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct HistoryMinuteTime<'d> {
    pub send: Box<[u8]>,
    pub market: u16,
    pub code: &'d str,
    /// 查询日期，格式 YYYYMMDD
    pub date: u32,
    pub response: Vec<u8>,
    pub data: Vec<super::minute_time::MinuteTimeData>,
}

impl<'d> HistoryMinuteTime<'d> {
    /// 创建历史分时查询请求。
    ///
    /// ## 参数
    /// - `market`: 市场代码（0=深市, 1=沪市）
    /// - `code`: 6位股票代码
    /// - `date`: 查询日期（YYYYMMDD）
    pub fn new(market: u16, code: &'d str, date: u32) -> Self {
        assert_eq!(code.len(), 6, "股票代码必须是6位");

        let mut send = [0u8; Self::LEN];
        send[0..12].copy_from_slice(Self::SEND);
        // body: date(u32 LE) + market(u8) + code(6字节)
        send[12..16].copy_from_slice(&date.to_le_bytes());
        send[16] = market as u8;
        send[17..23].copy_from_slice(code.as_bytes());

        Self {
            send: send.into(),
            market,
            code,
            date,
            response: Vec::new(),
            data: Vec::new(),
        }
    }
}

impl<'a> Tdx for HistoryMinuteTime<'a> {
    type Item = [super::minute_time::MinuteTimeData];

    /// 历史分时的请求字节。包头 12 字节 + date(4) + market(1) + code(6) = 23 字节。
    /// （SEND 只包含包头；body 由 [`HistoryMinuteTime::new`] 填充。）
    const SEND: &'static [u8] = &[
        0x0c, 0x01, 0x30, 0x00, 0x01, 0x01, 0x0d, 0x00, 0x0d, 0x00, 0xb4, 0x0f,
    ];

    const TAG: &'static str = "历史分时";
    const LEN: usize = 12 + 4 + 1 + 6;

    fn send(&mut self) -> &[u8] {
        &self.send
    }

    /// 解析响应：数量(2) + 跳过4 + 每点 3 个变长编码（与当日分时相同的数据编码）。
    fn parse(&mut self, v: Vec<u8>) {
        self.data = Vec::new();

        if v.len() < 6 {
            self.response = v;
            return;
        }

        let num_points = u16_from_le_bytes(&v, 0);
        let mut pos = 6; // 数量(2) + 跳过4

        let mut last_price = 0i32;

        for _ in 0..num_points {
            if v.len() - pos < 3 {
                break;
            }
            let Some(price_raw) = super::minute_time::price_checked(&v, &mut pos) else {
                self.response = v;
                return;
            };
            let Some(_reversed1) = super::minute_time::price_checked(&v, &mut pos) else {
                self.response = v;
                return;
            };
            let Some(vol) = super::minute_time::price_checked(&v, &mut pos) else {
                self.response = v;
                return;
            };

            last_price += price_raw;
            let price = last_price as f64 / 100.0;

            self.data.push(super::minute_time::MinuteTimeData { price, vol });
        }

        // 防御性校验（同当日分时）：协议变化时返回空数据而非垃圾
        let points_complete = self.data.len() == num_points as usize;
        let bytes_consumed = points_complete && pos + 8 >= v.len();
        let prices_valid = self
            .data
            .iter()
            .all(|d| (0.01..=100000.0).contains(&d.price));

        if !points_complete || !bytes_consumed || !prices_valid {
            self.data = Vec::new();
        }

        self.response = v;
    }

    fn result(&self) -> &Self::Item {
        &self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let mt = HistoryMinuteTime::new(0, "000001", 20260901);
        assert_eq!(mt.market, 0);
        assert_eq!(mt.code, "000001");
        assert_eq!(mt.date, 20260901);
        assert_eq!(mt.send.len(), 23);
        // date 与 market/code 写入发送字节
        assert_eq!(&mt.send[12..16], &20260901u32.to_le_bytes());
        assert_eq!(mt.send[16], 0);
        assert_eq!(&mt.send[17..23], b"000001");
    }

    #[test]
    #[should_panic(expected = "股票代码必须是6位")]
    fn test_invalid_code() {
        HistoryMinuteTime::new(0, "00001", 20260901);
    }

    /// 构造的旧格式响应：2 个点，价格累加还原。
    #[test]
    fn parse_legacy_format() {
        let mut v = vec![0x02, 0x00, 0x00, 0x00, 0x00, 0x00]; // num=2 + 跳过4
        v.extend_from_slice(&[0x84, 0x0e, 0x00, 0xa4, 0x01]); // +900, 0, 100
        v.extend_from_slice(&[0x32, 0x00, 0x90, 0x01]); // +50, 0, 80
        let mut mt = HistoryMinuteTime::new(0, "000001", 20260901);
        mt.parse(v);
        assert_eq!(mt.result().len(), 2);
        assert_eq!(mt.result()[0].price, 9.0);
        assert_eq!(mt.result()[1].price, 9.5);
    }

    /// 截断响应 → 空。
    #[test]
    fn parse_truncated_returns_empty() {
        let mut v = vec![0x0a, 0x00, 0x00, 0x00, 0x00, 0x00];
        v.extend_from_slice(&[0x84, 0x0e, 0x00, 0xa4, 0x01]);
        let mut mt = HistoryMinuteTime::new(0, "000001", 20260901);
        mt.parse(v);
        assert!(mt.result().is_empty());
    }
}
