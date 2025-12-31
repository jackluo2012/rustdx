/// 东方财富概念板块成分股映射表
///
/// 数据来源: AKShare (东方财富概念板块)
/// 更新时间: 手动更新
///
/// 使用方法:
/// ```ignore
/// use rustdx_complete::tcp::stock::get_concept_stocks;
///
/// // 获取"新能源汽车"概念的成分股
/// if let Some(stocks) = get_concept_stocks("新能源汽车") {
///     for stock in stocks {
///         println!("{}: {}", stock.code, stock.name);
///     }
/// }
/// ```


/// 概念股信息
#[derive(Debug, Clone)]
pub struct ConceptStock {
    pub code: &'static str,
    pub name: &'static str,
}

/// 同花顺/东方财富概念板块成分股映射（示例数据）
/// 
/// 返回指定概念的成分股列表（前20只）
pub fn get_concept_stocks(concept: &str) -> Option<Vec<ConceptStock>> {
    match concept {
        "新能源汽车" => Some(vec![
            ConceptStock { code: "603278", name: "大业股份" },
            ConceptStock { code: "002634", name: "棒杰股份" },
            ConceptStock { code: "600416", name: "湘电股份" },
            ConceptStock { code: "688196", name: "卓越新能" },
            ConceptStock { code: "002342", name: "巨力索具" },
            ConceptStock { code: "002163", name: "海南发展" },
            ConceptStock { code: "300091", name: "金通灵" },
            ConceptStock { code: "688660", name: "电气风电" },
            ConceptStock { code: "300129", name: "泰胜风能" },
            ConceptStock { code: "000551", name: "创元科技" },
            ConceptStock { code: "300670", name: "大烨智能" },
            ConceptStock { code: "300084", name: "海默科技" },
            ConceptStock { code: "600273", name: "嘉化能源" },
            ConceptStock { code: "603507", name: "振江股份" },
            ConceptStock { code: "002897", name: "意华股份" },
            ConceptStock { code: "000065", name: "北方国际" },
            ConceptStock { code: "002529", name: "*ST海源" },
            ConceptStock { code: "300125", name: "*ST聆达" },
            ConceptStock { code: "002366", name: "融发核电" },
            ConceptStock { code: "601727", name: "上海电气" },
        ]),
        "锂电池" => Some(vec![
            ConceptStock { code: "002757", name: "南兴股份" },
            ConceptStock { code: "300921", name: "南凌科技" },
            ConceptStock { code: "002212", name: "天融信" },
            ConceptStock { code: "688030", name: "山石网科" },
            ConceptStock { code: "300454", name: "深信服" },
            ConceptStock { code: "300494", name: "盛天网络" },
            ConceptStock { code: "002467", name: "二六三" },
            ConceptStock { code: "300352", name: "北信源" },
            ConceptStock { code: "300383", name: "光环新网" },
            ConceptStock { code: "600602", name: "云赛智联" },
            ConceptStock { code: "688168", name: "安博通" },
            ConceptStock { code: "300768", name: "迪普科技" },
            ConceptStock { code: "603019", name: "中科曙光" },
            ConceptStock { code: "688158", name: "优刻得-W" },
            ConceptStock { code: "002238", name: "天威视讯" },
            ConceptStock { code: "002439", name: "启明星辰" },
            ConceptStock { code: "300311", name: "ST任子行" },
            ConceptStock { code: "002268", name: "电科网安" },
            ConceptStock { code: "601789", name: "宁波建工" },
        ]),
        "芯片" => Some(vec![
            ConceptStock { code: "603726", name: "朗迪集团" },
            ConceptStock { code: "603868", name: "飞科电器" },
            ConceptStock { code: "603215", name: "比依股份" },
            ConceptStock { code: "002050", name: "三花智控" },
            ConceptStock { code: "002848", name: "*ST高斯" },
            ConceptStock { code: "300342", name: "天银机电" },
            ConceptStock { code: "300894", name: "火星人" },
            ConceptStock { code: "002860", name: "星帅尔" },
            ConceptStock { code: "605336", name: "帅丰电器" },
            ConceptStock { code: "603219", name: "富佳股份" },
            ConceptStock { code: "600619", name: "海立股份" },
            ConceptStock { code: "605365", name: "立达信" },
            ConceptStock { code: "688696", name: "极米科技" },
            ConceptStock { code: "000521", name: "长虹美菱" },
            ConceptStock { code: "603515", name: "欧普照明" },
            ConceptStock { code: "603579", name: "荣泰健康" },
            ConceptStock { code: "688475", name: "萤石网络" },
            ConceptStock { code: "920768", name: "拾比佰" },
            ConceptStock { code: "603578", name: "三星新材" },
            ConceptStock { code: "002705", name: "新宝股份" },
        ]),
        "军民融合" => Some(vec![
            ConceptStock { code: "300516", name: "久之洋" },
            ConceptStock { code: "688523", name: "航天环宇" },
            ConceptStock { code: "688272", name: "富吉瑞" },
            ConceptStock { code: "300696", name: "爱乐达" },
            ConceptStock { code: "300900", name: "广联航空" },
            ConceptStock { code: "600879", name: "航天电子" },
            ConceptStock { code: "600783", name: "鲁信创投" },
            ConceptStock { code: "600118", name: "中国卫星" },
            ConceptStock { code: "003009", name: "中天火箭" },
            ConceptStock { code: "000547", name: "航天发展" },
            ConceptStock { code: "002151", name: "北斗星通" },
            ConceptStock { code: "002413", name: "雷科防务" },
            ConceptStock { code: "002361", name: "神剑股份" },
            ConceptStock { code: "300762", name: "上海瀚讯" },
            ConceptStock { code: "600416", name: "湘电股份" },
            ConceptStock { code: "002683", name: "广东宏大" },
            ConceptStock { code: "002342", name: "巨力索具" },
            ConceptStock { code: "002471", name: "中超控股" },
            ConceptStock { code: "300091", name: "金通灵" },
            ConceptStock { code: "688011", name: "新光光电" },
        ]),
        "北斗导航" => Some(vec![
            ConceptStock { code: "300516", name: "久之洋" },
            ConceptStock { code: "688568", name: "中科星图" },
            ConceptStock { code: "601698", name: "中国卫通" },
            ConceptStock { code: "600879", name: "航天电子" },
            ConceptStock { code: "600118", name: "中国卫星" },
            ConceptStock { code: "002766", name: "索菱股份" },
            ConceptStock { code: "002151", name: "北斗星通" },
            ConceptStock { code: "600345", name: "长江通信" },
            ConceptStock { code: "002413", name: "雷科防务" },
            ConceptStock { code: "002361", name: "神剑股份" },
            ConceptStock { code: "301517", name: "陕西华达" },
            ConceptStock { code: "000901", name: "航天科技" },
            ConceptStock { code: "600198", name: "大唐电信" },
            ConceptStock { code: "603712", name: "七一二" },
            ConceptStock { code: "920116", name: "星图测控" },
            ConceptStock { code: "002935", name: "天奥电子" },
            ConceptStock { code: "688066", name: "航天宏图" },
            ConceptStock { code: "002465", name: "海格通信" },
            ConceptStock { code: "688311", name: "盟升电子" },
            ConceptStock { code: "300455", name: "航天智装" },
        ]),
        "AIGC概念" => Some(vec![
            ConceptStock { code: "300058", name: "蓝色光标" },
            ConceptStock { code: "301066", name: "万事利" },
            ConceptStock { code: "300071", name: "福石控股" },
            ConceptStock { code: "301171", name: "易点天下" },
            ConceptStock { code: "002131", name: "利欧股份" },
            ConceptStock { code: "300612", name: "宣亚国际" },
            ConceptStock { code: "300170", name: "汉得信息" },
            ConceptStock { code: "300785", name: "值得买" },
            ConceptStock { code: "688365", name: "光云科技" },
            ConceptStock { code: "300348", name: "长亮科技" },
            ConceptStock { code: "300654", name: "世纪天鸿" },
            ConceptStock { code: "600556", name: "天下秀" },
            ConceptStock { code: "300846", name: "首都在线" },
            ConceptStock { code: "300781", name: "因赛集团" },
            ConceptStock { code: "300364", name: "中文在线" },
            ConceptStock { code: "002400", name: "省广集团" },
            ConceptStock { code: "000681", name: "视觉中国" },
            ConceptStock { code: "300063", name: "天龙集团" },
            ConceptStock { code: "002607", name: "中公教育" },
            ConceptStock { code: "300592", name: "华凯易佰" },
        ]),
        "航母概念" => Some(vec![
            ConceptStock { code: "002151", name: "北斗星通" },
            ConceptStock { code: "600391", name: "航发科技" },
            ConceptStock { code: "600456", name: "宝钛股份" },
            ConceptStock { code: "002149", name: "西部材料" },
            ConceptStock { code: "002465", name: "海格通信" },
            ConceptStock { code: "002318", name: "久立特材" },
            ConceptStock { code: "600435", name: "北方导航" },
            ConceptStock { code: "300447", name: "全信股份" },
            ConceptStock { code: "002324", name: "普利特" },
            ConceptStock { code: "300101", name: "振芯科技" },
            ConceptStock { code: "600316", name: "洪都航空" },
            ConceptStock { code: "300252", name: "金信诺" },
            ConceptStock { code: "000738", name: "航发控制" },
            ConceptStock { code: "600990", name: "四创电子" },
            ConceptStock { code: "600372", name: "中航机载" },
            ConceptStock { code: "000768", name: "中航西飞" },
            ConceptStock { code: "600764", name: "中国海防" },
            ConceptStock { code: "000099", name: "中信海直" },
            ConceptStock { code: "600378", name: "昊华科技" },
            ConceptStock { code: "600184", name: "光电股份" },
        ]),
        "5G概念" => Some(vec![
            ConceptStock { code: "002718", name: "友邦吊顶" },
            ConceptStock { code: "300599", name: "雄塑科技" },
            ConceptStock { code: "300234", name: "开尔新材" },
            ConceptStock { code: "301227", name: "森鹰窗业" },
            ConceptStock { code: "603838", name: "*ST四通" },
            ConceptStock { code: "600076", name: "康欣新材" },
            ConceptStock { code: "002457", name: "青龙管业" },
            ConceptStock { code: "300737", name: "科顺股份" },
            ConceptStock { code: "002043", name: "兔 宝 宝" },
            ConceptStock { code: "603992", name: "松霖科技" },
            ConceptStock { code: "002798", name: "帝欧水华" },
            ConceptStock { code: "603818", name: "曲美家居" },
            ConceptStock { code: "603661", name: "恒林股份" },
            ConceptStock { code: "002572", name: "索菲亚" },
            ConceptStock { code: "300715", name: "凯伦股份" },
            ConceptStock { code: "300198", name: "ST纳川" },
            ConceptStock { code: "000786", name: "北新建材" },
            ConceptStock { code: "603378", name: "亚士创能" },
            ConceptStock { code: "603208", name: "江山欧派" },
            ConceptStock { code: "301429", name: "森泰股份" },
        ]),
        "数字货币" => Some(vec![
            ConceptStock { code: "300658", name: "延江股份" },
            ConceptStock { code: "300740", name: "水羊股份" },
            ConceptStock { code: "603238", name: "诺邦股份" },
            ConceptStock { code: "603605", name: "珀莱雅" },
            ConceptStock { code: "001328", name: "登康口腔" },
            ConceptStock { code: "300886", name: "华业香料" },
            ConceptStock { code: "300849", name: "锦盛新材" },
            ConceptStock { code: "603059", name: "倍加洁" },
            ConceptStock { code: "300896", name: "爱美客" },
            ConceptStock { code: "301009", name: "可靠股份" },
            ConceptStock { code: "300856", name: "科思股份" },
            ConceptStock { code: "300132", name: "青松股份" },
            ConceptStock { code: "603630", name: "拉芳家化" },
            ConceptStock { code: "603193", name: "润本股份" },
            ConceptStock { code: "605009", name: "豪悦护理" },
            ConceptStock { code: "600315", name: "上海家化" },
            ConceptStock { code: "688363", name: "华熙生物" },
            ConceptStock { code: "603983", name: "丸美生物" },
            ConceptStock { code: "002243", name: "力合科创" },
            ConceptStock { code: "301371", name: "敷尔佳" },
        ]),
        "人工智能" => Some(vec![
            ConceptStock { code: "600362", name: "江西铜业" },
            ConceptStock { code: "600895", name: "张江高科" },
            ConceptStock { code: "600115", name: "中国东航" },
            ConceptStock { code: "601888", name: "中国中免" },
            ConceptStock { code: "600029", name: "南方航空" },
            ConceptStock { code: "000417", name: "合百集团" },
            ConceptStock { code: "601111", name: "中国国航" },
            ConceptStock { code: "601021", name: "春秋航空" },
            ConceptStock { code: "601899", name: "紫金矿业" },
            ConceptStock { code: "002244", name: "滨江集团" },
            ConceptStock { code: "000685", name: "中山公用" },
            ConceptStock { code: "600435", name: "北方导航" },
            ConceptStock { code: "002153", name: "石基信息" },
            ConceptStock { code: "000425", name: "徐工机械" },
            ConceptStock { code: "601600", name: "中国铝业" },
            ConceptStock { code: "600636", name: "*ST国化" },
            ConceptStock { code: "600637", name: "东方明珠" },
            ConceptStock { code: "000997", name: "新 大 陆" },
            ConceptStock { code: "600757", name: "长江传媒" },
            ConceptStock { code: "000598", name: "兴蓉环境" },
        ]),
        _ => None,
    }
}

/// 获取所有支持的概念板块名称
pub fn get_concept_names() -> Vec<&'static str> {
    vec![
        "新能源汽车",
        "锂电池",
        "芯片",
        "军民融合",
        "北斗导航",
        "AIGC概念",
        "航母概念",
        "5G概念",
        "数字货币",
        "人工智能",
    ]
}

/// 获取概念板块信息
pub fn get_concept_info(concept: &str) -> Option<(&'static str, usize)> {
    match concept {
        "新能源汽车" => Some(("新能源汽车", 216)),
        "锂电池" => Some(("锂电池", 19)),
        "芯片" => Some(("芯片", 91)),
        "军民融合" => Some(("军民融合", 183)),
        "北斗导航" => Some(("北斗导航", 82)),
        "AIGC概念" => Some(("AIGC概念", 123)),
        "航母概念" => Some(("航母概念", 32)),
        "5G概念" => Some(("5G概念", 67)),
        "数字货币" => Some(("数字货币", 28)),
        "人工智能" => Some(("人工智能", 217)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_concept_stocks() {
        let stocks = get_concept_stocks("新能源汽车");
        assert!(stocks.is_some());
        let stocks = stocks.unwrap();
        assert!(!stocks.is_empty());
    }

    #[test]
    fn test_get_concept_names() {
        let names = get_concept_names();
        assert!(!names.is_empty());
    }
}
