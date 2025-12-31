#!/usr/bin/env python3
"""测试AKShare数据结构"""

import akshare as ak
import pandas as pd

# 设置pandas显示选项
pd.set_option('display.max_columns', None)
pd.set_option('display.width', None)
pd.set_option('display.max_colwidth', 50)

print("测试获取同花顺概念板块名称\n")
print("=" * 80)

try:
    # 获取概念板块名称
    df = ak.stock_board_concept_name_ths()

    print(f"数据形状: {df.shape}")
    print(f"列名: {df.columns.tolist()}\n")

    print("前10行数据:")
    print(df.head(10))
    print("\n")

    # 尝试获取某个概念的成分股
    print("=" * 80)
    print("测试获取'人脸识别'概念的成分股\n")

    stocks_df = ak.stock_board_concept_cons_ths(symbol_code="人脸识别")
    print(f"成分股数据形状: {stocks_df.shape}")
    print(f"列名: {stocks_df.columns.tolist()}\n")

    print("前5只股票:")
    print(stocks_df.head())

except Exception as e:
    print(f"错误: {e}")
    import traceback
    traceback.print_exc()
