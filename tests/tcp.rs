// use insta::{assert_debug_snapshot, assert_yaml_snapshot};
use insta::assert_debug_snapshot;
use rustdx_complete::tcp::{self, Tcp, Tdx};
use std::io::Result;

#[test]
fn tcp_security_count() -> Result<()> {
    if std::env::var("RUSTDX_SKIP_INTEGRATION_TESTS").is_ok() {
        println!("⚠️  跳过集成测试 (RUSTDX_SKIP_INTEGRATION_TESTS 已设置)");
        return Ok(());
    }

    let mut tcp = Tcp::new()?;

    // 证券数量随时间增长（2021 年深市约 1.3 万、2026 年已超 1.4 万），
    // 因此只做合理范围断言，不对具体数值做快照。
    let mut count = tcp::SecurityCount::new(0); // sz
    let c = *count.recv_parsed(&mut tcp)?;
    assert!((5000..=65535).contains(&c), "深市证券数量异常: {c}");

    let mut count = tcp::SecurityCount::new(1); // sh
    let c = *count.recv_parsed(&mut tcp)?;
    assert!((5000..=65535).contains(&c), "沪市证券数量异常: {c}");

    Ok(())
}

#[test]
fn tcp_security_list() -> Result<()> {
    if std::env::var("RUSTDX_SKIP_INTEGRATION_TESTS").is_ok() {
        println!("⚠️  跳过集成测试 (RUSTDX_SKIP_INTEGRATION_TESTS 已设置)");
        return Ok(());
    }

    let mut list = tcp::SecurityList::default(); // sz
    assert_debug_snapshot!("security-list-send", list.send);
    list.recv_parsed(&mut Tcp::new()?)?;
    assert_debug_snapshot!("security-list-count", list.count);
    // assert_yaml_snapshot!("security-list-recv", list.data);
    Ok(())
}
