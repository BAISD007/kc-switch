use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

use super::error::{KcError, Result};
use super::fs_ops;

/// 进程内写锁：防止 GUI 连点导致的并发写。
static WRITE_LOCK: Mutex<()> = Mutex::new(());

pub struct WriteGuard {
    _guard: std::sync::MutexGuard<'static, ()>,
}

/// 获取写锁并做一次 mtime 快照，供 guard 释放前比对。
pub fn lock() -> WriteGuard {
    let guard = WRITE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    WriteGuard { _guard: guard }
}

pub fn mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

/// 读取并解析 TOML；同时返回读到的 mtime，用于写前冲突检测。
pub fn read_tracked(path: &Path) -> Result<(String, Option<SystemTime>)> {
    Ok((fs_ops::read_to_string(path)?, mtime(path)))
}

/// 写回前确认文件没有被外部（Kimi Code 自身）改过。
/// 若 mtime 与读取时不同，返回冲突错误，让调用方重读后再改。
pub fn write_if_unchanged(
    path: &Path,
    expected: Option<SystemTime>,
    contents: &[u8],
) -> Result<()> {
    let actual = mtime(path);
    if expected.is_some() && actual != expected {
        return Err(KcError::Invalid(
            "配置文件在读取后被外部修改（可能是 Kimi Code 正在写入），请刷新后重试".into(),
        ));
    }
    fs_ops::safe_write(path, contents)
}

/// 本项目自己的路径集合，供 UI 展示与冲突提示使用。
pub fn describe_all() -> Result<Vec<(String, PathBuf)>> {
    Ok(vec![
        ("config.toml".into(), super::paths::config_file()?),
        ("mcp.json".into(), super::paths::mcp_file()?),
        ("installed.json".into(), super::paths::plugins_file()?),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::test_support::lock_data_dir;

    #[test]
    fn write_if_unchanged_detects_external_edit() {
        let _data = lock_data_dir();
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("config.toml");
        std::fs::write(&target, "a = 1\n").unwrap();

        let (_, snapshot) = read_tracked(&target).unwrap();
        // 模拟外部修改：推到未来时间，保证 mtime 变化可观测
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&target, "a = 2\n").unwrap();

        let err = write_if_unchanged(&target, snapshot, b"a = 3\n");
        assert!(err.is_err(), "外部修改后应拒绝写入");

        // 重新读取后可以正常写入
        let (_, snapshot) = read_tracked(&target).unwrap();
        write_if_unchanged(&target, snapshot, b"a = 3\n").unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "a = 3\n");
    }
}
