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
    /// ## 响应格式
    ///
    /// ### 2026 新协议（实测 2026-09 沪深主板均此格式）
    /// - 头 11 字节：数量(2) + 跳过(2) + market(1) + code(6, ASCII，即请求的代码)
    /// - 之后是**可变个数**的头部 varint（盘口/统计快照：最新价、昨收/开/高/低差值、
    ///   总量等，实测 29~32 个，随内容浮动，无法正向跳过）
    /// - 最后是 `num × 3` 个数据 varint（每分钟一点：价格增量 / 保留 / 成交量），
    ///   **数据块恰好耗尽到响应末尾** → 正向穷举头部结束位置唯一定位
    ///   （见 [`locate_data_block`]），已用历史分时（旧协议）交叉验证逐点吻合
    ///
    /// ### 旧协议（pytdx GetMinuteTimeDataCmd.parseResponse）
    /// - 前 2 字节：数据点数量；字节 2-3 跳过；之后每点 3 个 varint
    ///
    /// 格式判别：头部 code 锚点（`v[5..11] == 请求 code`）命中即新协议，
    /// 否则按旧协议解析。两种格式均带防御性校验（点数完整、价格合理、
    /// 字节耗尽），异常时返回**空数据**而非垃圾数据。
    fn parse(&mut self, v: Vec<u8>) {
        self.data = Vec::new();

        if v.len() >= 4 {
            let num_points = u16_from_le_bytes(&v, 0) as usize;

            // 新协议：头 11 字节中的 code 与请求一致（市场号在 v[4]，0=深 1=沪）
            let is_new = v.len() >= 11
                && v[2..4] == [0, 0]
                && &v[5..11] == self.code.as_bytes();
            if is_new {
                if let Some(data) = locate_data_block(&v, num_points) {
                    self.data = data;
                }
                self.response = v;
                return;
            }

            // 旧协议：头 4 字节（数量 + 跳过）后即数据
            if let Some((data, end)) = decode_points(&v, 4, num_points) {
                let points_complete = data.len() == num_points;
                let bytes_consumed = points_complete && end + 8 >= v.len();
                let prices_valid = data
                    .iter()
                    .all(|d| (0.01..=100000.0).contains(&d.price));
                if points_complete && bytes_consumed && prices_valid {
                    self.data = data;
                }
            }
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

/// 定位 2026 新协议响应中的分时数据块并解码。
///
/// 响应 = 头 11 字节 + 可变个数头部 varint（盘口快照，实测 29~32 个，
/// 随行情内容浮动，无法正向跳过）+ `num × 3` 个数据 varint，数据块
/// **恰好耗尽到响应末尾**。
///
/// varint 编码非自同步（一个 bit7=0 字节既可能是单字节 varint，也可能是
/// 多字节 varint 的末续字节），从尾部反向切分不唯一，故正向穷举头部结束
/// 位置：数据块须恰好读满 `num × 3` 个 varint 且结束于响应末尾，叠加价格
/// 合理性校验（实测沪深主板多只股票下该解唯一）。
fn locate_data_block(v: &[u8], num_points: usize) -> Option<Vec<MinuteTimeData>> {
    // 收集 offset 11 起全部 varint 的起始字节位置
    let mut starts = Vec::new();
    let mut pos = 11;
    while pos < v.len() {
        starts.push(pos);
        price_checked(v, &mut pos)?;
    }

    let need = num_points * 3;
    if need == 0 {
        return Some(Vec::new());
    }
    // 头部结束位置的候选：其后须仍有 N×3 个 varint
    if starts.len() < need {
        return None;
    }
    for &start in &starts[..=starts.len() - need] {
        if let Some((data, end)) = decode_points(v, start, num_points) {
            let prices_valid = data
                .iter()
                .all(|d| (0.01..=100000.0).contains(&d.price));
            if end == v.len() && prices_valid {
                return Some(data);
            }
        }
    }
    None
}

/// 从 `pos` 起解码 `num` 个分时点（每点 3 个 varint：价格增量 / 保留 / 成交量），
/// 价格按增量累加还原，返回数据与结束位置；任一字节越界返回 `None`。
fn decode_points(v: &[u8], pos: usize, num: usize) -> Option<(Vec<MinuteTimeData>, usize)> {
    let mut pos = pos;
    let mut out = Vec::with_capacity(num.min(241));
    let mut last_price = 0i32;
    for _ in 0..num {
        let price_raw = price_checked(v, &mut pos)?;
        let _reversed1 = price_checked(v, &mut pos)?;
        let vol = price_checked(v, &mut pos)?;
        last_price += price_raw;
        out.push(MinuteTimeData {
            price: last_price as f64 / 100.0,
            vol,
        });
    }
    Some((out, pos))
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

    /// 2026-09-07 开盘实测的完整新协议响应（深市 000001，盘中 195 点）：
    /// 头 11 字节（数量+跳过+market+code）+ 可变头部 varint + 195×3 数据 varint，
    /// 数据块顶到响应末尾。反向切分定位数据块后应完整解析。
    #[test]
    fn parse_new_protocol() {
        let hex = "c300000000303030303031320f921213111245af9cbe0dd212bea973a801f933844e859b3dba8e3600b5ec08410090129a6b42018128901d43028b559a308c0fa2126c95d6034241a9d7020066a1e101415690b7010166a1d401415397b1010048ab6d41508ecc0141518c8a01015189a70101478285010041be5d0044a7700001bb7d004087320041be3a414785734149bc6b0245b85c0141b22e4141be360042a032434fb8ea0100459e4f43df01a098064272afd5020049b83e415a81b3010053b38301006a96b5024150968101015bb9dc01414dbd6441508b74014e9f6a4154899501025aa1f30101488a6d4146b7510044843b0145b34e014492424144833e4449b95c024c8a7b41478048415182a8010049895b0147a14a0148a86c414691500148bd634142931a0043b72a0148ad74014693694141801a00419e0e0144aa570042842043468a5a00429d1a0144bb34004dbdbd010042ac1c0041bc0e4142891c014c949d014142931d004386274254b3f3010044a02a0045aa3e0043a91e0148bf644246bd4300438325004d868801014bbe840100439c230042b9110145963d0042af194143992c01489f750041a414414187120143ac2d4244af2f0141961141469b470047964f4144a52f0144b82b4142a61b0049b46b0047ba4e0142a7160045a9440043b0234142bf1301429a150044ab2c01439d2c00428f234141870d004297230143be2b4244bc350043942a0141911000428f1f414190100242ab1f4243bd2742518dd201014695470142a7160042921500439623004ba096010145974a004398330144ad4600429d2542438a320042a5240042bc190047b964014385310041bb0c0042ac1f0141900d00438c44004395400046b37241418e120141841742418d17004182100045b44b4141b30e0042b02800429d1c0141981741449f3b01438a304141910f0046a36301418d160043903d014397304144a54e0042ab2b0041870c00418212414394380044ad3e004290280041b71600408608014485510041bb170041940d0041b3170141840d4140bb090041b2160143bc3c00418f1300419c0f00439a5501409d094140b7080141a80f0041a4130041b52300418e0e4141981300419a1f01438c4c00419e1a4141ad170244bc714242ae3701428f3741408c0a0142a22641439047004192270140b90c0041be1d014193244141a62a0140a004";
        let v: Vec<u8> = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect();
        let mut mt = MinuteTime::new(0, "000001");
        mt.parse(v);
        assert_eq!(mt.result().len(), 195, "应完整解析 195 个点");
        let prices: Vec<f64> = mt.result().iter().map(|d| d.price).collect();
        let max = prices.iter().cloned().fold(0.0, f64::max);
        let min = prices.iter().cloned().fold(f64::INFINITY, f64::min);
        assert!(
            (9.0..=15.0).contains(&prices[0]) && (9.0..=15.0).contains(&max) && (9.0..=15.0).contains(&min),
            "价格应在平安银行当日合理区间: 首={:.2} 高={:.2} 低={:.2}",
            prices[0], max, min
        );
        // 价格序列自洽：所有点均为正且末点接近最新价（样本时刻 11.68 附近）
        assert!(prices.iter().all(|p| *p > 0.0));
        assert!((prices[prices.len() - 1] - 11.68).abs() < 0.10);
    }

    /// 2026-09-07 实测的沪市新协议响应（600000，195 点）——code 锚点同样成立。
    #[test]
    fn parse_new_protocol_shanghai() {
        let hex = "c3000000013630303030308c0e9c0e13131645a5a0be0ddc0e92ef5d09dfa12a4e93ff2fbfef2d00918e0300018d03bf114102b81882234203a444be080d08ae0e58b1e30142a601a0d802027083a301435bb37a02629c7043e00190c2030255ab94010142b7514247966603028345010aa55001088630430dbd8b010303ab27000bb8554106b13f0002831a0002a11c010489300106bf34430bbf784243b2574149b98f014148bd544258b7ad014150a85e41529e6c004da946004ea34e006786c6014259b18a0100479b230063acb8010049ae2d004a9437425794760258a18101004aa3460047b131004d9b550045ac224143a217004eb85e44c1018ef302014fbd4f0048ad2a0048852c41738e9f02014d8852014bb454004fa7750149814c4145b629004baf6a03478a500347bd8a014142a33c0043bf4b4141931e4141b51a4143b2300141880e4142a3200142b71d0043ad2f41419a140042a01e004288160041920d424ab76e0149ad6100459a3e0141a5090043bc2f4141b70d4143a32501408f0400409c050043922a41519bb80101428f1701448c334141af0e0143ae3141428b2401409e064142971c0041a9114143a7234141810d0142b81b41429d190043851c0041ac054144af29004184090142b2130041900e0045bd350043a6240143a421004289140041a40b0041a2074142a11e0141800a0041900b0042a7154144903100429d1800408a050041af070042b1190040a70300428d15424bb27600418c0e01439c240041801000439d2144718ef703024e837b0046be3b004a916b00439b20424682370143981d0043aa1841498c5401459f3500419d090144b9300044972f0143ac1e4145983c0042a01c426087cc0201458a364149aa5b4141bd0b41638ccf020048904b0044a625005898d7010041af0d0044a92a0143ac204145ab3001418d0c0041a20a0042871c0146a44441458e39004a816b4141910c00439c1f4142ab140143a02301418d084143a4250141b8090042aa120043bb2901418d120043af270041aa0a0041a20800408205004680500142ba1d4141820d0141880e4142ac1a0141a2080141b51741418b0e0141b0090041b80d0040ae0700418f184142b7210140b1064142a4220041951500418b19004181164140b00301408d074141a0160141b3110040a40400419b1a0041bd100140b0054140ae01";
        let v: Vec<u8> = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect();
        let mut mt = MinuteTime::new(1, "600000");
        mt.parse(v);
        assert_eq!(mt.result().len(), 195, "应完整解析 195 个点");
        let prices: Vec<f64> = mt.result().iter().map(|d| d.price).collect();
        assert!(prices.iter().all(|p| (8.0..=11.0).contains(p)));
    }

    /// code 锚点不匹配（伪造的 code 头）→ 不走新协议路径，返回空。
    #[test]
    fn parse_new_protocol_code_mismatch_returns_empty() {
        // 把 "000001" 换成 "999999"（0x39），数据区无法通过旧协议校验
        let mut v = vec![0x02, 0x00, 0x00, 0x00, 0x00];
        v.extend_from_slice(b"999999");
        v.extend_from_slice(&[0x84, 0x0e, 0x00, 0xa4, 0x01, 0x32, 0x00, 0x90, 0x01]);
        let mut mt = MinuteTime::new(0, "000001");
        mt.parse(v);
        assert!(mt.result().is_empty());
    }

    /// 正向穷举定位：头部 2 个 varint（+100, -9）+ 数据 2 点 6 个 varint
    /// （+900, 0, 100 / +50, 0, 80），恰好耗尽到末尾 → 应解出 2 点。
    #[test]
    fn locate_data_block_finds_unique_start() {
        // 头 11 字节 + 头部 varint [+100(0xe4 0x00), -9(0x49)] + 数据
        let mut v = vec![0u8; 11];
        v.extend_from_slice(&[0xe4, 0x00, 0x49]); // +100, -9
        v.extend_from_slice(&[0x84, 0x0e, 0x00, 0xa4, 0x01]); // +900, 0, 100
        v.extend_from_slice(&[0x32, 0x00, 0x90, 0x01]); // +50, 0, 80
        let data = super::locate_data_block(&v, 2).expect("应定位数据块");
        assert_eq!(data.len(), 2);
        assert_eq!(data[0].price, 9.0);
        assert_eq!(data[0].vol, 100);
        assert_eq!(data[1].price, 9.5);
        assert_eq!(data[1].vol, 80);
    }

    /// 数据块未耗尽到响应末尾（尾部残留 varint）→ 无解返回 None。
    #[test]
    fn locate_data_block_requires_full_consumption() {
        let mut v = vec![0u8; 11];
        v.extend_from_slice(&[0x84, 0x0e, 0x00, 0x64]); // 1 点: +900, 0, 100
        v.extend_from_slice(&[0x32]); // 残留 +50（凑不齐合法价格序列）
        assert!(super::locate_data_block(&v, 1).is_none());
    }

    /// 点数声明超出实际 varint 数 → None。
    #[test]
    fn locate_data_block_insufficient_vars() {
        let mut v = vec![0u8; 11];
        v.extend_from_slice(&[0x84, 0x0e, 0x00, 0xa4, 0x01]); // 仅 2 个 varint
        assert!(super::locate_data_block(&v, 5).is_none());
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
