use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::error::{KcError, Result};
use super::paths;

const MAX_BACKUPS: usize = 20;

fn stamp() -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    // 毫秒不够用时追加自增序号，保证同毫秒内的多次写入不会互相覆盖
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    format!("{}-{:03}-{:04}", now.as_secs(), now.subsec_millis(), seq)
}

/// 写前备份：把目标文件复制到 backups 目录，文件名带时间戳。
pub fn backup_file(target: &Path) -> Result<Option<PathBuf>> {
    if !target.exists() {
        return Ok(None);
    }
    let dir = paths::backups_dir()?;
    fs::create_dir_all(&dir)?;
    let name = target
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".into());
    let dest = dir.join(format!("{name}.{}.bak", stamp()));
    fs::copy(target, &dest)?;
    prune_backups(&dir, &name)?;
    Ok(Some(dest))
}

fn prune_backups(dir: &Path, name: &str) -> Result<()> {
    let prefix = format!("{name}.");
    let mut entries: Vec<(SystemTime, PathBuf)> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with(&prefix)
        })
        .filter_map(|e| {
            let md = e.metadata().ok()?;
            Some((md.modified().ok()?, e.path()))
        })
        .collect();
    if entries.len() <= MAX_BACKUPS {
        return Ok(());
    }
    entries.sort_by_key(|(t, _)| *t);
    let remove_count = entries.len() - MAX_BACKUPS;
    for (_, p) in entries.into_iter().take(remove_count) {
        let _ = fs::remove_file(p);
    }
    Ok(())
}

/// 原子写：先写同目录临时文件，再 rename 覆盖，避免半截文件。
pub fn atomic_write(target: &Path, contents: &[u8]) -> Result<()> {
    let parent = target
        .parent()
        .ok_or_else(|| KcError::Invalid("目标路径没有父目录".into()))?;
    fs::create_dir_all(parent)?;
    let tmp = parent.join(format!(
        ".{}.tmp-{}",
        target
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".into()),
        stamp()
    ));
    fs::write(&tmp, contents)?;
    if let Err(e) = fs::rename(&tmp, target) {
        let _ = fs::remove_file(&tmp);
        return Err(KcError::Io(e.to_string()));
    }
    Ok(())
}

pub fn read_to_string(target: &Path) -> Result<String> {
    if !target.exists() {
        return Ok(String::new());
    }
    Ok(fs::read_to_string(target)?)
}

/// 备份 + 原子写一次性完成。
pub fn safe_write(target: &Path, contents: &[u8]) -> Result<()> {
    backup_file(target)?;
    atomic_write(target, contents)
}

/// 移动到回收站（而不是直接删除）。
pub fn move_to_trash(source: &Path) -> Result<PathBuf> {
    let dir = paths::trash_dir()?;
    fs::create_dir_all(&dir)?;
    let name = source
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "item".into());
    let dest = dir.join(format!("{name}.{}", stamp()));
    fs::rename(source, &dest).or_else(|_| {
        copy_dir_all(source, &dest)?;
        fs::remove_dir_all(source)?;
        Ok::<(), std::io::Error>(())
    })?;
    Ok(dest)
}

pub fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &to)?;
        } else {
            fs::copy(entry.path(), &to)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::test_support::with_data_dir;

    #[test]
    fn atomic_write_creates_file() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("a.txt");
        atomic_write(&target, b"hello").unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "hello");
    }

    #[test]
    fn safe_write_backs_up_previous() {
        with_data_dir(|_| {
            let tmp = tempfile::tempdir().unwrap();
            let target = tmp.path().join("config.toml");
            fs::write(&target, "old").unwrap();
            safe_write(&target, b"new").unwrap();
            assert_eq!(fs::read_to_string(&target).unwrap(), "new");
            let backups = fs::read_dir(paths::backups_dir().unwrap())
                .unwrap()
                .count();
            assert_eq!(backups, 1);
        });
    }

    #[test]
    fn move_to_trash_preserves_content() {
        with_data_dir(|_| {
            let tmp = tempfile::tempdir().unwrap();
            let src = tmp.path().join("thing");
            fs::create_dir_all(&src).unwrap();
            fs::write(src.join("x.md"), "content").unwrap();
            let dest = move_to_trash(&src).unwrap();
            assert!(!src.exists());
            assert_eq!(fs::read_to_string(dest.join("x.md")).unwrap(), "content");
        });
    }
}
