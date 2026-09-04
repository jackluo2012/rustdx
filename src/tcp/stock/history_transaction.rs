use crate::bytes_helper::u16_from_le_bytes;
use crate::tcp::Tdx;

/// 获取历史逐笔成交数据。对应于 pytdx 的 hq.get_history_transaction_data。
///
/// ## 注意
/// - 返回指定日期的逐笔成交（tick 级别）
/// - 只能查询近期交易日
/// - 买卖方向：0=买盘, 1=卖盘, 2=中性盘（pytdx 文档值）。⚠️ 2026-09 实测历史
///   逐笔中还会出现 5、8 等值（疑似集合竞价/特殊成交标记），服务器语义未公开，
///   使用时建议以 0/1 区分主买卖，其余值单独处理
///
/// ## 示例
/// ```ignore
/// use rustdx_complete::tcp::{Tcp, Tdx};
/// use rustdx_complete::tcp::stock::HistoryTransaction;
///
/// let mut tcp = Tcp::new()?;
/// let mut tx = HistoryTransaction::new(0, "000001", 0, 100, 20260901);
/// tx.recv_parsed(&mut tcp)?;
/// for t in tx.result() {
///     println!("{} 价格={:.2} 量={} 方向={}", t.time, t.price, t.vol, t.buyorsell);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct HistoryTransaction<'d> {
    pub send: Box<[u8]>,
    pub market: u16,
    pub code: &'d str,
    pub start: u16,
    pub count: u16,
    /// 查询日期，格式 YYYYMMDD
    pub date: u32,
    pub response: Vec<u8>,
    pub data: Vec<super::transaction::TransactionData>,
}

impl<'d> HistoryTransaction<'d> {
    /// 创建历史逐笔成交查询请求。
    ///
    /// ## 参数
    /// - `market`: 市场代码（0=深市, 1=沪市）
    /// - `code`: 6位股票代码
    /// - `start`: 起始位置（分页）
    /// - `count`: 获取数量
    /// - `date`: 查询日期（YYYYMMDD）
    pub fn new(market: u16, code: &'d str, start: u16, count: u16, date: u32) -> Self {
        assert_eq!(code.len(), 6, "股票代码必须是6位");

        let mut send = [0u8; Self::LEN];
        // SEND 只包含包头 12 字节（原定义含占位 body 时会与 LEN 不符，见 new 的分段填充）
        send[0..12].copy_from_slice(&Self::SEND[0..12]);
        // body: date(u32) + market(u16) + code(6) + start(u16) + count(u16)
        send[12..16].copy_from_slice(&date.to_le_bytes());
        send[16..18].copy_from_slice(&market.to_le_bytes());
        send[18..24].copy_from_slice(code.as_bytes());
        send[24..26].copy_from_slice(&start.to_le_bytes());
        send[26..28].copy_from_slice(&count.to_le_bytes());

        Self {
            send: send.into(),
            market,
            code,
            start,
            count,
            date,
            response: Vec::new(),
            data: Vec::new(),
        }
    }
}

impl<'a> Tdx for HistoryTransaction<'a> {
    type Item = [super::transaction::TransactionData];

    /// 历史逐笔的请求字节。包头 12 字节 + date(4)+market(2)+code(6)+start(2)+count(2) = 28 字节。
    const SEND: &'static [u8] = &[
        0x0c, 0x01, 0x30, 0x01, 0x00, 0x01, 0x12, 0x00, 0x12, 0x00, 0xb5,
        0x0f, // 包头 12 字节
        // date u32 + market u16 + code 6字节 + start u16 + count u16（占位）
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x30, 0x30, 0x30, 0x30, 0x30, 0x31, 0x00, 0x00, 0x00,
        0x00,
    ];

    const TAG: &'static str = "历史逐笔成交";
    const LEN: usize = 12 + 4 + 2 + 6 + 2 + 2;

    fn send(&mut self) -> &[u8] {
        &self.send
    }

    /// 解析响应：数量(2) + 跳过4 + 每笔 [时间u16 + price/vol/buyorsell/保留 各一个变长编码]。
    fn parse(&mut self, v: Vec<u8>) {
        self.data = Vec::new();

        if v.len() < 6 {
            self.response = v;
            return;
        }

        let num_ticks = u16_from_le_bytes(&v, 0);
        let mut pos = 6; // 数量(2) + 跳过4

        let mut last_price = 0i32;

        for _ in 0..num_ticks {
            if v.len() - pos < 3 {
                break;
            }
            // 时间（2字节：分钟数，如 0x0380 = 896 → 14:56）
            let time_minutes = u16_from_le_bytes(&v, pos);
            pos += 2;
            let hour = time_minutes / 60;
            let minute = time_minutes % 60;

            let Some(price_raw) = super::minute_time::price_checked(&v, &mut pos) else {
                self.response = v;
                return;
            };
            let Some(vol) = super::minute_time::price_checked(&v, &mut pos) else {
                self.response = v;
                return;
            };
            let Some(buyorsell) = super::minute_time::price_checked(&v, &mut pos) else {
                self.response = v;
                return;
            };
            let Some(_reserved) = super::minute_time::price_checked(&v, &mut pos) else {
                self.response = v;
                return;
            };

            last_price += price_raw;
            let price = last_price as f64 / 100.0;

            self.data.push(super::transaction::TransactionData {
                time: format!("{hour:02}:{minute:02}"),
                price,
                vol,
                num: 0, // 历史逐笔协议中无成交编号字段
                buyorsell,
            });
        }

        // 防御性校验：点数完整 + 字节基本耗尽 + 价格合理
        let ticks_complete = self.data.len() == num_ticks as usize;
        let bytes_consumed = ticks_complete && pos + 8 >= v.len();
        let prices_valid = self
            .data
            .iter()
            .all(|t| (0.01..=100000.0).contains(&t.price));

        if !ticks_complete || !bytes_consumed || !prices_valid {
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
        let tx = HistoryTransaction::new(0, "000001", 20, 50, 20260901);
        assert_eq!(tx.market, 0);
        assert_eq!(tx.code, "000001");
        assert_eq!(tx.start, 20);
        assert_eq!(tx.count, 50);
        assert_eq!(tx.date, 20260901);
        assert_eq!(tx.send.len(), 28);
        assert_eq!(&tx.send[12..16], &20260901u32.to_le_bytes());
        assert_eq!(&tx.send[16..18], &0u16.to_le_bytes());
        assert_eq!(&tx.send[18..24], b"000001");
        assert_eq!(&tx.send[24..26], &20u16.to_le_bytes());
        assert_eq!(&tx.send[26..28], &50u16.to_le_bytes());
    }

    #[test]
    #[should_panic(expected = "股票代码必须是6位")]
    fn test_invalid_code() {
        HistoryTransaction::new(0, "00001", 0, 20, 20260901);
    }

    /// 构造的响应：2 笔成交。
    #[test]
    fn parse_two_ticks() {
        // num=2 + 跳过4；笔1: time=09:30(570=0x023a), price=+900, vol=100, dir=0, rsv=0
        let mut v = vec![0x02, 0x00, 0x00, 0x00, 0x00, 0x00];
        v.extend_from_slice(&[0x3a, 0x02]); // 570 分钟 → 09:30
        v.extend_from_slice(&[0x84, 0x0e, 0xa4, 0x01, 0x00, 0x00]); // +900, 100, 0, 0
        // 笔2: time=09:31(571), price=+50, vol=80, dir=1, rsv=0
        v.extend_from_slice(&[0x3b, 0x02]);
        v.extend_from_slice(&[0x32, 0x90, 0x01, 0x01, 0x00]); // +50, 80, +1, 0
        let mut tx = HistoryTransaction::new(0, "000001", 0, 10, 20260901);
        tx.parse(v);
        assert_eq!(tx.result().len(), 2);
        assert_eq!(tx.result()[0].time, "09:30");
        assert_eq!(tx.result()[0].price, 9.0);
        assert_eq!(tx.result()[0].vol, 100);
        assert_eq!(tx.result()[0].buyorsell, 0);
        assert_eq!(tx.result()[1].time, "09:31");
        assert_eq!(tx.result()[1].price, 9.5);
        assert_eq!(tx.result()[1].buyorsell, 1);
    }
}
