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

    /// 旧入口（不可失败）：等价于忽略 [`Tdx::try_parse`] 的错误。
    ///
    /// 畸形/截断的响应不会 panic，也不会留下部分数据：此时 `data` 为空、
    /// `response` 仍保存原始字节。
    fn parse(&mut self, v: Vec<u8>) {
        let _ = self.try_parse(v);
    }

    /// 严格解析响应。每根K线 = datetime(4) + 开/收/高/低(变长) + vol(u32) + amount(u32)
    /// + 上涨家数(u16) + 下跌家数(u16)。
    ///
    /// 价格为相对前一根收盘价的差值累加（÷1000），与股票K线相同。
    ///
    /// 与 [`Kline`](super::Kline) 同款规则：响应体不足 2 字节、或实际解析出的记录数
    /// < min(响应头声明数, 请求数) 时返回 [`std::io::ErrorKind::InvalidData`]
    /// （由连接层重试/换服务器）；声明数为 0 则成功且结果为空。
    /// 出错时 `data` 被清空，`response` 始终保存原始字节。
    fn try_parse(&mut self, v: Vec<u8>) -> std::io::Result<()> {
        use super::kline::{bar_head, declared_count, incomplete};
        use crate::tcp::helper::vol_amount;

        self.data.clear();
        self.response = v;
        let v = &self.response;
        let declared = declared_count(v, Self::TAG)?;
        let expected = declared.min(self.count as usize);

        let (mut pos, mut base) = (2usize, 0i64);
        let mut data = Vec::with_capacity(expected);
        while data.len() < expected {
            let Some(bar) = bar_head(v, &mut pos, self.category) else {
                break;
            };
            // 涨跌家数：u16 + u16
            let Some(&[u0, u1, d0, d1]) = v.get(pos..pos + 4).and_then(|s| s.first_chunk::<4>())
            else {
                break;
            };
            pos += 4;

            let [open, close, high, low] = bar.prices;
            base += open;
            data.push(IndexKlineData {
                dt: bar.dt,
                code: self.code,
                open: base as f64 / 1000.,
                close: real_price(close, base),
                high: real_price(high, base),
                low: real_price(low, base),
                vol: super::kline::vol_in_hands(
                    vol_amount(bar.vol as i32),
                    self.category,
                    true,
                    super::PriceKind::Stock,
                ),
                amount: vol_amount(bar.amount as i32),
                up_count: u16::from_le_bytes([u0, u1]),
                down_count: u16::from_le_bytes([d0, d1]),
            });
            base += close;
        }
        if data.len() < expected {
            return Err(incomplete(Self::TAG, declared, self.count, data.len()));
        }
        // 落实升序契约（与 Kline 相同）
        data.sort_by_key(|b| b.dt);
        self.data = data;
        Ok(())
    }

    fn result(&self) -> &Self::Item {
        &self.data
    }
}

/// 价格以 i64 累加：畸形数据也不会让差值累加溢出（合法数据下结果与 i32 相同）。
#[inline]
fn real_price(p: i64, base: i64) -> f64 {
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
    /// 成交量，单位：手，与行情快照口径一致（日线在协议里是百手，解析时已换算）。
    /// 例外：**分钟线没有成交量**，此字段是成交额的百分之一。
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
    use std::io::ErrorKind;

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

    /// 截断的响应不 panic，且旧入口不留下部分数据、保存原始字节。
    #[test]
    fn parse_truncated_no_panic() {
        // count=2 但只有 1 个 datetime，无价格字段
        let v = vec![0x02, 0x00, 0x3f, 0xb3, 0x00, 0x00];
        let mut kline = IndexKline::new(1, "000001", 9, 0, 2);
        kline.parse(v.clone()); // 不应 panic
        assert_eq!(kline.response.len(), 6);
        assert!(kline.data.is_empty());
        assert_eq!(
            kline.try_parse(v).unwrap_err().kind(),
            ErrorKind::InvalidData
        );
    }

    /// 一条完整合法的指数日线记录（2021-09-23）：公共前缀 + 上涨 300 家 + 下跌 200 家。
    const BAR: [u8; 25] = [
        0xeb, 0x64, 0x34, 0x01, 0xb4, 0x9a, 0x02, 0xe4, 0x06, 0x9c, 0x03, 0xc2, 0x07, 0xe8, 0x6f,
        0xa8, 0x49, 0x59, 0xf7, 0x12, 0x4f, 0x2c, 0x01, 0xc8, 0x00,
    ];

    fn body(count: u16, bars: &[&[u8]]) -> Vec<u8> {
        let mut v = count.to_le_bytes().to_vec();
        bars.iter().for_each(|b| v.extend_from_slice(b));
        v
    }

    /// 实盘出现过的异常响应：声明 800 根却没有任何记录，不能当成「历史尽头」。
    #[test]
    fn declared_without_body_is_invalid() {
        let mut kline = IndexKline::new(1, "000001", 9, 0, 5);
        let err = kline.try_parse(vec![0x20, 0x03]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);
        let msg = err.to_string();
        assert!(msg.contains("800") && msg.contains("实际解析 0"), "{msg}");
        assert!(kline.data.is_empty());
        assert_eq!(kline.response, [0x20, 0x03]);
    }

    /// 声明 0 根是合法的「真空」。
    #[test]
    fn declared_zero_is_ok_and_empty() {
        let mut kline = IndexKline::new(1, "000001", 9, 0, 5);
        kline.try_parse(vec![0x00, 0x00]).unwrap();
        assert!(kline.data.is_empty());
        assert_eq!(kline.response, [0x00, 0x00]);
    }

    #[test]
    fn too_short_body_is_invalid() {
        for v in [vec![], vec![0x01]] {
            let mut kline = IndexKline::new(1, "000001", 9, 0, 5);
            assert_eq!(
                kline.try_parse(v).unwrap_err().kind(),
                ErrorKind::InvalidData
            );
            assert!(kline.data.is_empty());
        }
    }

    /// 在任意位置截断（含涨跌家数字段）都返回错误且不留下部分数据。
    #[test]
    fn truncated_record_is_invalid() {
        let full = body(2, &[&BAR, &BAR]);
        for len in 2..full.len() {
            let mut kline = IndexKline::new(1, "000001", 9, 0, 2);
            let err = kline.try_parse(full[..len].to_vec()).unwrap_err();
            assert_eq!(err.kind(), ErrorKind::InvalidData, "len={len}");
            assert!(kline.data.is_empty(), "len={len}");
            assert_eq!(kline.response.len(), len);
            kline.parse(full[..len].to_vec());
            assert!(kline.data.is_empty(), "len={len}");
        }
    }

    /// 超长 varint（连续的续位字节）不能 panic。
    #[test]
    fn overlong_varint_does_not_panic() {
        for filler in [0x80u8, 0xff] {
            let mut v = body(1, &[&BAR[..4]]);
            v.extend_from_slice(&[filler; 64]);
            let mut kline = IndexKline::new(1, "000001", 9, 0, 1);
            assert_eq!(
                kline.try_parse(v.clone()).unwrap_err().kind(),
                ErrorKind::InvalidData
            );
            kline.parse(v);
            assert!(kline.data.is_empty());
        }
    }

    #[test]
    fn single_valid_record() {
        let mut kline = IndexKline::new(1, "000001", 9, 0, 1);
        kline.try_parse(body(1, &[&BAR])).unwrap();
        assert_eq!(kline.data.len(), 1);
        let bar = &kline.data[0];
        assert_eq!(
            (bar.dt.year, bar.dt.month, bar.dt.day, bar.dt.hour),
            (2021, 9, 23, 15)
        );
        assert_eq!(bar.code, "000001");
        assert_eq!(
            (bar.open, bar.close, bar.high, bar.low),
            (18.1, 17.68, 18.32, 17.65)
        );
        assert_eq!((bar.vol, bar.amount), (137983700.0, 2465683712.0));
        assert_eq!((bar.up_count, bar.down_count), (300, 200));
    }

    /// 多条记录：差值累加与升序排序；少于请求数（历史尽头）合法。
    #[test]
    fn multiple_records_sorted_ascending() {
        // 第二条日期改为 2021-09-22（0x0134_64ea），应排到前面
        let mut earlier = BAR;
        earlier[0] = 0xea;
        let mut kline = IndexKline::new(1, "000001", 9, 0, 5);
        kline.try_parse(body(2, &[&BAR, &earlier])).unwrap();
        assert_eq!(kline.data.len(), 2);
        assert_eq!((kline.data[0].dt.day, kline.data[1].dt.day), (22, 23));
        // 第二条的开盘价 = 前一条 (open + close 差值) 累加后再加自身 open 差值
        assert_eq!(kline.data[0].open, (18100 - 420 + 18100) as f64 / 1000.);
    }
}
