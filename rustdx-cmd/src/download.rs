//! 通达信日线完整包（hsjday.zip）的自动下载、解压与解析目录发现。
//!
//! 供 `rustdx day`（无目录参数）使用：
//! 1. 从默认地址 <https://data.tdx.com.cn/vipdoc/hsjday.zip> 下载完整包；
//! 2. 下载失败时自动重试，仍失败则交互询问用户提供备选下载地址（URL）或
//!    本地 zip 文件路径；
//! 3. 断点续传：中断后重跑从已下载位置继续（服务器支持 `Accept-Ranges`）；
//! 4. 缓存跳过：按 ETag/Last-Modified/大小比对服务器文件，未变化则跳过
//!    下载与解压，二次运行接近秒开；
//! 5. 解压后收集 `sh/lday`、`sz/lday`、`bj/lday` 等目录，返回给解析流程。

use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use eyre::{bail, eyre, Result};
use serde::{Deserialize, Serialize};

/// 通达信官网日线完整包默认地址。
pub const DEFAULT_HSJDAY_URL: &str = "https://data.tdx.com.cn/vipdoc/hsjday.zip";

/// 下载/解压缓存目录下保存的元信息（与服务器文件比对，决定是否跳过下载/解压）。
#[derive(Serialize, Deserialize, Default)]
struct Meta {
    url: String,
    etag: Option<String>,
    last_modified: Option<String>,
    size: u64,
    downloaded_ok: bool,
    extracted_ok: bool,
    downloaded_at: String,
}

/// HEAD 探测到的服务器文件信息。
struct HeadInfo {
    etag: Option<String>,
    last_modified: Option<String>,
    size: Option<u64>,
}

/// 自动从默认地址下载并解压，返回解压出的 `lday` 目录列表。
pub fn prepare_hsjday_dirs() -> Result<Vec<PathBuf>> {
    prepare_hsjday_dirs_from(DEFAULT_HSJDAY_URL)
}

/// 从指定 URL 下载并解压（带缓存跳过 / 断点续传 / 自动重试 / 失败交互）。
pub fn prepare_hsjday_dirs_from(url: &str) -> Result<Vec<PathBuf>> {
    let workdir = cache_dir_for(url);
    std::fs::create_dir_all(&workdir)?;
    let zip_path = workdir.join("hsjday.zip");
    let extract_dir = workdir.join("extracted");
    let meta_path = workdir.join("meta.json");
    let mut meta = read_meta(&meta_path)?;

    // 1. 下载（服务器文件未变化 → 跳过下载）
    let head = head_info(url).ok(); // 探测失败不阻塞，见下方判断
    if zip_cached(url, &zip_path, &meta, head.as_ref()) {
        log::info!("缓存命中：服务器文件未变化，跳过下载 {}", zip_path.display());
    } else {
        download_with_fallback(url, &zip_path)?;
        let zip_len = std::fs::metadata(&zip_path)?.len();
        meta = Meta {
            url: url.to_string(),
            etag: head.as_ref().and_then(|h| h.etag.clone()),
            last_modified: head.as_ref().and_then(|h| h.last_modified.clone()),
            size: zip_len,
            downloaded_ok: true,
            extracted_ok: false,
            downloaded_at: chrono::Local::now()
                .format("%Y-%m-%d %H:%M:%S")
                .to_string(),
        };
        write_meta(&meta_path, &meta)?;
    }

    // 2. 解压（已解压且目录完整 → 跳过解压）
    if meta.extracted_ok && extract_dir.is_dir() && !find_lday_dirs(&extract_dir).is_empty() {
        log::info!("解压缓存命中：{}", extract_dir.display());
    } else {
        if extract_dir.exists() {
            std::fs::remove_dir_all(&extract_dir)?;
        }
        std::fs::create_dir_all(&extract_dir)?;
        extract_zip(&zip_path, &extract_dir)?;
        meta.extracted_ok = true;
        write_meta(&meta_path, &meta)?;
    }

    // 3. 收集 lday 目录（sh/lday、sz/lday、bj/lday ...）
    let dirs = find_lday_dirs(&extract_dir);
    if dirs.is_empty() {
        bail!(
            "解压后未找到含 *.day 文件的 lday 目录，来源: {url}（解压根: {}）",
            extract_dir.display()
        );
    }
    for d in &dirs {
        log::info!("发现解析目录: {}", d.display());
    }
    Ok(dirs)
}

/// 直接解压本地 zip 文件，返回解压出的 `lday` 目录列表。
pub fn prepare_hsjday_dirs_from_zip(zip_path: &Path) -> Result<Vec<PathBuf>> {
    let workdir = std::env::temp_dir().join("rustdx_hsjday");
    std::fs::create_dir_all(&workdir)?;
    extract_and_discover(
        zip_path,
        &workdir.join("extracted"),
        &zip_path.display().to_string(),
    )
}

/// 下载（首次失败自动重试 2 次；仍失败 → 交互提供备选 URL / 本地 zip 路径）。
fn download_with_fallback(initial_url: &str, zip_path: &Path) -> Result<()> {
    let mut last_err: Option<eyre::Report> = None;
    for attempt in 1..=3 {
        match download(initial_url, zip_path) {
            Ok(()) => return Ok(()),
            Err(e) => {
                log::warn!("下载失败（第 {attempt}/3 次）: {e:#}");
                last_err = Some(e);
                if attempt < 3 {
                    std::thread::sleep(Duration::from_secs(attempt as u64));
                }
            }
        }
    }
    log::warn!(
        "连续 3 次下载失败，进入交互模式：{}",
        last_err.as_ref().map(|e| format!("{e:#}")).unwrap_or_default()
    );

    let fallback = prompt_fallback()?;
    if fallback.starts_with("http://") || fallback.starts_with("https://") {
        download(&fallback, zip_path).map_err(|e2| eyre!("备选地址下载失败: {e2:#}"))?;
    } else {
        let p = PathBuf::from(&fallback);
        if !p.is_file() {
            bail!("提供的文件不存在: {}", p.display());
        }
        log::info!("使用本地 zip: {}", p.display());
        std::fs::copy(&p, zip_path)?;
    }
    Ok(())
}

/// 下载 url 到 dest 文件，支持断点续传（服务器返回 206 时从已下载位置追加）。
fn download(url: &str, dest: &Path) -> Result<()> {
    let done = if dest.exists() {
        std::fs::metadata(dest)?.len()
    } else {
        0
    };
    if done > 0 {
        log::info!(
            "正在下载 {url} ...（断点续传，已下载 {:.1} MiB）",
            done as f64 / (1024.0 * 1024.0)
        );
    } else {
        log::info!("正在下载 {url} ...");
    }

    let mut req = ureq::get(url);
    if done > 0 {
        req = req.header("Range", &format!("bytes={done}-"));
    }
    let mut resp = req.call().map_err(|e| eyre!("{e}"))?;

    let mut start = 0u64;
    match resp.status().as_u16() {
        200 if done > 0 => {
            // 服务器不支持 Range（或文件已变）：从头下载
            log::warn!("服务器返回 200（不支持断点续传），从头下载");
        }
        206 => start = done,
        200 => {}
        s => bail!("HTTP {s}"),
    }

    let mut out = if start > 0 {
        std::fs::OpenOptions::new().append(true).open(dest)?
    } else {
        std::fs::File::create(dest)?
    };

    let mut reader = resp.body_mut().as_reader();
    let mut buf = [0u8; 64 * 1024];
    let mut total = start;
    let mut last_mark = start / (32 * 1024 * 1024);
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n])?;
        total += n as u64;
        let mark = total / (32 * 1024 * 1024);
        if mark > last_mark {
            last_mark = mark;
            log::info!("已下载 {:.0} MiB", mark as f64 * 32.0);
        }
    }
    out.flush()?;
    log::info!(
        "下载完成: {} ({:.1} MiB)",
        dest.display(),
        total as f64 / (1024.0 * 1024.0)
    );
    Ok(())
}

/// 下载失败后的交互：询问用户提供备选 URL 或本地 zip 文件路径。
fn prompt_fallback() -> Result<String> {
    eprintln!(
        "下载失败。请提供备选下载地址（URL）或本地 zip 文件路径（直接回车退出）："
    );
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    let s = line.trim().to_string();
    if s.is_empty() {
        bail!("未提供备选来源，已退出。可重跑：`rustdx day <下载地址或本地zip路径>`");
    }
    Ok(s)
}

/// 解压 zip 到 dest。
///
/// 通达信 zip 的条目名使用 Windows 反斜杠分隔（如 `sh\lday\sh000001.day`），
/// 这里统一归一化为 `/`，并做路径穿越（zip-slip）安全检查。
fn extract_zip(zip_path: &Path, dest: &Path) -> Result<()> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    log::info!("解压 {} 个文件 ...", archive.len());

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        if entry.is_dir() {
            continue;
        }
        let normalized = entry.name().replace('\\', "/");
        let out_path = safe_join(dest, &normalized)
            .ok_or_else(|| eyre!("zip 内包含非法路径条目: {}", entry.name()))?;
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out = std::io::BufWriter::new(std::fs::File::create(&out_path)?);
        std::io::copy(&mut entry, &mut out)?;
        out.flush()?;
    }
    Ok(())
}

/// 解压 zip 并收集 `lday` 目录。
fn extract_and_discover(zip_path: &Path, extract_dir: &Path, source: &str) -> Result<Vec<PathBuf>> {
    if extract_dir.exists() {
        std::fs::remove_dir_all(extract_dir)?;
    }
    std::fs::create_dir_all(extract_dir)?;
    extract_zip(zip_path, extract_dir)?;
    log::info!("解压完成: {}", extract_dir.display());

    let dirs = find_lday_dirs(extract_dir);
    if dirs.is_empty() {
        bail!(
            "解压后未找到含 *.day 文件的 lday 目录，来源: {source}（解压根: {}）",
            extract_dir.display()
        );
    }
    for d in &dirs {
        log::info!("发现解析目录: {}", d.display());
    }
    Ok(dirs)
}

/// 安全拼接：只允许普通目录/文件名组件，拒绝 `..`、绝对路径、前缀等。
fn safe_join(base: &Path, name: &str) -> Option<PathBuf> {
    let mut out = PathBuf::from(base);
    for comp in Path::new(name).components() {
        match comp {
            Component::Normal(c) => out.push(c),
            _ => return None,
        }
    }
    Some(out)
}

/// 在解压根目录下收集所有名为 `lday` 的子目录（sh/lday、sz/lday、bj/lday ...）。
fn find_lday_dirs(root: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return dirs;
    };
    for e in entries.flatten() {
        let lday = e.path().join("lday");
        if lday.is_dir() {
            dirs.push(lday);
        }
    }
    dirs.sort();
    dirs
}

/// 按 URL 独立缓存目录（不同来源互不覆盖）。
fn cache_dir_for(url: &str) -> PathBuf {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut h);
    std::env::temp_dir()
        .join("rustdx_hsjday")
        .join(format!("{:016x}", h.finish()))
}

/// HEAD 探测服务器文件信息（ETag / Last-Modified / Content-Length）。
fn head_info(url: &str) -> Result<HeadInfo> {
    let resp = ureq::head(url).call().map_err(|e| eyre!("{e}"))?;
    let header = |name: &str| {
        resp.headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
    };
    Ok(HeadInfo {
        etag: header("etag"),
        last_modified: header("last-modified"),
        size: header("content-length").and_then(|s| s.parse::<u64>().ok()),
    })
}

/// 判断本地 zip 缓存是否仍与服务器一致（命中则跳过下载）。
fn zip_cached(url: &str, zip_path: &Path, meta: &Meta, head: Option<&HeadInfo>) -> bool {
    if !zip_path.exists() || !meta.downloaded_ok || meta.url != url {
        return false;
    }
    let Some(head) = head else {
        // 网络探测失败：信任已有下载（避免断网时重复下载报错）
        return true;
    };
    let size_ok = head.size.map(|s| meta.size == s).unwrap_or(true);
    let id_ok = match (&meta.etag, &head.etag) {
        (Some(a), Some(b)) => a == b,
        _ => match (&meta.last_modified, &head.last_modified) {
            (Some(a), Some(b)) => a == b,
            _ => false,
        },
    };
    size_ok && (id_ok || (head.etag.is_none() && head.last_modified.is_none()))
}

fn read_meta(path: &Path) -> Result<Meta> {
    if !path.exists() {
        return Ok(Meta::default());
    }
    let s = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&s).unwrap_or_default())
}

fn write_meta(path: &Path, meta: &Meta) -> Result<()> {
    std::fs::write(path, serde_json::to_string_pretty(meta)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_join_rejects_traversal() {
        let base = Path::new("/tmp/base");
        assert!(safe_join(base, "sh/lday/sh000001.day").is_some());
        assert!(safe_join(base, "../evil").is_none());
        assert!(safe_join(base, "/abs").is_none());
        assert!(safe_join(base, "a/../../b").is_none());
    }

    #[test]
    fn find_lday_dirs_empty_when_missing() {
        assert!(find_lday_dirs(Path::new("/nonexistent_dir_xyz")).is_empty());
    }

    #[test]
    fn zip_cached_logic() {
        let zip = Path::new("/tmp/rustdx_hsjday_test/hsjday.zip");
        // 无 zip → 不命中
        let m = Meta {
            url: "https://x/hsjday.zip".into(),
            downloaded_ok: true,
            size: 100,
            ..Default::default()
        };
        assert!(!zip_cached("https://x/hsjday.zip", zip, &m, None));
        // 同 etag → 命中
        std::fs::create_dir_all("/tmp/rustdx_hsjday_test").unwrap();
        std::fs::write(zip, b"x").unwrap();
        let m2 = Meta {
            url: "https://x/hsjday.zip".into(),
            etag: Some("\"abc\"".into()),
            downloaded_ok: true,
            size: 1,
            ..Default::default()
        };
        let h = HeadInfo {
            etag: Some("\"abc\"".into()),
            last_modified: None,
            size: Some(1),
        };
        assert!(zip_cached("https://x/hsjday.zip", zip, &m2, Some(&h)));
        // etag 变了 → 不命中
        let h2 = HeadInfo {
            etag: Some("\"def\"".into()),
            last_modified: None,
            size: Some(1),
        };
        assert!(!zip_cached("https://x/hsjday.zip", zip, &m2, Some(&h2)));
        // 不同 url → 不命中
        assert!(!zip_cached("https://y/hsjday.zip", zip, &m2, Some(&h)));
        let _ = std::fs::remove_dir_all("/tmp/rustdx_hsjday_test");
    }
}
