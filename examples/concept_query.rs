//! 板块/题材成分查询示例：从行情服务器取完整的板块数据。
//!
//! cargo run --example concept_query
use rustdx_complete::tcp::stock::{BlockKind, Client};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = Client::new()?;
    let blocks = client.blocks()?;

    let count = |kind| blocks.iter().filter(|b| b.kind == kind).count();
    println!(
        "共 {} 个板块：概念 {} / 风格 {} / 指数 {}",
        blocks.len(),
        count(BlockKind::Concept),
        count(BlockKind::Style),
        count(BlockKind::Index)
    );

    // 成分最多的 10 个概念
    let mut concepts: Vec<_> = blocks
        .iter()
        .filter(|b| b.kind == BlockKind::Concept)
        .collect();
    concepts.sort_by_key(|b| std::cmp::Reverse(b.stocks.len()));
    println!("\n成分最多的概念（名称 / 成分数 / 板块指数 / 最近调整）:");
    for b in concepts.iter().take(10) {
        println!(
            "  {:<10} {:>5} 只  {}  {}",
            b.name,
            b.stocks.len(),
            b.index_code.as_deref().unwrap_or("-"),
            b.updated.map(|d| d.to_string()).unwrap_or_default()
        );
    }

    // 某只股票属于哪些概念
    let code = "600519";
    let belongs: Vec<_> = concepts
        .iter()
        .filter(|b| b.stocks.iter().any(|(_, c)| c == code))
        .map(|b| b.name.as_str())
        .collect();
    println!("\n{code} 所属概念: {}", belongs.join(" "));
    Ok(())
}
