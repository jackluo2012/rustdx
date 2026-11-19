use crate::bytes_helper::u16_from_le_bytes;
use crate::tcp::Tdx;

/// 单条板块成分记录。
#[derive(Debug, Clone, serde::Serialize)]
pub struct BlockRecord {
    /// 板块名称（GBK 解码，如 "锂电池"、"沪深300"）
    pub blockname: String,
    /// 板块类型（0=板块 1=风格 等，随文件类型而异）
    pub block_type: u16,
    /// 成分股代码（6位）
    pub code: String,
}

/// 板块文件的单块下载请求（内部使用）。
///
/// 通达信服务器上保存着板块定义文件（`block_zs.dat` 指数板块、
/// `block_gn.dat` 概念板块、`block_fg.dat` 风格板块、`block.dat` 通用），
/// 通过两步协议下载：先 [`BlockInfoMeta`] 拿文件大小，再分块下载内容。
///
/// 高层 API 见 [`get_block_info`]。
#[derive(Debug, Clone)]
pub struct BlockInfoMeta {
    pub send: Box<[u8]>,
    pub response: Vec<u8>,
    /// 文件大小（字节）
    pub size: u32,
}

impl BlockInfoMeta {
    /// 查询板块文件大小。
    pub fn new(block_file: &str) -> Self {
        let mut send = [0u8; Self::LEN];
        send[0..12].copy_from_slice(Self::SEND);
        // 文件名（40 字节，\0 填充）
        let name = block_file.as_bytes();
        let n = name.len().min(40);
        send[12..12 + n].copy_from_slice(&name[..n]);
        Self {
            send: send.into(),
            response: Vec::new(),
            size: 0,
        }
    }
}

impl Tdx for BlockInfoMeta {
    type Item = u32;

    /// 包头 12 字节 + 文件名 40 字节 = 52。
    const SEND: &'static [u8] = &[
        0x0c, 0x39, 0x18, 0x69, 0x00, 0x01, 0x2a, 0x00, 0x2a, 0x00, 0xc5, 0x02,
    ];
    const TAG: &'static str = "板块文件大小";
    const LEN: usize = 12 + 40;

    fn send(&mut self) -> &[u8] {
        &self.send
    }

    /// 响应：size(u32 LE) + hash(32字节) + 1。
    fn parse(&mut self, v: Vec<u8>) {
        if v.len() >= 4 {
            self.size = u32::from_le_bytes([v[0], v[1], v[2], v[3]]);
        }
        self.response = v;
    }

    fn result(&self) -> &Self::Item {
        &self.size
    }
}

/// 板块文件内容块下载请求（内部使用）。
#[derive(Debug, Clone)]
pub struct BlockInfoChunk {
    pub send: Box<[u8]>,
    pub response: Vec<u8>,
    /// 文件内容（跳过前 4 字节后的部分）
    pub data: Vec<u8>,
}

impl BlockInfoChunk {
    /// 下载板块文件的 [start, start+size) 字节段。
    pub fn new(block_file: &str, start: u32, size: u32) -> Self {
        let mut send = [0u8; Self::LEN];
        send[0..12].copy_from_slice(Self::SEND);
        send[12..16].copy_from_slice(&start.to_le_bytes());
        send[16..20].copy_from_slice(&size.to_le_bytes());
        let name = block_file.as_bytes();
        let n = name.len().min(100);
        send[20..20 + n].copy_from_slice(&name[..n]);
        Self {
            send: send.into(),
            response: Vec::new(),
            data: Vec::new(),
        }
    }
}

impl Tdx for BlockInfoChunk {
    type Item = [u8];

    /// 包头 12 字节 + start(4) + size(4) + 文件名(100) = 120。
    const SEND: &'static [u8] = &[
        0x0c, 0x37, 0x18, 0x6a, 0x00, 0x01, 0x6e, 0x00, 0x6e, 0x00, 0xb9, 0x06,
    ];
    const TAG: &'static str = "板块文件内容";
    const LEN: usize = 12 + 4 + 4 + 100;

    fn send(&mut self) -> &[u8] {
        &self.send
    }

    /// 响应：前 4 字节跳过，其余为文件内容。
    fn parse(&mut self, v: Vec<u8>) {
        self.data = v.iter().skip(4).copied().collect();
        self.response = v;
    }

    fn result(&self) -> &Self::Item {
        &self.data
    }
}

/// 下载并解析板块文件，返回全部板块成分记录。
///
/// **每个板块最多 400 只成分股**：这是 `block_*.dat` 文件格式的上限（固定槽位），
/// 超出的部分在文件里不存在，而且保留的是按代码排序的前 400 只——热门题材里
/// 沪市主板和科创板的股票几乎全被截掉。2026-11-19 实测 269 个概念板块中 28 个
/// 被截断（人工智能、芯片、低空经济、商业航天等），共丢失 5766 条成分。
/// 需要完整成分请用 [`get_blocks`]。
///
/// ## 参数
/// - `block_file`: 板块文件名。常用：
///   - `"block_gn.dat"` 概念板块
///   - `"block_zs.dat"` 指数板块
///   - `"block_fg.dat"` 风格板块
///   - `"block.dat"` 通用板块
///
/// ## 示例
/// ```ignore
/// use rustdx_complete::tcp::{Tcp, Tdx};
/// use rustdx_complete::tcp::stock::get_block_info;
///
/// let mut tcp = Tcp::new()?;
/// let records = get_block_info(&mut tcp, "block_gn.dat")?;
/// for r in records.iter().filter(|r| r.blockname == "锂电池") {
///     println!("{} 属于锂电池板块", r.code);
/// }
/// ```
pub fn get_block_info(
    tcp: &mut crate::tcp::Tcp,
    block_file: &str,
) -> crate::Result<Vec<BlockRecord>> {
    // 每板块 = 名称(9, GBK) + 数量(u16) + 类型(u16) + 数量×代码(7)，前有 384 字节文件头
    Ok(parse_block_content(&download_file(tcp, block_file)?))
}

/// 从行情服务器下载一个配置/板块文件的原始字节（两步协议：先取大小，再分块下载）。
///
/// 实测可取到的文件：`block_gn.dat` / `block_zs.dat` / `block_fg.dat` / `block.dat`、
/// `infoharbor_block.dat`（完整板块成分）、`tdxhy.cfg`（个股行业代码）、
/// `tdxzsbase.cfg`（指数统计）。服务器上没有的文件返回空。
pub fn download_file(tcp: &mut crate::tcp::Tcp, file: &str) -> crate::Result<Vec<u8>> {
    let mut meta = BlockInfoMeta::new(file);
    meta.recv_parsed(tcp)?;
    let size = *meta.result();

    // 分块下载（pytdx：单块 0x7530 字节）
    const ONE_CHUNK: u32 = 0x7530;
    let mut content = Vec::with_capacity(size as usize);
    for seg in 0..size.div_ceil(ONE_CHUNK) {
        let start = seg * ONE_CHUNK;
        let mut chunk = BlockInfoChunk::new(file, start, ONE_CHUNK.min(size - start));
        chunk.recv_parsed(tcp)?;
        content.extend_from_slice(chunk.result());
    }
    Ok(content)
}

/// 板块类别（`infoharbor_block.dat` 里的前缀）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
pub enum BlockKind {
    /// 概念板块（`GN_`）：题材。
    Concept,
    /// 风格板块（`FG_`）：如破净、微盘股、基金重仓。
    Style,
    /// 指数板块（`ZS_`）：如沪深300、创业板指的成分。
    Index,
}

/// 一个板块及其完整成分，见 [`get_blocks`]。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Block {
    /// 类别。
    pub kind: BlockKind,
    /// 板块名。**最长 4 个汉字**，更长的名字在数据源里就被截断了
    /// （如「人形机器人」记作「人形机器」）。
    pub name: String,
    /// 对应的板块指数代码（沪市 `880xxx`），可用它取板块指数的行情与 K 线；
    /// 个别板块没有。
    pub index_code: Option<String>,
    /// 板块创建日期（YYYYMMDD）。
    pub created: Option<u32>,
    /// 成分最近一次调整的日期（YYYYMMDD）。
    pub updated: Option<u32>,
    /// 成分股 `(market, code)`：market 0=深市、1=沪市、2=北交所。
    pub stocks: Vec<(u16, String)>,
}

/// 下载并解析完整的板块数据（`infoharbor_block.dat`）：概念、风格、指数三类。
///
/// 与 [`get_block_info`] 的区别：**成分不截断**（实测最大的板块 1238 只），带市场
/// 前缀（含北交所）、对应的板块指数代码与最近调整日期。2026-11-19 实测 547 个
/// 板块（概念 269 / 风格 161 / 指数 117），最近的调整日期是前一个交易日。
///
/// 文件约 760KB，成分一天最多调整一次，建议每天取一次后自行缓存。
///
/// ## 示例
/// ```ignore
/// use rustdx_complete::tcp::stock::{BlockKind, Client};
///
/// let mut client = Client::new()?;
/// let blocks = client.blocks()?;
/// let ai = blocks.iter().find(|b| b.kind == BlockKind::Concept && b.name == "人工智能").unwrap();
/// println!("{} 只成分股，板块指数 {:?}，最近调整 {:?}", ai.stocks.len(), ai.index_code, ai.updated);
/// ```
pub fn get_blocks(tcp: &mut crate::tcp::Tcp) -> crate::Result<Vec<Block>> {
    Ok(parse_blocks(&download_file(tcp, "infoharbor_block.dat")?)?)
}

/// 解析 `infoharbor_block.dat` 的内容（GBK 文本）。
///
/// 格式：板块头一行 `#GN_板块名,成分数,指数代码,创建日期,调整日期,,`，其后若干行
/// `市场#代码,` 列表，直到下一个板块头。
///
/// 任一板块的实际成分数与头部声明不符时返回 [`std::io::ErrorKind::InvalidData`]：
/// 这通常意味着文件下载不完整，不能把残缺的成分当成真的用。
pub fn parse_blocks(data: &[u8]) -> std::io::Result<Vec<Block>> {
    let invalid = |msg: String| std::io::Error::new(std::io::ErrorKind::InvalidData, msg);
    let text = crate::tcp::helper::gbk_to_string(data);
    let mut blocks: Vec<(Block, Option<usize>)> = Vec::new();
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if let Some(head) = line.strip_prefix('#') {
            let mut fields = head.split(',').map(str::trim);
            let tag = fields.next().unwrap_or_default();
            let (kind, name) = match tag.split_once('_') {
                Some(("GN", name)) => (BlockKind::Concept, name),
                Some(("FG", name)) => (BlockKind::Style, name),
                Some(("ZS", name)) => (BlockKind::Index, name),
                _ => return Err(invalid(format!("无法识别的板块头: {line}"))),
            };
            let declared = fields.next().and_then(|s| s.parse().ok());
            let index_code = fields.next().filter(|s| !s.is_empty()).map(str::to_owned);
            let mut date = || {
                fields
                    .next()
                    .and_then(|s| s.parse().ok())
                    .filter(|d| *d > 0)
            };
            let (created, updated) = (date(), date());
            let block = Block {
                kind,
                name: name.to_owned(),
                index_code,
                created,
                updated,
                stocks: Vec::with_capacity(declared.unwrap_or(0)),
            };
            blocks.push((block, declared));
        } else {
            let Some((block, _)) = blocks.last_mut() else {
                return Err(invalid(format!("成分行出现在板块头之前: {line}")));
            };
            for item in line.split(',').filter(|s| !s.is_empty()) {
                let stock = item
                    .split_once('#')
                    .and_then(|(m, code)| Some((m.parse().ok()?, code.to_owned())))
                    .ok_or_else(|| invalid(format!("无法识别的成分: {item}")))?;
                block.stocks.push(stock);
            }
        }
    }
    blocks
        .into_iter()
        .map(|(block, declared)| match declared {
            Some(n) if n != block.stocks.len() => Err(invalid(format!(
                "板块「{}」声明 {n} 只成分，实际 {} 只（文件可能不完整）",
                block.name,
                block.stocks.len()
            ))),
            _ => Ok(block),
        })
        .collect()
}

/// 解析板块文件内容。
///
/// 文件布局（2026 实测）：384 字节文件头（"Registry ver: 1.0 ..."）+ 板块数(u16)，
/// 之后为 **固定大小的板块槽位**（`槽位大小 = (总长 - 386) / 板块数`，
/// block_gn.dat 为 2813 字节 = 板块头 13 + 最多 400 只代码 × 7）。
/// 每个槽位内：板块名(9, GBK) + 成分数(u16) + 类型(u16) + 成分数×代码(7)，
/// 剩余部分以 `\0` 填充。
pub fn parse_block_content(data: &[u8]) -> Vec<BlockRecord> {
    let mut result = Vec::new();
    if data.len() < 386 {
        return result;
    }
    let num = u16_from_le_bytes(data, 384) as usize;
    if num == 0 {
        return result;
    }
    // 固定槽位大小（总长 - 头部 / 板块数）
    let slot = (data.len() - 386).div_ceil(num);

    for i in 0..num {
        let mut pos = 386 + i * slot;
        if pos + 13 > data.len() {
            break;
        }
        let name_raw = &data[pos..pos + 9];
        let blockname = crate::tcp::helper::gbk_to_string_trim_null(name_raw);
        let stock_count = u16_from_le_bytes(data, pos + 9) as usize;
        let block_type = u16_from_le_bytes(data, pos + 11);
        pos += 13;
        if blockname.is_empty() || stock_count == 0 {
            continue; // 空槽位
        }
        for _ in 0..stock_count {
            if pos + 7 > data.len() {
                break;
            }
            // 7 字节 = 6 位代码 + 1 字节（市场/填充）
            let code_raw = &data[pos..pos + 6];
            pos += 7;
            let code: String = code_raw
                .iter()
                .map(|&b| b as char)
                .filter(|c| c.is_ascii_digit())
                .collect();
            if code.len() == 6 {
                result.push(BlockRecord {
                    blockname: blockname.clone(),
                    block_type,
                    code,
                });
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造的板块文件内容：384 字节头 + 数量 + 2 个固定槽位（槽位间填充）。
    #[test]
    fn parse_block_content_two_blocks() {
        // 槽位大小 = 13 + 2×7 = 27（2 个代码位）
        let slot: usize = 13 + 14;
        let mut data = vec![0u8; 384];
        data.extend_from_slice(&2u16.to_le_bytes()); // num = 2

        // 槽位0: "锂电池"（GBK: ef ae b5 e7 b3 d8）2 只成分股 类型1
        let mut b0 = vec![0u8; slot];
        b0[..6].copy_from_slice(&[0xef, 0xae, 0xb5, 0xe7, 0xb3, 0xd8]);
        b0[9..11].copy_from_slice(&2u16.to_le_bytes());
        b0[11..13].copy_from_slice(&1u16.to_le_bytes());
        b0[13..19].copy_from_slice(b"300750");
        b0[20..26].copy_from_slice(b"600519");
        data.extend_from_slice(&b0);

        // 槽位1: "HS300" 1 只成分股 类型0
        let mut b1 = vec![0u8; slot];
        b1[..5].copy_from_slice(b"HS300");
        b1[9..11].copy_from_slice(&1u16.to_le_bytes());
        b1[11..13].copy_from_slice(&0u16.to_le_bytes());
        b1[13..19].copy_from_slice(b"000001");
        data.extend_from_slice(&b1);

        let records = parse_block_content(&data);
        assert_eq!(records.len(), 3);
        assert_eq!(records[0].blockname, "锂电池");
        assert_eq!(records[0].code, "300750");
        assert_eq!(records[0].block_type, 1);
        assert_eq!(records[1].blockname, "锂电池");
        assert_eq!(records[1].code, "600519");
        assert_eq!(records[2].blockname, "HS300");
        assert_eq!(records[2].code, "000001");
    }

    #[test]
    fn parse_truncated_returns_empty() {
        assert!(parse_block_content(&[0u8; 100]).is_empty());
    }

    #[test]
    fn test_meta_send_bytes() {
        let meta = BlockInfoMeta::new("block_gn.dat");
        assert_eq!(meta.send.len(), 52);
        assert_eq!(&meta.send[12..24], b"block_gn.dat");
        assert_eq!(&meta.send[24..52], &[0u8; 28]);
    }

    /// 完整板块文件的解析：类别、指数代码、日期、带市场前缀的成分、跨行成分。
    #[test]
    fn parse_blocks_text() {
        let text = "#GN_人工智能,5,880948,20170314,20261103,,\r\n0#000032,0#000034,\r\n1#600000,1#688981,2#920799,\r\n\
                    #FG_破净资产,0,,20050607,,,\r\n\
                    #ZS_上证50,1,880529,20050607,20260512,,\r\n1#600519,\r\n";
        let (gbk, _, _) = encoding_rs::GBK.encode(text);
        let blocks = super::parse_blocks(&gbk).unwrap();
        assert_eq!(blocks.len(), 3);
        let ai = &blocks[0];
        assert_eq!(
            (ai.kind, ai.name.as_str()),
            (super::BlockKind::Concept, "人工智能")
        );
        assert_eq!(ai.index_code.as_deref(), Some("880948"));
        assert_eq!((ai.created, ai.updated), (Some(20170314), Some(20261103)));
        assert_eq!(
            ai.stocks,
            [
                (0, "000032"),
                (0, "000034"),
                (1, "600000"),
                (1, "688981"),
                (2, "920799")
            ]
            .map(|(m, c)| (m, c.to_string()))
        );
        // 没有指数代码、没有调整日期、没有成分的板块
        let empty = &blocks[1];
        assert_eq!(empty.kind, super::BlockKind::Style);
        assert_eq!(
            (empty.index_code.clone(), empty.updated, empty.stocks.len()),
            (None, None, 0)
        );
        assert_eq!(blocks[2].kind, super::BlockKind::Index);
    }

    /// 成分数与头部声明不符（下载不完整）、无法识别的内容，都是错误。
    #[test]
    fn parse_blocks_rejects_incomplete_or_malformed() {
        for text in [
            "#GN_芯片,3,880952,20170907,20261118,,\n0#000021,0#000063,\n", // 少一只
            "#XX_未知,1,,,,\n0#000001,\n",                                 // 未知类别
            "0#000001,\n",                                                 // 没有板块头
            "#GN_芯片,1,880952,,,,\nabc,\n",                               // 成分格式不对
        ] {
            let (gbk, _, _) = encoding_rs::GBK.encode(text);
            let err = super::parse_blocks(&gbk).unwrap_err();
            assert_eq!(err.kind(), std::io::ErrorKind::InvalidData, "{text}");
        }
        assert!(super::parse_blocks(b"").unwrap().is_empty());
    }
}
