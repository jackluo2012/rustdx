use crate::tcp::Tdx;

/// 获取指数K线。对应于 pytdx 的 hq.get_index_bars、GetIndexBarsCmd。
///
/// 请求与股票K线（[`Kline`](super::Kline)）相同（命令 0x052d），
/// 但**响应格式不同**：每根K线在成交量/成交额之后多 4 字节
/// （上涨家数 u16 + 下跌家数 u16），因此需要独立的解析。
///
/// ## 示例
/// ```ignore
/// use rustdx_complete::tcp::{Tcp, Tdx};
/// use rustdx_complete::tcp::stock::IndexKline;
///
/// let mut tcp = Tcp::new()?;
/// let mut kline = IndexKline::new(1, "000001", 9, 0, 10); // 上证指数日线
/// kline.recv_parsed(&mut tcp)?;
/// for bar in kline.result() {
///     println!("{}-{:02}-{:02} 收{:.2} 上涨{}家 下跌{}家",
///         bar.dt.year, bar.dt.month, bar.dt.day, bar.close, bar.up_count, bar.down_count);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct IndexKline<'d> {
    pub send: Box<[u8]>,
    pub market: u16,
    pub code: &'d str,
    pub category: u16,
    pub start: u16,
    pub count: u16,
    pub response: Vec<u8>,
    pub data: Vec<IndexKlineData<'d>>,
}

impl<'d> IndexKline<'d> {
    /// 创建指数K线请求。参数含义同 [`Kline::new`](super::Kline::new)。
    ///
    /// ## panic
    /// 当 code 的字节长度不是 6 时，程序会 panic。
    pub fn new(market: u16, code: &'d str, category: u16, start: u16, count: u16) -> Self {
        assert_eq!(code.len(), 6, "股票代码必须是6位");
        let mut send = [0u8; Self::LEN];
        // SEND 为完整 38 字节（含默认占位），全量复制后覆盖参数字段
        send.copy_from_slice(Self::SEND);
        send[12..14].copy_from_slice(&market.to_le_bytes());
        send[14..20].copy_from_slice(code.as_bytes());
        send[20..22].copy_from_slice(&category.to_le_bytes());
        send[22..24].copy_from_slice(&1u16.to_le_bytes());
        send[24..26].copy_from_slice(&start.to_le_bytes());
        send[26..28].copy_from_slice(&count.to_le_bytes());
        // 28..38 保持为 0
        Self {
            send: send.into(),
            market,
            code,
            category,
            start,
            count,
            response: Vec::new(),
            data: vec![IndexKlineData::default(); count as usize],
        }
    }
}

impl<'a> Tdx for IndexKline<'a> {
    type Item = [IndexKlineData<'a>];

    /// 与股票K线相同的请求字节（命令 0x052d），默认 sz000001 日线 3 根。
    const SEND: &'static [u8] = &[
        0x0c, 0x01, 0x08, 0x64, 0x01, 0x01, 0x1c, 0x00, 0x1c, 0x00, 0x2d, 0x05, 0x00, 0x00, 0x30,
        0x30, 0x30, 0x30, 0x30, 0x31, 0x09, 0x00, 0x01, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    const TAG: &'static str = "指数K线";

    fn send(&mut self) -> &[u8] {
        &self.send
    }

    /// 解析响应。每根K线 = datetime(4) + 开/收/高/低(变长) + vol(u32) + amount(u32)
    /// + 上涨家数(u16) + 下跌家数(u16)。
    ///
    /// 价格为相对前一根收盘价的差值累加（÷1000），与股票K线相同。
    fn parse(&mut self, v: Vec<u8>) {
        use crate::bytes_helper::{u16_from_le_bytes, u32_from_le_bytes};
        use crate::tcp::helper::{datetime, price, vol_amount};

        if v.len() < 2 {
            self.response = v;
            return;
        }
        let (count, mut pos, mut base) = (u16_from_le_bytes(&v, 0), 2usize, 0i32);
        let n = (count as usize).min(self.data.len());
        for item in self.data.iter_mut().take(n) {
            if v.len() - pos < 4 + 4 + 8 + 4 {
                break;
            }
            let dt = datetime(&v[pos..pos + 4], self.category);
            pos += 4;
            let open = price(&v, &mut pos);
            let close = price(&v, &mut pos);

            *item = IndexKlineData {
                dt,
                code: self.code,
                open: {
                    base += open;
                    base as f64 / 1000.
                },
                close: real_price(close, base),
                high: real_price(price(&v, &mut pos), base),
                low: real_price(price(&v, &mut pos), base),
                vol: {
                    pos += 4;
                    vol_amount(u32_from_le_bytes(&v, pos - 4) as i32)
                },
                amount: {
                    pos += 4;
                    vol_amount(u32_from_le_bytes(&v, pos - 4) as i32)
                },
                up_count: u16_from_le_bytes(&v, pos),
                down_count: u16_from_le_bytes(&v, pos + 2),
            };
            pos += 4;

            base += close;
        }
        self.response = v;
    }

    fn result(&self) -> &Self::Item {
        &self.data
    }
}

#[inline]
fn real_price(p: i32, base: i32) -> f64 {
    (p + base) as f64 / 1000.
}

/// 指数K线数据点。与 [`KlineData`](super::KlineData) 相比多了涨跌家数。
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct IndexKlineData<'d> {
    pub dt: crate::tcp::helper::DateTime,
    pub code: &'d str,
    pub open: f64,
    pub close: f64,
    pub high: f64,
    pub low: f64,
    /// 成交量
    pub vol: f64,
    /// 成交额
    pub amount: f64,
    /// 上涨家数
    pub up_count: u16,
    /// 下跌家数
    pub down_count: u16,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_send_bytes() {
        let kline = IndexKline::new(1, "000001", 9, 0, 5);
        assert_eq!(kline.send.len(), 38);
        assert_eq!(kline.market, 1);
        assert_eq!(&kline.send[12..14], &[1, 0]);
        assert_eq!(&kline.send[14..20], b"000001");
        assert_eq!(&kline.send[20..22], &[9, 0]); // category=9 日线
        assert_eq!(&kline.send[24..26], &[0, 0]); // start=0
        assert_eq!(&kline.send[26..28], &[5, 0]); // count=5
    }

    #[test]
    #[should_panic(expected = "股票代码必须是6位")]
    fn test_invalid_code() {
        IndexKline::new(1, "00000", 9, 0, 5);
    }

    /// 截断的响应不 panic（真实字节的完整解析验证由 live 验证完成）。
    #[test]
    fn parse_truncated_no_panic() {
        // count=2 但只有 1 个 datetime，无价格字段
        let v = vec![0x02, 0x00, 0x3f, 0xb3, 0x00, 0x00];
        let mut kline = IndexKline::new(1, "000001", 9, 0, 2);
        kline.parse(v); // 不应 panic
        assert_eq!(kline.response.len(), 6);
    }
}
