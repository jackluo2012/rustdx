#!/usr/bin/env python3
"""
获取同花顺概念板块数据并生成Rust映射表

使用AKShare库获取同花顺概念板块数据，生成可用于rustdx的代码
"""

import akshare as ak
import json
from pathlib import Path


def get_all_concept_names():
    """获取所有同花顺概念板块名称"""
    print("🔍 获取同花顺概念板块名称列表...")
    try:
        # 获取概念板块名称
        df = ak.stock_board_concept_name_ths()
        print(f"✅ 成功获取 {len(df)} 个概念板块\n")
        return df
    except Exception as e:
        print(f"❌ 获取失败: {e}")
        return None


def get_concept_stocks(concept_name):
    """获取指定概念板块的成分股"""
    try:
        df = ak.stock_board_concept_cons_ths(symbol_code=concept_name)
        return df
    except Exception as e:
        print(f"⚠️  获取概念 '{concept_name}' 成分股失败: {e}")
        return None


def generate_concept_code():
    """生成概念板块映射的Rust代码"""
    print("🚀 开始生成概念板块映射代码\n")

    # 获取所有概念板块
    concepts_df = get_all_concept_names()
    if concepts_df is None:
        return

    # 显示前20个概念板块
    print("📋 同花顺概念板块示例（前20个）:")
    print("-" * 80)
    for idx, row in concepts_df.head(20).iterrows():
        print(f"  {row['板块名称']}")
    print("-" * 80)

    # 获取几个热门概念的成分股作为示例
    hot_concepts = ["人脸识别", "锂电池", "芯片", "新能源汽车", "人工智能", "5G概念", "数字货币"]

    print(f"\n🔥 获取热门概念板块的成分股: {', '.join(hot_concepts)}\n")

    concept_data = {}
    for concept in hot_concepts:
        print(f"  获取 '{concept}' 的成分股...")
        stocks_df = get_concept_stocks(concept)
        if stocks_df is not None and len(stocks_df) > 0:
            # 提取股票代码
            stock_codes = stocks_df['代码'].tolist()
            concept_data[concept] = {
                'stocks': stock_codes[:10],  # 只保存前10个作为示例
                'count': len(stocks_df)
            }
            print(f"    ✅ 成功获取 {len(stocks_df)} 只股票")

    # 保存为JSON
    output_file = Path("ths_concepts_example.json")
    with open(output_file, 'w', encoding='utf-8') as f:
        json.dump(concept_data, f, ensure_ascii=False, indent=2)
    print(f"\n✅ 数据已保存到: {output_file}")

    # 生成Rust代码示例
    generate_rust_example(concept_data)


def generate_rust_example(concept_data):
    """生成Rust使用示例代码"""
    rust_code = r"""/// 同花顺概念板块数据示例
///
/// 数据来源: AKShare (同花顺概念板块)
/// 更新时间: 手动更新
///
/// 使用方法:
/// ```ignore
/// use rustdx_complete::tcp::stock::get_ths_concept_stocks;
///
/// // 获取"人脸识别"概念的成分股
/// let stocks = get_ths_concept_stocks("人脸识别");
/// println!("人脸识别概念股: {:?}", stocks);
/// ```
"""

    rust_code += "\nuse std::collections::HashMap;\n\n"
    rust_code += "/// 同花顺概念板块成分股映射（示例数据）\n"
    rust_code += "pub fn get_ths_concept_stocks(concept: &str) -> Option<Vec<&'static str>> {\n"
    rust_code += "    match concept {\n"

    for concept, data in concept_data.items():
        rust_code += f'        "{concept}" => Some(vec!['
        for code in data['stocks']:
            rust_code += f'"{code}", '
        rust_code += "]),\n"

    rust_code += "        _ => None,\n"
    rust_code += "    }\n"
    rust_code += "}\n\n"

    rust_code += "/// 获取所有支持的概念板块名称\n"
    rust_code += "pub fn get_all_concept_names() -> Vec<&'static str> {\n"
    rust_code += "    vec![\n"
    for concept in concept_data.keys():
        rust_code += f'        "{concept}",\n'
    rust_code += "    ]\n"
    rust_code += "}\n"

    # 保存Rust代码
    rust_file = Path("ths_concepts_example.rs")
    with open(rust_file, 'w', encoding='utf-8') as f:
        f.write(rust_code)
    print(f"✅ Rust代码已生成: {rust_file}\n")

    print("💡 提示:")
    print("   1. 将生成的JSON数据用于建立股票代码到概念的映射")
    print("   2. 可以定期运行此脚本更新概念板块数据")
    print("   3. 建议结合通达信的行业代码一起使用")


if __name__ == "__main__":
    print("=" * 80)
    print("  同花顺概念板块数据获取工具")
    print("=" * 80)
    print()

    try:
        generate_concept_code()
    except KeyboardInterrupt:
        print("\n\n⚠️  用户中断")
    except Exception as e:
        print(f"\n❌ 发生错误: {e}")
        import traceback
        traceback.print_exc()

    print("\n" + "=" * 80)
    print("  执行完成")
    print("=" * 80)
