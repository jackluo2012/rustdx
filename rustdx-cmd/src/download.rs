//! 通达信日线完整包（hsjday.zip）的自动下载、解压与解析目录发现。
//!
//! 供 `rustdx day`（无目录参数）使用：
//! 1. 从默认地址 <https://data.tdx.com.cn/vipdoc/hsjday.zip> 下载完整包；
//! 2. 下载失败时交互询问用户提供备选下载地址（URL）或本地 zip 文件路径；
//! 3. 解压后收集 `sh/lday`、`sz/lday`、`bj/lday` 等目录，返回给解析流程。

use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

use eyre::{bail, eyre, Result};

/// 通达信官网日线完整包默认地址。
pub const DEFAULT_HSJDAY_URL: &str = "https://data.tdx.com.cn/vipdoc/hsjday.zip";

/// 自动从默认地址下载并解压，返回解压出的 `lday` 目录列表。
/// 默认地址下载失败时，交互询问用户提供备选 URL 或本地 zip 路径。
pub fn prepare_hsjday_dirs() -> Result<Vec<PathBuf>> {
    prepare_hsjday_dirs_from(DEFAULT_HSJDAY_URL)
}

/// 从指定 URL 下载并解压；下载失败同样交互询问备选来源。
pub fn prepare_hsjday_dirs_from(url: &str) -> Result<Vec<PathBuf>> {
    let workdir = std::env::temp_dir().join("rustdx_hsjday");
    std::fs::create_dir_all(&workdir)?;
    let zip_path = workdir.join("hsjday.zip");

    download_with_fallback(url, &zip_path)?;
    extract_and_discover(&zip_path, &workdir.join("extracted"), url)
}

/// 直接解压本地 zip 文件，返回解压出的 `lday` 目录列表。
pub fn prepare_hsjday_dirs_from_zip(zip_path: &Path) -> Result<Vec<PathBuf>> {
    let workdir = std::env::temp_dir().join("rustdx_hsjday");
    std::fs::create_dir_all(&workdir)?;
    extract_and_discover(zip_path, &workdir.join("extracted"), &zip_path.display().to_string())
}

/// 下载（首次失败 → 交互提供备选 URL / 本地 zip 路径）。
fn download_with_fallback(initial_url: &str, zip_path: &Path) -> Result<()> {
    if let Err(e) = download(initial_url, zip_path) {
        log::warn!("下载失败: {e:#}");
        // 交互：用户可能给备选 URL，也可能给本地 zip 路径
        let fallback = prompt_fallback()?;
        if fallback.starts_with("http://") || fallback.starts_with("https://") {
            download(&fallback, zip_path).map_err(|e2| eyre!("备选地址下载失败: {e2:#}"))?;
        } else {
            let p = PathBuf::from(&fallback);
            if !p.is_file() {
                bail!("提供的文件不存在: {}", p.display());
            }
            // 本地 zip：直接复制到工作目录
            log::info!("使用本地 zip: {}", p.display());
            std::fs::copy(&p, zip_path)?;
        }
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

/// 下载 url 到 dest 文件（流式，每 32MiB 打印一次进度）。
fn download(url: &str, dest: &Path) -> Result<()> {
    log::info!("正在下载 {url} ...");
    let mut resp = ureq::get(url).call().map_err(|e| eyre!("{e}"))?;
    let mut reader = resp.body_mut().as_reader();
    let mut out = std::io::BufWriter::new(std::fs::File::create(dest)?);

    let mut buf = [0u8; 64 * 1024];
    let mut total: u64 = 0;
    let mut last_mark: u64 = 0;
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
            log::info!("已下载 {} MiB", mark * 32);
        }
    }
    out.flush()?;
    log::info!("下载完成: {} ({:.1} MiB)", dest.display(), total as f64 / 1024.0 / 1024.0);
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
        bail!(
            "未提供备选来源，已退出。可重跑：`rustdx day <下载地址或本地zip路径>`"
        );
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
}
