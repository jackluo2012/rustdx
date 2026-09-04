use crate::bytes_helper::u16_from_le_bytes;
use crate::tcp::Tdx;

/// F10 公司资料目录条目。
#[derive(Debug, Clone, serde::Serialize)]
pub struct CompanyInfoCategoryItem {
    /// 栏目名称（GBK 解码，如 "最新提示"、"公司概况"）
    pub name: String,
    /// 服务器端文件名
    pub filename: String,
    /// 内容起始偏移
    pub start: u32,
    /// 内容长度
    pub length: u32,
}

/// 获取 F10 公司资料的栏目目录。对应于 pytdx 的 hq.get_company_info_category。
///
/// ## 示例
/// ```ignore
/// use rustdx_complete::tcp::{Tcp, Tdx};
/// use rustdx_complete::tcp::stock::CompanyInfoCategory;
///
/// let mut tcp = Tcp::new()?;
/// let mut cat = CompanyInfoCategory::new(0, "000001");
/// cat.recv_parsed(&mut tcp)?;
/// for item in cat.result() {
///     println!("{}: offset={}, len={}", item.name, item.start, item.length);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct CompanyInfoCategory<'d> {
    pub send: Box<[u8]>,
    pub market: u16,
    pub code: &'d str,
    pub response: Vec<u8>,
    pub data: Vec<CompanyInfoCategoryItem>,
}

impl<'d> CompanyInfoCategory<'d> {
    pub fn new(market: u16, code: &'d str) -> Self {
        assert_eq!(code.len(), 6, "股票代码必须是6位");
        let mut send = [0u8; Self::LEN];
        // SEND 为完整 24 字节（含默认占位），全量复制后覆盖参数字段
        send.copy_from_slice(Self::SEND);
        send[12..14].copy_from_slice(&market.to_le_bytes());
        send[14..20].copy_from_slice(code.as_bytes());
        // 20..24 为 u32 0，SEND 中已是 0
        Self {
            send: send.into(),
            market,
            code,
            response: Vec::new(),
            data: Vec::new(),
        }
    }
}

impl<'a> Tdx for CompanyInfoCategory<'a> {
    type Item = [CompanyInfoCategoryItem];

    /// 包头 12 字节 + market(u16) + code(6) + 0(u32) = 24 字节。
    const SEND: &'static [u8] = &[
        0x0c, 0x0f, 0x10, 0x9b, 0x00, 0x01, 0x0e, 0x00, 0x0e, 0x00, 0xcf, 0x02, 0x00, 0x00, 0x30,
        0x30, 0x30, 0x30, 0x30, 0x31, 0x00, 0x00, 0x00, 0x00,
    ];
    const TAG: &'static str = "F10栏目目录";
    const LEN: usize = 12 + 2 + 6 + 4;

    fn send(&mut self) -> &[u8] {
        &self.send
    }

    /// 响应：数量(2) + 每条 152 字节 [name(64, GBK) + filename(80) + start(u32) + length(u32)]。
    fn parse(&mut self, v: Vec<u8>) {
        self.data = Vec::new();
        if v.len() < 2 {
            self.response = v;
            return;
        }
        let num = u16_from_le_bytes(&v, 0) as usize;
        let mut pos = 2;
        for _ in 0..num {
            if pos + 152 > v.len() {
                break;
            }
            let name_raw = &v[pos..pos + 64];
            let file_raw = &v[pos + 64..pos + 144];
            let start = u32::from_le_bytes(v[pos + 144..pos + 148].try_into().unwrap());
            let length = u32::from_le_bytes(v[pos + 148..pos + 152].try_into().unwrap());
            pos += 152;
            self.data.push(CompanyInfoCategoryItem {
                name: crate::tcp::helper::gbk_to_string_trim_null(name_raw),
                filename: crate::tcp::helper::gbk_to_string_trim_null(file_raw),
                start,
                length,
            });
        }
        self.response = v;
    }

    fn result(&self) -> &Self::Item {
        &self.data
    }
}

/// 获取 F10 公司资料某栏目的内容。对应于 pytdx 的 hq.get_company_info_content。
///
/// 通常先查 [`CompanyInfoCategory`] 拿到各栏目的 filename/start/length，
/// 再用本请求读取内容。
#[derive(Debug, Clone)]
pub struct CompanyInfoContent<'d> {
    pub send: Box<[u8]>,
    pub market: u16,
    pub code: &'d str,
    pub filename: &'d str,
    pub start: u32,
    pub length: u32,
    pub response: Vec<u8>,
    /// 栏目内容（GBK 解码为 UTF-8）
    pub data: String,
}

impl<'d> CompanyInfoContent<'d> {
    pub fn new(market: u16, code: &'d str, filename: &'d str, start: u32, length: u32) -> Self {
        assert_eq!(code.len(), 6, "股票代码必须是6位");
        assert!(filename.len() <= 80, "文件名不能超过 80 字节");
        let mut send = [0u8; Self::LEN];
        // SEND 只包含前 22 字节的包头 + 默认 market/code 占位
        send[0..22].copy_from_slice(Self::SEND);
        send[12..14].copy_from_slice(&market.to_le_bytes());
        send[14..20].copy_from_slice(code.as_bytes());
        // 20..22: u16 0（SEND 已为 0）
        // 22..102: filename（80 字节 \0 填充）
        send[22..22 + filename.len()].copy_from_slice(filename.as_bytes());
        send[102..106].copy_from_slice(&start.to_le_bytes());
        send[106..110].copy_from_slice(&length.to_le_bytes());
        // 110..114: u32 0（SEND 已为 0）
        Self {
            send: send.into(),
            market,
            code,
            filename,
            start,
            length,
            response: Vec::new(),
            data: String::new(),
        }
    }
}

impl<'a> Tdx for CompanyInfoContent<'a> {
    type Item = str;

    /// 包头 12 字节 + market(2) + code(6) + 0(2) + filename(80) + start(4) + length(4) + 0(4)
    /// = 114 字节。
    const SEND: &'static [u8] = &[
        0x0c, 0x07, 0x10, 0x9c, 0x00, 0x01, 0x68, 0x00, 0x68, 0x00, 0xd0, 0x02, 0x00, 0x00, 0x30,
        0x30, 0x30, 0x30, 0x30, 0x31, 0x00, 0x00,
    ];
    const TAG: &'static str = "F10栏目内容";
    const LEN: usize = 12 + 2 + 6 + 2 + 80 + 4 + 4 + 4;

    fn send(&mut self) -> &[u8] {
        &self.send
    }

    /// 响应：跳过 10 字节 + length(u16) + 内容（GBK）。
    fn parse(&mut self, v: Vec<u8>) {
        self.data = String::new();
        if v.len() >= 12 {
            let length = u16_from_le_bytes(&v, 10) as usize;
            if v.len() >= 12 + length {
                self.data = crate::tcp::helper::gbk_to_string(&v[12..12 + length]);
            }
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
    fn test_category_new() {
        let cat = CompanyInfoCategory::new(0, "000001");
        assert_eq!(cat.send.len(), 24);
        assert_eq!(&cat.send[0..12], &CompanyInfoCategory::SEND[0..12]);
        assert_eq!(&cat.send[12..14], &0u16.to_le_bytes());
        assert_eq!(&cat.send[14..20], b"000001");
    }

    #[test]
    fn test_content_new() {
        let content = CompanyInfoContent::new(0, "000001", "gsgk.txt", 0, 1000);
        assert_eq!(content.send.len(), 114);
        assert_eq!(content.send[22], b'g');
        assert_eq!(&content.send[102..106], &0u32.to_le_bytes());
        assert_eq!(&content.send[106..110], &1000u32.to_le_bytes());
    }

    /// 目录响应解析：1 条栏目。
    #[test]
    fn parse_category() {
        let mut v = [1u16.to_le_bytes().to_vec(), vec![0u8; 152]].concat();
        // name = "公司概况"（GBK: b9 ab cb be b8 c5 bf f6）
        v[2..10].copy_from_slice(&[0xb9, 0xab, 0xcb, 0xbe, 0xb8, 0xc5, 0xbf, 0xf6]);
        // filename = "gsgk.txt"
        v[66..74].copy_from_slice(b"gsgk.txt");
        // start = 0x100, length = 0x200
        v[146..150].copy_from_slice(&0x100u32.to_le_bytes());
        v[150..154].copy_from_slice(&0x200u32.to_le_bytes());

        let mut cat = CompanyInfoCategory::new(0, "000001");
        cat.parse(v);
        assert_eq!(cat.result().len(), 1);
        assert_eq!(cat.result()[0].name, "公司概况");
        assert_eq!(cat.result()[0].filename, "gsgk.txt");
        assert_eq!(cat.result()[0].start, 0x100);
        assert_eq!(cat.result()[0].length, 0x200);
    }
}
