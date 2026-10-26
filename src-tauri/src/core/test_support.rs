use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

/// 测试并行时会互相覆盖 KC_SWITCH_DATA_DIR，所有经过 safe_write 的测试都要拿这把锁。
static ENV_LOCK: Mutex<()> = Mutex::new(());

pub struct DataDirGuard {
    _guard: MutexGuard<'static, ()>,
    tmp: tempfile::TempDir,
}

impl DataDirGuard {
    pub fn path(&self) -> PathBuf {
        self.tmp.path().to_path_buf()
    }
}

impl Drop for DataDirGuard {
    fn drop(&mut self) {
        std::env::remove_var("KC_SWITCH_DATA_DIR");
    }
}

pub fn lock_data_dir() -> DataDirGuard {
    let guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let tmp = tempfile::tempdir().expect("temp data dir");
    std::env::set_var("KC_SWITCH_DATA_DIR", tmp.path());
    DataDirGuard {
        _guard: guard,
        tmp,
    }
}

pub fn with_data_dir<T>(f: impl FnOnce(&std::path::Path) -> T) -> T {
    let guard = lock_data_dir();
    f(guard.tmp.path())
}
