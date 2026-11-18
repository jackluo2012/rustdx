use insta::assert_debug_snapshot;

#[test]
fn day_sz000001() -> rustdx_complete::Result<()> {
    use rustdx_complete::file::{
        day::Day,
        gbbq::{Fq, Gbbq},
    };
    let day_src = std::fs::read("assets/sz000001.day")?;
    let days = day_src
        .as_chunks::<32>()
        .0
        .iter()
        .map(|arr| Day::from_bytes("sz000001", arr));

    let mut gbbq_src = std::fs::read("assets/gbbq")?;
    let stock_gbbq = Gbbq::filter_hashmap(Gbbq::iter(&mut gbbq_src[4..]));

    let fq = Fq::new(days, stock_gbbq.get("sz000001").unwrap()).unwrap();
    assert_debug_snapshot!(&fq[..3]);
    Ok(())
}

/// 增量复权：用第 N-5 天的因子接续，无论传全量文件还是只含新增交易日的文件，
/// 结果都应与全量计算的最后 5 天逐项一致；没有新增交易日时返回空。
#[test]
fn concat_matches_full_computation() -> rustdx_complete::Result<()> {
    use rustdx_complete::file::{
        day::fq::Day,
        gbbq::{Factor, Gbbq},
    };
    let mut gbbq_src = std::fs::read("assets/gbbq")?;
    let gbbq = Gbbq::filter_hashmap(Gbbq::iter(&mut gbbq_src[4..]));
    let g = gbbq.get("sz000001").map(Vec::as_slice);
    let full = Day::new("sz000001", "assets/sz000001.day", g)?;
    let n = full.len();
    let factor_at = |i: usize| Factor {
        date: full[i].date.clone(),
        code: full[i].code.clone(),
        preclose: full[i].close,
        factor: full[i].factor,
    };
    let same = |got: &[Day], want: &[Day]| {
        assert_eq!(got.len(), want.len());
        for (a, b) in got.iter().zip(want) {
            assert_eq!(a.date, b.date);
            assert_eq!(a.close, b.close);
            assert!((a.preclose - b.preclose).abs() < 1e-9, "{}", a.date);
            assert!((a.factor / b.factor - 1.).abs() < 1e-12, "{}", a.date);
        }
    };

    // 1) 全量文件 + 上次因子：自动跳过已有交易日
    let prev = factor_at(n - 6);
    let incr = Day::concat("sz000001", "assets/sz000001.day", g, Some(&prev))?;
    same(&incr, &full[n - 5..]);

    // 2) 只含新增交易日的文件
    let raw = std::fs::read("assets/sz000001.day")?;
    let tmp = std::env::temp_dir().join("rustdx_fq_concat_test.day");
    std::fs::write(&tmp, &raw[raw.len() - 5 * 32..])?;
    let incr = Day::concat("sz000001", &tmp, g, Some(&prev))?;
    same(&incr, &full[n - 5..]);
    std::fs::remove_file(&tmp)?;

    // 3) 无股本变迁记录的股票同样接续因子，而不是从 1 重新起算
    let no_gbbq_full = Day::new("sz000001", "assets/sz000001.day", None)?;
    let prev_ng = Factor {
        date: no_gbbq_full[n - 6].date.clone(),
        code: no_gbbq_full[n - 6].code.clone(),
        preclose: no_gbbq_full[n - 6].close,
        factor: no_gbbq_full[n - 6].factor,
    };
    let incr = Day::concat("sz000001", "assets/sz000001.day", None, Some(&prev_ng))?;
    same(&incr, &no_gbbq_full[n - 5..]);

    // 4) 没有新增交易日
    let latest = factor_at(n - 1);
    assert!(Day::concat("sz000001", "assets/sz000001.day", g, Some(&latest))?.is_empty());
    Ok(())
}
