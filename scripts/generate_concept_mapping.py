#!/usr/bin/env python3
"""
获取东方财富概念板块数据并生成Rust映射表

使用AKShare库获取东方财富概念板块数据（东方财富和同花顺概念类似）
生成可用于rustdx的代码
"""

import akshare as ak
import json
from pathlib import Path
import pandas as pd


def get_concept_stocks(concept_code):
    """获取指定概念板块的成分股"""
    try:
        df = ak.stock_board_concept_cons_em(symbol=concept_code)
        return df
    except Exception as e:
        print(f"⚠️  获取概念 '{concept_code}' 成分股失败: {e}")
        return None


def generate_mapping():
    """生成概念板块映射的Rust代码"""
    print("🚀 开始生成概念板块映射代码\n")

    # 定义热门概念板块（使用东方财富板块代码）
    hot_concepts = {
        "BK0493": "新能源汽车",
        "BK0885": "锂电池",
        "BK0456": "芯片",
        "BK0808": "军民融合",
        "BK0629": "北斗导航",
        "BK1111": "AIGC概念",
        "BK0715": "航母概念",
        "BK0476": "5G概念",
        "BK1035": "数字货币",
        "BK0718": "人工智能",
    }

    print(f"📋 获取热门概念板块成分股: {len(hot_concepts)} 个\n")

    concept_data = {}
    for code, name in hot_concepts.items():
        print(f"  获取 '{name}' ({code}) 的成分股...")
        stocks_df = get_concept_stocks(code)

        if stocks_df is not None and len(stocks_df) > 0:
            # 提取股票代码和名称
            stock_list = []
            for idx, row in stocks_df.head(20).iterrows():  # 只保存前20个
                stock_list.append({
                    'code': row['代码'],
                    'name': row['名称']
                })

            concept_data[name] = {
                'code': code,
                'stocks': stock_list,
                'total_count': len(stocks_df)
            }
            print(f"    ✅ 成功获取 {len(stocks_df)} 只股票，保存前{len(stock_list)}只")

    # 保存为JSON
    output_file = Path("concept_mapping.json")
    with open(output_file, 'w', encoding='utf-8') as f:
        json.dump(concept_data, f, ensure_ascii=False, indent=2)
    print(f"\n✅ 数据已保存到: {output_file}")

    # 生成Rust代码
    generate_rust_code(concept_data)


def generate_rust_code(concept_data):
    """生成Rust使用示例代码"""

    # 生成Rust映射表
    rust_code = """/// 东方财富概念板块成分股映射表
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
"""

    rust_code += "\nuse std::collections::HashMap;\n\n"
    rust_code += "/// 概念股信息\n"
    rust_code += "#[derive(Debug, Clone)]\n"
    rust_code += "pub struct ConceptStock {\n"
    rust_code += "    pub code: &'static str,\n"
    rust_code += "    pub name: &'static str,\n"
    rust_code += "}\n\n"

    rust_code += "/// 同花顺/东方财富概念板块成分股映射（示例数据）\n"
    rust_code += "/// \n"
    rust_code += "/// 返回指定概念的成分股列表（前20只）\n"
    rust_code += "pub fn get_concept_stocks(concept: &str) -> Option<Vec<ConceptStock>> {\n"
    rust_code += "    match concept {\n"

    for concept_name, data in concept_data.items():
        rust_code += f'        "{concept_name}" => Some(vec!['
        for stock in data['stocks']:
            rust_code += f'\n            ConceptStock {{ code: "{stock["code"]}", name: "{stock["name"]}" }},'
        rust_code += '\n        ]),\n'

    rust_code += "        _ => None,\n"
    rust_code += "    }\n"
    rust_code += "}\n\n"

    rust_code += "/// 获取所有支持的概念板块名称\n"
    rust_code += "pub fn get_concept_names() -> Vec<&'static str> {\n"
    rust_code += "    vec![\n"
    for concept_name in concept_data.keys():
        rust_code += f'        "{concept_name}",\n'
    rust_code += "    ]\n"
    rust_code += "}\n\n"

    rust_code += "/// 获取概念板块信息\n"
    rust_code += "pub fn get_concept_info(concept: &str) -> Option<(&'static str, usize)> {\n"
    rust_code += "    match concept {\n"

    for concept_name, data in concept_data.items():
        rust_code += f'        "{concept_name}" => Some(("{concept_name}", {data["total_count"]})),\n'

    rust_code += "        _ => None,\n"
    rust_code += "    }\n"
    rust_code += "}\n\n"

    rust_code += "#[cfg(test)]\n"
    rust_code += "mod tests {\n"
    rust_code += "    use super::*;\n\n"
    rust_code += "    #[test]\n"
    rust_code += "    fn test_get_concept_stocks() {\n"
    rust_code += "        let stocks = get_concept_stocks(\"新能源汽车\");\n"
    rust_code += "        assert!(stocks.is_some());\n"
    rust_code += "        let stocks = stocks.unwrap();\n"
    rust_code += "        assert!(!stocks.is_empty());\n"
    rust_code += "    }\n\n"
    rust_code += "    #[test]\n"
    rust_code += "    fn test_get_concept_names() {\n"
    rust_code += "        let names = get_concept_names();\n"
    rust_code += "        assert!(!names.is_empty());\n"
    rust_code += "    }\n"
    rust_code += "}\n"

    # 保存Rust代码
    rust_file = Path("concept_mapping.rs")
    with open(rust_file, 'w', encoding='utf-8') as f:
        f.write(rust_code)
    print(f"✅ Rust代码已生成: {rust_file}")

    # 生成使用示例
    generate_example_code()


def generate_example_code():
    """生成使用示例代码"""
    example_code = '''#!/usr/bin/env rustx
/**
概念股查询示例
*/
use rustdx_complete::tcp::stock::get_concept_stocks;

fn main() {
    println!("🚀 概念股查询示例\\n");

    // 查询新能源汽车概念的成分股
    if let Some(stocks) = get_concept_stocks("新能源汽车") {
        println!("📗 新能源汽车概念成分股（前20只）:");
        println!("   {:<10} {:<12}", "代码", "名称");
        println!("   {}", "-".repeat(30));

        for stock in stocks.iter().take(20) {
            println!("   {:<10} {:<12}", stock.code, stock.name);
        }
    }

    println!("\\n✅ 查询完成！");
}
'''

    example_file = Path("example_concept_query.rs")
    with open(example_file, 'w', encoding='utf-8') as f:
        f.write(example_code)
    print(f"✅ 使用示例已生成: {example_file}\n")

    print("💡 使用提示:")
    print("   1. 将 concept_mapping.rs 的内容复制到 src/tcp/stock/ 目录")
    print("   2. 在 mod.rs 中添加模块声明")
    print("   3. 可以定期运行此脚本更新概念板块数据")


if __name__ == "__main__":
    print("=" * 80)
    print("  东方财富概念板块数据获取工具")
    print("  （东方财富和同花顺概念板块类似）")
    print("=" * 80)
    print()

    try:
        generate_mapping()
    except KeyboardInterrupt:
        print("\n\n⚠️  用户中断")
    except Exception as e:
        print(f"\n❌ 发生错误: {e}")
        import traceback
        traceback.print_exc()

    print("\n" + "=" * 80)
    print("  执行完成")
    print("=" * 80)
