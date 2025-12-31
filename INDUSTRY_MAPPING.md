# 股票行业和省份代码映射功能

## 📖 功能概述

rustdx 现已支持获取股票的行业和省份信息，并提供代码到名称的自动映射功能。

### 主要特性

✅ **行业代码映射**: 将通达信行业代码（u16）自动转换为中文行业名称
✅ **省份代码映射**: 将通达信省份代码（u16）自动转换为中文省份名称
✅ **行业大类分类**: 提供行业大类信息（金融、消费、科技、材料等）
✅ **完整测试**: 包含单元测试和示例程序

## 🚀 快速开始

### 1. 基本用法

```rust
use rustdx_complete::tcp::{Tcp, Tdx};
use rustdx_complete::tcp::stock::{FinanceInfo, get_industry_name, get_province_name};

fn main() -> eyre::Result<()> {
    // 创建TCP连接
    let mut tcp = Tcp::new()?;

    // 获取股票财务信息（包含行业和省份代码）
    let mut finance = FinanceInfo::new(0, "000001");  // 平安银行
    finance.recv_parsed(&mut tcp)?;

    let info = &finance.result()[0];

    // 直接使用行业代码和省份代码
    println!("股票代码: {}", info.code);
    println!("行业代码: {}", info.industry);        // 输出: 1
    println!("省份代码: {}", info.province);        // 输出: 18

    // 使用映射函数获取名称
    let industry_name = get_industry_name(info.industry);
    let province_name = get_province_name(info.province);

    println!("行业名称: {}", industry_name);         // 输出: 银行
    println!("省份名称: {}", province_name);         // 输出: 深圳

    Ok(())
}
```

### 2. 获取详细行业信息

```rust
use rustdx_complete::tcp::stock::get_industry_info;

fn example() {
    let (code, name, category) = get_industry_info(1);
    println!("代码: {}, 名称: {}, 大类: {}", code, name, category);
    // 输出: 代码: 1, 名称: 银行, 大类: 金融
}
```

## 📊 支持的行业代码

### 金融行业
- `1`: 银行
- `2`: 证券
- `3`: 保险
- `4`: 信托
- `5`: 其他金融

### 房地产
- `10`: 房地产
- `11`: 房地产开发
- `12`: 园区开发

### 消费行业
- `20`: 食品饮料
- `21`: 酒类
- `22`: 纺织服装
- `23`: 轻工制造
- `24`: 商业贸易
- `25`: 休闲服务
- `37`: 酒类（五粮液、贵州茅台等）

### 医药生物
- `30`: 医药生物
- `31`: 化学制药
- `32`: 中药
- `33`: 生物制品
- `34`: 医疗器械
- `35`: 医疗服务

### 科技行业
- `40`: 电子
- `41`: 计算机
- `42`: 通信
- `43`: 电气设备
- `44`: 机械设备
- `45`: 国防军工

### 材料行业
- `50`: 化工
- `51`: 有色金属
- `52`: 钢铁
- `53`: 建筑材料
- `54`: 建筑装饰

### 能源行业
- `60`: 采掘
- `61`: 石油石化
- `62`: 煤炭
- `63`: 公用事业
- `64`: 环保

### 交通运输
- `70`: 交通运输
- `71`: 汽车
- `72`: 轨道交通

### 其他
- `80`: 传媒
- `81`: 农林牧渔
- `82`: 综合

> 💡 **提示**: 行业代码基于通达信标准，实际使用中可能需要根据具体情况调整映射表。

## 🌍 支持的省份代码

```rust
1 => "北京"
2 => "上海"
3 => "天津"
4 => "重庆"
5 => "广东"
6 => "浙江"
7 => "江苏"
18 => "深圳"  // 特别处理
23 => "四川"  // 五粮液
29 => "贵州"  // 贵州茅台
// ... 更多省份代码
```

## 🧪 运行示例程序

项目提供了完整的示例程序，展示如何使用行业和省份映射功能：

```bash
# 运行行业信息测试示例
cargo run --example test_industry_info
```

### 示例输出

```
🚀 测试股票行业和省份信息获取功能

1️⃣  连接到通达信服务器...
   ✅ 连接成功

2️⃣  获取测试股票的行业和省份信息:

   股票         名称           行业           省份           行业大类
   ----------------------------------------------------------------------
   000001     平安银行         银行           深圳           金融
   000002     万科A          房地产开发        深圳           房地产
   600000     浦发银行         银行           辽宁           金融
   600036     招商银行         银行           深圳           金融
   000858     五粮液          酒类           四川           消费
   600519     贵州茅台         酒类           贵州           消费
   300750     宁德时代         电气设备         福建           科技
   601318     中国平安         银行           深圳           金融

✅ 测试完成！
```

## 🧪 运行测试

```bash
# 运行所有单元测试
cargo test

# 仅运行行业映射相关测试
cargo test industry_mapping
```

## 📝 API 文档

### `get_industry_name(code: u16) -> &'static str`

根据行业代码获取行业名称。

**参数**:
- `code`: 通达信行业代码（u16）

**返回**:
- `&'static str`: 行业名称，如果代码未知则返回"未知行业"

**示例**:
```rust
assert_eq!(get_industry_name(1), "银行");
assert_eq!(get_industry_name(999), "未知行业");
```

### `get_industry_info(code: u16) -> (u16, &'static str, &'static str)`

获取行业详细信息，包括代码、名称和大类。

**参数**:
- `code`: 通达信行业代码（u16）

**返回**:
- `(u16, &'static str, &'static str)`: 元组包含（代码, 名称, 大类）

**示例**:
```rust
let (code, name, category) = get_industry_info(1);
assert_eq!(code, 1);
assert_eq!(name, "银行");
assert_eq!(category, "金融");
```

### `get_province_name(code: u16) -> &'static str`

根据省份代码获取省份名称。

**参数**:
- `code`: 通达信省份代码（u16）

**返回**:
- `&'static str`: 省份名称，如果代码未知则返回"未知省份"

**示例**:
```rust
assert_eq!(get_province_name(18), "深圳");
assert_eq!(get_province_name(999), "未知省份");
```

## 🔧 自定义映射

如果需要添加或修改行业/省份映射，可以编辑 `src/tcp/stock/industry_mapping.rs` 文件。

### 添加新的行业代码

```rust
pub fn get_industry_name(code: u16) -> &'static str {
    match code {
        // ... 现有代码 ...

        // 添加新的行业代码
        200 => "新行业",

        _ => "未知行业",
    }
}
```

### 添加新的省份代码

```rust
pub fn get_province_name(code: u16) -> &'static str {
    match code {
        // ... 现有代码 ...

        // 添加新的省份代码
        100 => "新省份",

        _ => "未知省份",
    }
}
```

## ⚠️ 注意事项

1. **代码标准**: 行业代码基于通达信标准，不同数据源可能使用不同的代码标准
2. **代码更新**: 通达信行业代码可能随时间调整，建议定期更新映射表
3. **未知代码**: 如果遇到未知代码，函数会返回"未知行业"或"未知省份"
4. **扩展性**: 映射表可以轻松扩展，添加新的行业和省份代码

## 📚 相关资源

- [通达信行业分类标准](https://zhuanlan.zhihu.com/p/1961011810472814438)
- [pytdx 项目](https://pypi.org/project/pytdx/1.28)
- [如何用pytdx解析通达信板块文件](https://blog.csdn.net/weixin_37164550/article/details/152458199)

## 📄 文件位置

- **映射模块**: `src/tcp/stock/industry_mapping.rs`
- **示例程序**: `examples/test_industry_info.rs`
- **使用文档**: `INDUSTRY_MAPPING.md`

## 🎯 总结

rustdx 的行业和省份映射功能让获取股票分类信息变得简单直观：

✅ **开箱即用**: 无需手动处理代码，直接获取中文名称
✅ **完整覆盖**: 支持主流行业和省份
✅ **易于扩展**: 可以轻松添加新的映射关系
✅ **测试完善**: 包含单元测试和示例程序

**Sources:**
- [通达信全能实战指南·板块管理](https://zhuanlan.zhihu.com/p/1961011810472814438)
- [如何用pytdx来有效的解析通达信板块文件](https://blog.csdn.net/weixin_37164550/article/details/152458199)
