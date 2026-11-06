//! cgroup v2 helpers for per-child resource enforcement.
//!
//! Layout:
//!   /sys/fs/cgroup/forkd/                  ← parent (created on first use)
//!     ├── child-1/                         ← leaf, one per child VM
//!     │   ├── memory.max                   ← limit in bytes
//!     │   ├── memory.swap.max              ← 0: the cap is the whole budget, never a swap allowance
//!     │   └── cgroup.procs                 ← child Firecracker pid lives here
//!     └── child-2/ ...
//!
//! Why a parent dir: operators can read `/sys/fs/cgroup/forkd/memory.current`
//! to see total memory used by all forkd children at once. Also gives us a
//! single place to set workload-wide caps later (cpu.max, io.max, pids.max).
//!
//! Requires cgroup v2 unified hierarchy (kernel 4.5+, default since Ubuntu 22.04).
//! The memory / cpu / io / pids controllers are enabled at the parent's
//! subtree_control on first use; only `memory.max` is actually written today
//! (in `place_child`), but enabling the rest up front lets later PRs add
//! `cpu.max` etc. without re-touching this file.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

const FORKD_PARENT: &str = "/sys/fs/cgroup/forkd";
const CGROUP_ROOT_SUBTREE: &str = "/sys/fs/cgroup/cgroup.subtree_control";

pub fn mib_to_bytes(mib: u64) -> u64 {
    mib.saturating_mul(1024 * 1024)
}

/// Make sure `/sys/fs/cgroup/forkd/` exists and has the memory controller
/// enabled for its children. Idempotent.
///
/// Returns an error if cgroup v2 isn't mounted or we lack privileges — the
/// caller (forkd-controller, forkd-cli) should surface this with context
/// (e.g. "run as root, or move forkd to a delegated cgroup").
pub fn ensure_parent() -> Result<PathBuf> {
    let parent = PathBuf::from(FORKD_PARENT);

    if !Path::new("/sys/fs/cgroup/cgroup.controllers").exists() {
        anyhow::bail!(
            "cgroup v2 not mounted at /sys/fs/cgroup — \
             enable unified hierarchy (kernel cgroup_v2=1, or boot with systemd.unified_cgroup_hierarchy=1)"
        );
    }

    // The root's subtree_control must include "memory" for any of our
    // sibling-of-root cgroups (forkd/, forkd-child-N/) to expose memory.*.
    let root_subtree = std::fs::read_to_string(CGROUP_ROOT_SUBTREE)
        .with_context(|| format!("read {CGROUP_ROOT_SUBTREE}"))?;
    if !root_subtree.split_whitespace().any(|c| c == "memory") {
        anyhow::bail!(
            "cgroup v2 root subtree_control does not include 'memory' — \
             this is unusual; check /sys/fs/cgroup/cgroup.subtree_control"
        );
    }

    if !parent.exists() {
        std::fs::create_dir(&parent)
            .with_context(|| format!("mkdir {} (need root or delegation?)", parent.display()))?;
    }

    // Enable every controller the project is *likely* to want, not just the
    // one place_child writes today (memory.max). Adding cpu.max / io.max /
    // pids.max in a future PR otherwise silently fails because the child
    // cgroup won't expose those files — see #159. The kernel ignores
    // unknown controllers in subtree_control (they just aren't listed in
    // /sys/fs/cgroup/cgroup.controllers), so writes for controllers that
    // aren't compiled in are no-ops, not errors.
    const WANT_CONTROLLERS: &[&str] = &["memory", "cpu", "io", "pids"];

    let parent_subtree = parent.join("cgroup.subtree_control");
    let cur = std::fs::read_to_string(&parent_subtree).unwrap_or_default();
    let enabled: std::collections::HashSet<&str> = cur.split_whitespace().collect();
    let missing: Vec<&&str> = WANT_CONTROLLERS
        .iter()
        .filter(|c| !enabled.contains(**c))
        .collect();
    if !missing.is_empty() {
        // subtree_control accepts multiple "+ctrl" tokens space-separated in
        // a single write. One syscall is cheaper and atomic relative to
        // concurrent readers.
        let plus = missing
            .iter()
            .map(|c| format!("+{c}"))
            .collect::<Vec<_>>()
            .join(" ");
        std::fs::write(&parent_subtree, &plus).with_context(|| {
            format!(
                "enable controllers ({}) on {} subtree_control",
                plus,
                parent.display()
            )
        })?;
    }
    Ok(parent)
}

/// Place a child VM's Firecracker process under
/// `/sys/fs/cgroup/forkd/<name>/` with the given memory limit.
///
/// Returns the path of the child cgroup so the caller can `cleanup` it
/// when the VM dies.
pub fn place_child(name: &str, pid: u32, memory_max_bytes: u64) -> Result<PathBuf> {
    let parent = ensure_parent()?;
    let child = parent.join(name);
    if !child.exists() {
        std::fs::create_dir(&child).with_context(|| format!("mkdir {}", child.display()))?;
    }
    write_memory_limits(&child, memory_max_bytes)?;
    std::fs::write(child.join("cgroup.procs"), pid.to_string())
        .with_context(|| format!("attach pid {pid} to {}", child.display()))?;
    Ok(child)
}

/// Write a child's memory ceiling, before its process is attached.
///
/// `memory.max` alone is not a ceiling: cgroup v2's `memory.swap.max` defaults to `max`, so a child at its cap swaps
/// without limit and pushes the host into swap instead of being reclaimed or OOM-killed inside its own cgroup. Measured
/// 5 Nov 2026 on a 16 GB host: two idle 4 GiB children held 2.3 and 2.9 GB in swap, 5.2 of the host's 7.6 GB swapped,
/// beside a protected service. Writing `memory.swap.max 0` makes `memory.max` the whole budget.
///
/// It is written before `cgroup.procs`, so the process never runs with swap allowed. A host built without swap
/// accounting has no `memory.swap.max`; there is no swap to bound there, so its absence is not an error.
pub fn write_memory_limits(child: &Path, memory_max_bytes: u64) -> Result<()> {
    std::fs::write(child.join("memory.max"), memory_max_bytes.to_string())
        .with_context(|| format!("set memory.max on {}", child.display()))?;
    let swap_max = child.join("memory.swap.max");
    if swap_max.exists() {
        std::fs::write(&swap_max, "0").with_context(|| format!("set memory.swap.max on {}", child.display()))?;
    }
    Ok(())
}

/// Remove a child cgroup. Best-effort: returns Ok even if rmdir fails
/// (the cgroup may still hold a zombie process; the kernel cleans up
/// eventually). Logs the error via tracing for observability.
pub fn cleanup(cgroup: &Path) {
    if let Err(e) = std::fs::remove_dir(cgroup) {
        tracing::warn!(path = %cgroup.display(), error = %e, "cgroup cleanup failed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mib_conversion() {
        assert_eq!(mib_to_bytes(1), 1024 * 1024);
        assert_eq!(mib_to_bytes(256), 256 * 1024 * 1024);
        assert_eq!(mib_to_bytes(0), 0);
    }

    #[test]
    fn memory_limits_close_swap_past_the_cap() {
        let dir = std::env::temp_dir().join(format!("forkd-cgroup-limits-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("memory.swap.max"), "max").unwrap();
        write_memory_limits(&dir, mib_to_bytes(768)).unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("memory.max")).unwrap(), (768u64 * 1024 * 1024).to_string());
        assert_eq!(std::fs::read_to_string(dir.join("memory.swap.max")).unwrap(), "0");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn memory_limits_without_swap_accounting_set_only_the_cap() {
        let dir = std::env::temp_dir().join(format!("forkd-cgroup-noswap-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        write_memory_limits(&dir, mib_to_bytes(256)).unwrap();
        assert!(dir.join("memory.max").exists());
        assert!(!dir.join("memory.swap.max").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn mib_saturates_instead_of_overflowing() {
        // 1 << 53 MiB → would overflow u64 if multiplied, must saturate
        let huge = u64::MAX / 1024;
        let _ = mib_to_bytes(huge); // doesn't panic
    }
}
