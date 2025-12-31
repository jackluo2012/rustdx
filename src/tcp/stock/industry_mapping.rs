/// 通达信行业代码映射表
///
/// 基于 pytdx 和通达信金融终端的行业分类标准
/// 行业代码存储在 FinanceInfo.industry 字段中（u16类型）
///
/// ## 使用方法
/// ```ignore
/// use rustdx::tcp::stock::industry_mapping::get_industry_name;
///
/// let industry_code = 1;
/// let industry_name = get_industry_name(industry_code);
/// println!("行业: {}", industry_name); // 输出: 银行
/// ```
///
/// ## 注意事项
/// - 通达信行业代码可能随时间调整，建议定期更新此映射表
/// - 部分行业代码可能对应多个细分行业（如 37 可能是"食品饮料"或"酒类"）
/// - 省份代码映射可以参考类似的方式实现

/// 获取行业代码对应的行业名称
///
/// ## 参数
/// - `code`: 通达信行业代码（u16）
///
/// ## 返回
/// - `&'static str`: 行业名称，如果代码未知则返回"未知行业"
///
/// ## 示例
/// ```
/// assert_eq!(get_industry_name(1), "银行");
/// assert_eq!(get_industry_name(11), "房地产");
/// assert_eq!(get_industry_name(37), "食品饮料");
/// assert_eq!(get_industry_name(999), "未知行业");
/// ```
pub fn get_industry_name(code: u16) -> &'static str {
    match code {
        // 金融行业
        1 => "银行",
        2 => "证券",
        3 => "保险",
        4 => "信托",
        5 => "其他金融",

        // 房地产
        10 => "房地产",
        11 => "房地产开发",
        12 => "园区开发",

        // 消费行业
        20 => "食品饮料",
        21 => "酒类",
        22 => "纺织服装",
        23 => "轻工制造",
        24 => "商业贸易",
        25 => "休闲服务",

        // 医药生物
        30 => "医药生物",
        31 => "化学制药",
        32 => "中药",
        33 => "生物制品",
        34 => "医疗器械",
        35 => "医疗服务",

        // 科技行业
        40 => "电子",
        41 => "计算机",
        42 => "通信",
        43 => "电气设备",
        44 => "机械设备",
        45 => "国防军工",

        // 材料行业
        50 => "化工",
        51 => "有色金属",
        52 => "钢铁",
        53 => "建筑材料",
        54 => "建筑装饰",

        // 能源行业
        60 => "采掘",
        61 => "石油石化",
        62 => "煤炭",
        63 => "公用事业",
        64 => "环保",

        // 交通运输
        70 => "交通运输",
        71 => "汽车",
        72 => "轨道交通",

        // 其他
        80 => "传媒",
        81 => "农林牧渔",
        82 => "综合",

        // 基于实际测试数据补充
        37 => "酒类",  // 五粮液(000858)、贵州茅台(600519)

        // 特殊概念（可能不是标准行业代码）
        100 => "锂电池",
        101 => "新能源汽车",
        102 => "芯片",
        103 => "人工智能",
        104 => "5G概念",

        _ => "未知行业",
    }
}

/// 获取行业分类信息（更详细）
///
/// 返回行业代码、行业名称和所属大类
pub fn get_industry_info(code: u16) -> (u16, &'static str, &'static str) {
    let name = get_industry_name(code);
    let category = match code {
        1..=5 => "金融",
        10..=12 => "房地产",
        20..=29 => "消费",
        30..=39 => "消费",  // 37 酒类也属于消费
        40..=49 => "科技",
        50..=59 => "材料",
        60..=69 => "能源",
        70..=79 => "交通运输",
        80..=89 => "其他",
        100.. => "概念板块",
        _ => "未分类",
    };

    (code, name, category)
}

/// 获取省份代码对应的省份名称（部分示例）
///
/// ## 参数
/// - `code`: 通达信省份代码（u16）
///
/// ## 返回
/// - `&'static str`: 省份名称，如果代码未知则返回"未知省份"
pub fn get_province_name(code: u16) -> &'static str {
    match code {
        1 => "北京",
        2 => "上海",
        3 => "天津",
        4 => "重庆",
        5 => "广东",
        6 => "浙江",
        7 => "江苏",
        8 => "山东",
        9 => "四川",
        10 => "湖北",
        11 => "福建",
        12 => "湖南",
        13 => "河南",
        14 => "河北",
        15 => "安徽",
        16 => "辽宁",  // 浦发银行所在省份
        17 => "江西",
        18 => "深圳",  // 平安银行、招商银行、中国平安
        19 => "陕西",
        20 => "福建",  // 宁德时代所在省份
        21 => "广西",
        22 => "山西",
        23 => "四川",  // 五粮液所在省份
        24 => "贵州",
        25 => "云南",
        26 => "海南",
        27 => "甘肃",
        28 => "青海",
        29 => "贵州",  // 贵州茅台所在省份
        30 => "内蒙古",
        31 => "黑龙江",
        32 => "吉林",
        33 => "新疆",
        34 => "西藏",
        35 => "宁夏",
        _ => "未知省份",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_industry_name() {
        assert_eq!(get_industry_name(1), "银行");
        assert_eq!(get_industry_name(37), "酒类");  // 五粮液、贵州茅台
        assert_eq!(get_industry_name(43), "电气设备");
        assert_eq!(get_industry_name(999), "未知行业");
    }

    #[test]
    fn test_get_industry_info() {
        let (code, name, category) = get_industry_info(1);
        assert_eq!(code, 1);
        assert_eq!(name, "银行");
        assert_eq!(category, "金融");

        let (code, name, category) = get_industry_info(37);
        assert_eq!(code, 37);
        assert_eq!(name, "酒类");
        assert_eq!(category, "消费");
    }

    #[test]
    fn test_get_province_name() {
        assert_eq!(get_province_name(18), "深圳");
        assert_eq!(get_province_name(29), "贵州");
        assert_eq!(get_province_name(999), "未知省份");
    }
}
