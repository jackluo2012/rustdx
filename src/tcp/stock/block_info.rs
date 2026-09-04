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
pub fn get_block_info(tcp: &mut crate::tcp::Tcp, block_file: &str) -> crate::Result<Vec<BlockRecord>> {
    // 1. 文件大小
    let mut meta = BlockInfoMeta::new(block_file);
    meta.recv_parsed(tcp)?;
    let size = *meta.result();
    if size == 0 {
        return Ok(Vec::new());
    }

    // 2. 分块下载（pytdx：单块 0x7530 字节）
    const ONE_CHUNK: u32 = 0x7530;
    let chunks = size.div_ceil(ONE_CHUNK);
    let mut content = Vec::with_capacity(size as usize);
    for seg in 0..chunks {
        let start = seg * ONE_CHUNK;
        let take = ONE_CHUNK.min(size - start);
        let mut chunk = BlockInfoChunk::new(block_file, start, take);
        chunk.recv_parsed(tcp)?;
        content.extend_from_slice(chunk.result());
    }

    // 3. 解析记录：跳过 384 字节文件头 + 数量(2)，
    //    每板块 = 名称(9, GBK) + 数量(u16) + 类型(u16) + 数量×代码(7)
    Ok(parse_block_content(&content))
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
}
