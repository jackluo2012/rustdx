use rustdx_cmd::{eastmoney, fetch_code};
use std::{collections::HashSet, sync::LazyLock};

macro_rules! get {
    (sz) => {{
        let mut set = ::std::collections::HashSet::with_capacity(3000);
        fetch_code::get_sz_stocks(&mut set).unwrap();
        let mut v = set.into_iter().collect::<Vec<_>>();
        v.sort();
        v
    }};
    (sh, $a:literal, $b:literal) => {{
        let mut set = ::std::collections::HashSet::with_capacity(3000);
        fetch_code::get_sh_stocks(&mut set, $a, $b).unwrap();
        let mut v = set.into_iter().collect::<Vec<_>>();
        v.sort();
        v
    }};
}

static SH8: LazyLock<Vec<String>> = LazyLock::new(|| get!(sh, "8", "1000"));
static SH1: LazyLock<Vec<String>> = LazyLock::new(|| get!(sh, "1", "2500"));
static SZ: LazyLock<Vec<String>> = LazyLock::new(|| get!(sz));

/// 打印当前抓取结果（证券名单随上市/退市每日变动，无法快照对比，仅供人工查看）。
#[test]
fn head() {
    let (sh8, sh1, sz) = (&*SH8, &*SH1, &*SZ);
    println!(
        "sh8: {}\n{:?}\n{:?}",
        sh8.len(),
        &sh8[..10],
        &sh8[sh8.len() - 10..]
    );
    println!(
        "sh1: {}\n{:?}\n{:?}",
        sh1.len(),
        &sh1[..10],
        &sh1[sh1.len() - 10..]
    );
    println!(
        "sz: {}\n{:?}\n{:?}",
        sz.len(),
        &sz[..10],
        &sz[sz.len() - 10..]
    );
}

/// 验证交易所官网与东财两条数据获取通道均可用，
/// 并对名单数量与代码格式做合理性断言（不对比具体名单）。
#[test]
fn stocklist() {
    let (sh8, sh1, sz) = (&*SH8, &*SH1, &*SZ);
    let (lsh8, lsh1, lsz) = (sh8.len(), sh1.len(), sz.len());

    // 数量级断言：证券数量随时间缓慢增长，异常缩小才说明抓取出了问题
    assert!(lsh1 > 1000, "沪市主板(60x)股票数异常: {lsh1}");
    assert!(lsh8 > 300, "科创板(688)股票数异常: {lsh8}");
    assert!(lsz > 2000, "深市股票数异常: {lsz}");

    // 代码格式断言
    assert!(
        sh1.iter().all(|s| s.starts_with("sh60")),
        "沪市主板代码前缀异常"
    );
    assert!(
        sh8.iter()
            .all(|s| s.starts_with("sh688") || s.starts_with("sh689")),
        "科创板代码前缀异常（688/689，689 为 CDR 存托凭证）"
    );
    assert!(
        sz.iter()
            .all(|s| s.starts_with("sz00") || s.starts_with("sz30")),
        "深市代码前缀异常"
    );
}

/// 东财通道及其与交易所数据的交叉验证。
///
/// 注意：东财 push2 接口在部分代理/VPN 环境（fake-IP DNS）下会被断开，
/// 需要直连网络运行：`cargo test -p tests-integration --test stocklist -- --ignored`
#[test]
#[ignore = "东财接口需直连网络（代理环境下不可用）"]
fn east_crosscheck() {
    let (sh8, sh1, sz) = (&*SH8, &*SH1, &*SZ);

    // 东财通道
    let res = eastmoney::fetch(None).unwrap();
    let east: HashSet<_> = res
        .data
        .diff
        .into_iter()
        .filter_map(|v| v.open.map(|_| v.code)) // 这排除了不需要的股票
        .collect();
    let total = res.data.total as usize;
    assert!(
        east.len() <= total,
        "东财有效股票数 ({}) 不应大于 total ({total})",
        east.len()
    );
    assert!(total > 4000, "东财股票总数异常: {total}");

    // 交叉验证：两通道的差集应远小于各自总量（停牌、ST 等会导致少量差异）
    let exchange = HashSet::from_iter(
        [sh8.iter(), sh1.iter(), sz.iter()]
            .into_iter()
            .flatten()
            .map(|s| s[2..].to_string()),
    );
    let diff_east_exchange = east.difference(&exchange).count();
    let diff_exchange_east = exchange.difference(&east).count();
    println!(
        "exchange: {}, east: {}, diff(east-exchange): {diff_east_exchange}, \
         diff(exchange-east): {diff_exchange_east}",
        exchange.len(),
        east.len()
    );
    assert!(
        diff_east_exchange < east.len() / 10,
        "东财独有股票过多: {diff_east_exchange}"
    );
    assert!(
        diff_exchange_east < exchange.len() / 10,
        "交易所独有股票过多: {diff_exchange_east}"
    );
}
