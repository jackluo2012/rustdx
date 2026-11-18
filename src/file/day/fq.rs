use crate::{
    Error::Custom,
    Result,
    file::gbbq::{Factor, Fq, Gbbq},
};
use std::path::Path;

/// 注意：成交量的单位为 “手”，而不是股。在通达信和交易所数据里，单位为股。
#[derive(Debug, Clone, serde::Serialize)]
pub struct Day {
    pub date: String,
    pub code: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub amount: f64,
    pub vol: f64,
    pub preclose: f64,
    pub factor: f64,
}

impl Day {
    pub fn new(code: &str, p: impl AsRef<Path>, gbbqs: Option<&[Gbbq]>) -> Result<Vec<Self>> {
        let raw = std::fs::read(p)?;
        let code = code.to_owned();
        let days = raw
            .as_chunks::<32>()
            .0
            .iter()
            .map(|b| super::Day::from_bytes(&code, b));
        let fq = gbbqs
            .map(|g| Fq::new(days.clone(), g))
            .unwrap_or(Fq::no_gbbq(days.clone()))
            .ok_or(Custom("复权失败"))?
            .into_iter()
            .filter(|d| d.trading);
        Ok(days
            .zip(fq)
            .map(|(d, f)| Self {
                date: crate::bytes_helper::date_string(d.date),
                code: d.code,
                open: d.open,
                high: d.high,
                low: d.low,
                close: d.close,
                amount: d.amount,
                vol: d.vol as f64 / 100.,
                preclose: f.preclose,
                factor: f.factor,
            })
            .collect())
    }

    /// 接续已有的复权因子做增量计算。
    ///
    /// `f` 为该股票上次的最新因子（日期、收盘价、因子）。day 文件里日期不晚于
    /// `f.date` 的交易日会被跳过，所以既可以传只含新增交易日的文件，也可以直接
    /// 传全量文件；没有新增交易日时返回空。`f` 为 `None` 时从文件首日起算。
    pub fn concat(
        code: &str,
        p: impl AsRef<Path>,
        gbbqs: Option<&[Gbbq]>,
        f: Option<&Factor>,
    ) -> Result<Vec<Self>> {
        let raw = std::fs::read(p)?;
        let code = code.to_owned();
        let days: Vec<super::Day> = raw
            .as_chunks::<32>()
            .0
            .iter()
            .map(|b| super::Day::from_bytes(&code, b))
            // 日期字符串为 `YYYY-MM-DD`，字典序即时间序
            .filter(|d| f.is_none_or(|f| crate::bytes_helper::date_string(d.date) > f.date))
            .collect();
        let Some(first) = days.first() else {
            return if f.is_some() {
                Ok(Vec::new()) // 没有新增交易日
            } else {
                Err(Custom("初始化前收盘价失败"))
            };
        };
        let (preclose, factor) = f
            .map(|f| (f.preclose, f.factor))
            .unwrap_or((first.close, 1.));
        let days = days.into_iter();
        // 无股本变迁记录的股票也要接续上次的因子，不能从 1 重新起算
        let fq = match gbbqs {
            Some(g) => Fq::concat(days.clone(), g, preclose, factor),
            None => Fq::no_gbbq_from(days.clone(), preclose, factor),
        }
        .ok_or(Custom("复权失败"))?
        .into_iter()
        .filter(|d| d.trading);
        Ok(days
            .zip(fq)
            .map(|(d, f)| Self {
                date: crate::bytes_helper::date_string(d.date),
                code: d.code,
                open: d.open,
                high: d.high,
                low: d.low,
                close: d.close,
                amount: d.amount,
                vol: d.vol as f64 / 100.,
                preclose: f.preclose,
                factor: f.factor,
            })
            .collect())
    }
}
