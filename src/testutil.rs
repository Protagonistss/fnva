//! 测试公用工具:把 `FNVA_HOME` 隔离到临时目录。
//!
//! `FNVA_HOME` 是进程级全局环境变量,依赖它的测试必须**串行**运行——
//! 否则并行 `set_var` 会互相覆盖。所有用到它的测试通过 [`FnvaHomeGuard`]
//! 共享同一把全局 Mutex,保证跨文件也串行。

use std::path::Path;
use std::sync::{Mutex, OnceLock};

static LOCK: OnceLock<Mutex<()>> = OnceLock::new();

/// RAII guard:构造时把 `FNVA_HOME` 指向 `dir`,drop 时还原。
/// 持有期间占用全局锁,使依赖 `FNVA_HOME` 的测试串行执行。
pub struct FnvaHomeGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
}

impl FnvaHomeGuard {
    pub fn new(dir: &Path) -> Self {
        let lock = LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
        std::env::set_var("FNVA_HOME", dir);
        Self { _lock: lock }
    }
}

impl Drop for FnvaHomeGuard {
    fn drop(&mut self) {
        std::env::remove_var("FNVA_HOME");
    }
}
