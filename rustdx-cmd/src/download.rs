//! 通达信日线完整包（hsjday.zip）的自动下载、解压与解析目录发现。
//!
//! 供 `rustdx day`（无目录参数）使用：
//! 1. 从默认地址 <https://data.tdx.com.cn/vipdoc/hsjday.zip> 下载完整包；
//! 2. 下载失败时自动重试，仍失败则交互询问用户提供备选下载地址（URL）或
//!    本地 zip 文件路径；
//! 3. 断点续传：数据先写入 `hsjday.zip.part`，中断后重跑且服务器文件未变化时
//!    从已下载位置继续（服务器支持 `Accept-Ranges`），完成后原子改名为 zip；
//! 4. 缓存跳过：按 ETag/Last-Modified/大小比对服务器文件，未变化则跳过
//!    下载与解压，二次运行接近秒开；
//! 5. 解压后收集 `sh/lday`、`sz/lday`、`bj/lday` 等目录，返回给解析流程。

use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use eyre::{Result, bail, eyre};
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
        log::info!(
            "缓存命中：服务器文件未变化，跳过下载 {}",
            zip_path.display()
        );
    } else {
        download_with_fallback(url, &zip_path, head.as_ref())?;
        let zip_len = std::fs::metadata(&zip_path)?.len();
        meta = Meta {
            url: url.to_string(),
            etag: head.as_ref().and_then(|h| h.etag.clone()),
            last_modified: head.as_ref().and_then(|h| h.last_modified.clone()),
            size: zip_len,
            downloaded_ok: true,
            extracted_ok: false,
            downloaded_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
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
fn download_with_fallback(
    initial_url: &str,
    zip_path: &Path,
    head: Option<&HeadInfo>,
) -> Result<()> {
    let mut last_err: Option<eyre::Report> = None;
    for attempt in 1..=3 {
        match download(initial_url, zip_path, head) {
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
        last_err
            .as_ref()
            .map(|e| format!("{e:#}"))
            .unwrap_or_default()
    );

    let fallback = prompt_fallback()?;
    if fallback.starts_with("http://") || fallback.starts_with("https://") {
        // 备选地址没有 HEAD 信息 → 身份不可比对 → 一律从头下载
        download(&fallback, zip_path, None).map_err(|e2| eyre!("备选地址下载失败: {e2:#}"))?;
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

/// 半截下载（`.part`）的来源身份，写入 sidecar，用于判断能否断点续传。
#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct SourceId {
    url: String,
    etag: Option<String>,
    last_modified: Option<String>,
    size: Option<u64>,
}

impl SourceId {
    fn new(url: &str, head: Option<&HeadInfo>) -> Self {
        Self {
            url: url.to_string(),
            etag: head.and_then(|h| h.etag.clone()),
            last_modified: head.and_then(|h| h.last_modified.clone()),
            size: head.and_then(|h| h.size),
        }
    }
}

/// 计算续传起点：返回 0 表示必须丢弃 `.part` 从头下载。
///
/// 只有 sidecar 记录的身份与本次完全一致、且身份里至少有 etag 或
/// last-modified（否则无法证明服务器文件没变）时才续传；`.part` 比服务器
/// 文件还大说明已损坏，同样从头下载。
fn resume_offset(saved: Option<&SourceId>, current: &SourceId, part_len: u64) -> u64 {
    let same = saved == Some(current);
    let identifiable = current.etag.is_some() || current.last_modified.is_some();
    let oversized = current.size.is_some_and(|s| part_len > s);
    if same && identifiable && !oversized {
        part_len
    } else {
        0
    }
}

/// 在文件名后追加后缀（`hsjday.zip` → `hsjday.zip.part`）。
fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut s = path.as_os_str().to_os_string();
    s.push(suffix);
    PathBuf::from(s)
}

/// 下载 url 到 dest。
///
/// 数据先写入 `<dest>.part`，成功后原子 `rename` 为 dest，因此 dest 要么是
/// 旧的完整文件，要么是新的完整文件；断点续传只针对 `.part`，且要求
/// `<dest>.part.id` 记录的来源身份与本次一致（见 [`resume_offset`]）。
fn download(url: &str, dest: &Path, head: Option<&HeadInfo>) -> Result<()> {
    let part = with_suffix(dest, ".part");
    let id_path = with_suffix(dest, ".part.id");
    let current = SourceId::new(url, head);

    let part_len = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
    let saved: Option<SourceId> = std::fs::read_to_string(&id_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok());
    let done = resume_offset(saved.as_ref(), &current, part_len);
    if done == 0 && part_len > 0 {
        log::info!("已有的半截下载与服务器文件不一致（或无法比对），从头下载");
        std::fs::remove_file(&part)?;
    }
    std::fs::write(&id_path, serde_json::to_string(&current)?)?;

    let total = if done > 0 && current.size == Some(done) {
        // 上次已下完但没来得及改名：无需再请求（否则 Range 越界必得 416）
        done
    } else {
        match fetch(url, &part, done)? {
            Some(total) => total,
            None => {
                log::warn!("服务器返回 416（续传位置无效），删除半截文件从头下载");
                std::fs::remove_file(&part)?;
                fetch(url, &part, 0)?.ok_or_else(|| eyre!("HTTP 416"))?
            }
        }
    };

    if let Some(expect) = current.size
        && total != expect
    {
        // 保留 .part：字节数不足时下次可续传，超出时下次会被判定为损坏而重下
        bail!("下载不完整: 期望 {expect} 字节，实际 {total} 字节");
    }

    std::fs::rename(&part, dest)?;
    let _ = std::fs::remove_file(&id_path);
    log::info!(
        "下载完成: {} ({:.1} MiB)",
        dest.display(),
        total as f64 / (1024.0 * 1024.0)
    );
    Ok(())
}

/// 把 url 的内容写入 part（`done > 0` 时带 Range 续传），返回文件总字节数。
///
/// 返回 `Ok(None)` 表示服务器对续传请求回了 416，调用方应丢弃 part 重下。
fn fetch(url: &str, part: &Path, done: u64) -> Result<Option<u64>> {
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
    let mut resp = match req.call() {
        Ok(resp) => resp,
        Err(ureq::Error::StatusCode(416)) if done > 0 => return Ok(None),
        Err(e) => return Err(eyre!("{e}")),
    };

    let mut start = 0u64;
    match resp.status().as_u16() {
        200 if done > 0 => {
            // 服务器不支持 Range：从头下载
            log::warn!("服务器返回 200（不支持断点续传），从头下载");
        }
        206 if done > 0 => start = done,
        200 => {}
        416 if done > 0 => return Ok(None),
        s => {
            bail!("HTTP {s}");
        }
    }

    let mut out = if start > 0 {
        std::fs::OpenOptions::new().append(true).open(part)?
    } else {
        std::fs::File::create(part)?
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
    Ok(Some(total))
}

/// 下载失败后的交互：询问用户提供备选 URL 或本地 zip 文件路径。
fn prompt_fallback() -> Result<String> {
    eprintln!("下载失败。请提供备选下载地址（URL）或本地 zip 文件路径（直接回车退出）：");
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

    fn sid(etag: Option<&str>, lm: Option<&str>, size: Option<u64>) -> SourceId {
        SourceId {
            url: "https://x/hsjday.zip".into(),
            etag: etag.map(Into::into),
            last_modified: lm.map(Into::into),
            size,
        }
    }

    #[test]
    fn resume_offset_logic() {
        let cur = sid(Some("\"abc\""), None, Some(100));
        // 身份一致 → 续传
        assert_eq!(resume_offset(Some(&cur), &cur, 40), 40);
        // 已下完未改名 → 仍返回完整长度
        assert_eq!(resume_offset(Some(&cur), &cur, 100), 100);
        // 无 sidecar → 从头
        assert_eq!(resume_offset(None, &cur, 40), 0);
        // etag 变了（服务器文件已更新）→ 从头
        let old = sid(Some("\"old\""), None, Some(100));
        assert_eq!(resume_offset(Some(&old), &cur, 40), 0);
        // 大小变了 → 从头
        let other_size = sid(Some("\"abc\""), None, Some(90));
        assert_eq!(resume_offset(Some(&other_size), &cur, 40), 0);
        // url 不同 → 从头
        let mut other_url = sid(Some("\"abc\""), None, Some(100));
        other_url.url = "https://y/hsjday.zip".into();
        assert_eq!(resume_offset(Some(&other_url), &cur, 40), 0);
        // .part 比服务器文件还大 → 从头
        assert_eq!(resume_offset(Some(&cur), &cur, 101), 0);
        // 仅 last-modified 一致 → 续传
        let lm = sid(None, Some("Mon, 16 Nov 2026 10:00:00 GMT"), None);
        assert_eq!(resume_offset(Some(&lm), &lm, 40), 40);
        // 既无 etag 也无 last-modified（如备选 URL 无 HEAD 信息）→ 从头
        let anon = sid(None, None, Some(100));
        assert_eq!(resume_offset(Some(&anon), &anon, 40), 0);
        let no_head = SourceId::new("https://x/hsjday.zip", None);
        assert_eq!(resume_offset(Some(&no_head), &no_head, 40), 0);
    }

    #[test]
    fn with_suffix_appends_to_file_name() {
        let p = Path::new("/tmp/a/hsjday.zip");
        assert_eq!(with_suffix(p, ".part"), Path::new("/tmp/a/hsjday.zip.part"));
        assert_eq!(
            with_suffix(p, ".part.id"),
            Path::new("/tmp/a/hsjday.zip.part.id")
        );
    }
}
