//! 目录扫描可复用骨架:Java / Maven / 未来 Gradle/Node 共用。
//!
//! 把「遍历候选根目录 → 校验是否有效安装 → 提取信息 → 去重」这套逻辑集中到一处,
//! 各工具只提供「候选路径表」「判定函数」「提取函数」。

use crate::core::presentation::ScanHit;
use crate::utils::path::normalize_path;
use std::collections::HashSet;
use std::path::Path;

/// 遍历候选根目录,收集有效安装。
///
/// 对每个 root:若 root 本身就是有效安装(`is_valid` 为真)直接收;否则 `read_dir`
/// 遍历其子目录,对每个子目录做同样校验。用 `normalize_path` + `HashSet` 按路径去重。
pub fn scan_directory_roots(
    roots: &[String],
    is_valid: impl Fn(&Path) -> bool,
    make_hit: impl Fn(&Path) -> Result<ScanHit, String>,
) -> Vec<ScanHit> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut out = Vec::new();

    for root in roots {
        let root_path = Path::new(root);
        // root 自身有效 → 直接作为候选;否则遍历子目录
        let candidates: Vec<std::path::PathBuf> = if is_valid(root_path) {
            vec![root_path.to_path_buf()]
        } else if let Ok(entries) = std::fs::read_dir(root_path) {
            entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect()
        } else {
            Vec::new()
        };

        for cand in candidates {
            if !is_valid(&cand) {
                continue;
            }
            let path_str = cand.to_string_lossy().to_string();
            if seen.insert(normalize_path(&path_str)) {
                if let Ok(hit) = make_hit(&cand) {
                    out.push(hit);
                }
            }
        }
    }

    out
}
