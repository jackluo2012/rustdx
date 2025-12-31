#!/usr/bin/env python3
"""测试AKShare概念板块成分股获取"""

import akshare as ak
import pandas as pd

# 设置pandas显示选项
pd.set_option('display.max_columns', None)
pd.set_option('display.width', 200)

print("测试获取概念板块信息\n")
print("=" * 80)

try:
    # 尝试使用concept_info获取成分股
    # 先看看需要什么参数
    print("测试 stock_board_concept_info_ths:\n")

    # 使用概念代码
    info_df = ak.stock_board_concept_info_ths(symbol="人脸识别", symbol_type="概念板块")
    print(f"数据形状: {info_df.shape}")
    print(f"列名: {info_df.columns.tolist()}\n")

    print("前10行:")
    print(info_df.head(10))

except Exception as e:
    print(f"错误: {e}")
    import traceback
    traceback.print_exc()

print("\n" + "=" * 80)
print("尝试使用概念代码\n")

try:
    # 使用数字代码试试
    code_df = ak.stock_board_concept_info_ths(symbol="301558", symbol_type="概念板块")
    print(f"数据形状: {code_df.shape}")
    print(f"列名: {code_df.columns.tolist()}\n")
    print("前10行:")
    print(code_df.head(10))

except Exception as e:
    print(f"错误: {e}")
