# jj VCS Integration — Implementation Guide

> Companion to `task-jj-vcs-integration.spec`
> Total scope: ~120 lines of new Rust code, 1 new file, 1 modified file, 0 new dependencies

---

## Overview

All work happens in two files inside `crates/spec-cli/`:

```
crates/spec-cli/src/
├── main.rs    ← modify: wire VCS context into stamp, lifecycle, explain
└── vcs.rs     ← new: VCS detection, context retrieval, diff between operations
```

No other crate is touched. No new Cargo dependencies.

---

## Step 1: Create `crates/spec-cli/src/vcs.rs`

This is the entire VCS abstraction layer. Every jj interaction is a
`Command::new("jj")` call. Every function returns `Option` or silently
degrades—no panics, no hard errors if jj is missing.

```rust
//! VCS context detection and retrieval.
//!
//! Supports Git and jj. Detection is automatic based on directory
//! markers (.git / .jj). All jj interaction is through CLI calls—
//! no jj-lib dependency.

use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};

// ── Types ───────────────────────────────────────────────────────

/// Detected version control system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VcsType {
    Git,
    Jj,
    None,
}

/// Snapshot of VCS state at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VcsContext {
    pub vcs_type: VcsType,
    /// Git: short commit hash. jj: short change ID.
    pub change_ref: String,
    /// jj only: short operation ID. None for Git.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_ref: Option<String>,
}

// ── Detection ───────────────────────────────────────────────────

/// Auto-detect VCS type from the current working directory.
/// Prefers jj when colocated (both .jj and .git exist).
pub fn detect_vcs_type(repo_root: &Path) -> VcsType {
    // jj colocate creates both .jj/ and .git/, so check jj first
    if repo_root.join(".jj").exists() {
        VcsType::Jj
    } else if repo_root.join(".git").exists() {
        VcsType::Git
    } else {
        VcsType::None
    }
}

// ── Context Retrieval ───────────────────────────────────────────

/// Get the current VCS context. Returns None if no VCS is detected
/// or if the necessary CLI commands fail.
pub fn get_vcs_context(repo_root: &Path) -> Option<VcsContext> {
    match detect_vcs_type(repo_root) {
        VcsType::Jj => get_jj_context(repo_root),
        VcsType::Git => get_git_context(repo_root),
        VcsType::None => None,
    }
}

fn get_jj_context(repo_root: &Path) -> Option<VcsContext> {
    let change_ref = run_jj(repo_root, &[
        "log", "-r", "@", "--no-graph", "-T", "change_id.short()",
    ])?;
    let operation_ref = run_jj(repo_root, &[
        "op", "log", "--limit", "1", "--no-graph", "-T", "self.id().short()",
    ]);
    Some(VcsContext {
        vcs_type: VcsType::Jj,
        change_ref,
        operation_ref,
    })
}

fn get_git_context(repo_root: &Path) -> Option<VcsContext> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let change_ref = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if change_ref.is_empty() {
        return None;
    }
    Some(VcsContext {
        vcs_type: VcsType::Git,
        change_ref,
        operation_ref: None,
    })
}

// ── Cross-Run Diff ──────────────────────────────────────────────

/// Get the list of files changed between two jj operations.
/// Returns None if jj is unavailable or the command fails.
pub fn jj_diff_between_ops(repo_root: &Path, from_op: &str, to_op: &str) -> Option<Vec<String>> {
    // jj op diff is not yet stable in all versions.
    // Fall back to jj diff with revisions if needed.
    let output = run_jj(repo_root, &[
        "diff", "--name-only", "--from", from_op, "--to", to_op,
    ])?;
    let files: Vec<String> = output
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();
    if files.is_empty() { None } else { Some(files) }
}

// ── Helpers ─────────────────────────────────────────────────────

/// Run a jj command and return trimmed stdout, or None on failure.
fn run_jj(repo_root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("jj")
        .current_dir(repo_root)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() { None } else { Some(text) }
}
```

This is ~100 lines including doc comments. Every function is fallible
and returns `Option`. If jj is not installed, every call returns `None`
and the caller continues with its existing Git-or-nothing behaviour.

---

## Step 2: Wire VCS context into `main.rs`

Three touch points in main.rs, each small and isolated.

### 2a. Declare the module

At the top of main.rs, add:

```rust
mod vcs;
```

### 2b. Enhance `build_stamp_trailers`

Current signature:

```rust
fn build_stamp_trailers(
    name: &str,
    passing: bool,
    summary: &spec_core::VerificationSummary,
) -> Vec<String>
```

New signature adds an optional VCS context:

```rust
fn build_stamp_trailers(
    name: &str,
    passing: bool,
    summary: &spec_core::VerificationSummary,
    vcs_ctx: Option<&vcs::VcsContext>,
) -> Vec<String> {
    let mut trailers = vec![
        format!("Spec-Name: {name}"),
        format!("Spec-Passing: {passing}"),
        format!(
            "Spec-Summary: {}/{} passed, {} failed, {} skipped, {} uncertain",
            summary.passed, summary.total, summary.failed,
            summary.skipped, summary.uncertain,
        ),
    ];

    // Append jj change ID when available
    if let Some(ctx) = vcs_ctx {
        if ctx.vcs_type == vcs::VcsType::Jj {
            trailers.push(format!("Spec-Change: {}", ctx.change_ref));
        }
    }

    trailers
}
```

In `cmd_stamp`, before calling `build_stamp_trailers`:

```rust
let vcs_ctx = vcs::get_vcs_context(code);
let trailers = build_stamp_trailers(
    &contract.name, passing, &report.summary, vcs_ctx.as_ref(),
);
```

### 2c. Enhance `RunLogEntry`

Add a VCS field:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RunLogEntry {
    pub spec_name: String,
    pub passing: bool,
    pub summary: String,
    pub timestamp: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vcs: Option<vcs::VcsContext>,
}
```

In `cmd_lifecycle`, when building the entry:

```rust
let vcs_ctx = vcs::get_vcs_context(code);

let entry = RunLogEntry {
    spec_name: contract.name.clone(),
    passing,
    summary: format!(...),
    timestamp: ...,
    vcs: vcs_ctx,
};
```

This is fully backward-compatible: old JSON files without the `vcs` field
will deserialise with `vcs: None` thanks to `#[serde(default)]`.

### 2d. Enhance `read_run_log_history`

After sorting the log entries by timestamp, when formatting adjacent
entries, check if both have jj operation IDs and attempt a diff:

```rust
// Inside the formatting loop, between adjacent entries:
if let (Some(prev_vcs), Some(curr_vcs)) = (&prev_entry.vcs, &current_entry.vcs) {
    if prev_vcs.vcs_type == vcs::VcsType::Jj
        && curr_vcs.vcs_type == vcs::VcsType::Jj
    {
        if let (Some(prev_op), Some(curr_op)) =
            (&prev_vcs.operation_ref, &curr_vcs.operation_ref)
        {
            if let Some(changed_files) =
                vcs::jj_diff_between_ops(Path::new("."), prev_op, curr_op)
            {
                out.push_str("    Changes between runs:\n");
                for f in &changed_files {
                    out.push_str(&format!("      - {f}\n"));
                }
            }
        }
    }
}
```

If jj is unavailable or the diff command fails, `jj_diff_between_ops`
returns `None` and this block is simply skipped. No error, no degradation.

---

## Step 3: Update existing tests and add new ones

All new tests go in `main.rs`'s existing `#[cfg(test)] mod tests` block,
plus unit tests inside `vcs.rs`.

### Tests inside `vcs.rs`

These test the VCS detection and context logic directly:

- `test_vcs_detect_prefers_jj_when_colocated`: create temp dir with both `.git` and `.jj`, assert `Jj`
- `test_vcs_detect_returns_git_when_only_git`: create temp dir with `.git` only, assert `Git`
- `test_vcs_detect_returns_none_outside_repo`: empty temp dir, assert `None`
- `test_vcs_context_returns_jj_ids`: conditional on `jj` being available (like the existing jj test)
- `test_vcs_context_returns_git_hash`: create temp git repo with one commit

### Tests inside `main.rs` tests module

These test the integration points:

- `test_stamp_trailers_include_jj_change_id`: construct a `VcsContext` with `Jj` type, call `build_stamp_trailers`, assert `Spec-Change:` trailer exists
- `test_stamp_trailers_omit_change_id_for_git`: construct a `VcsContext` with `Git` type, assert no `Spec-Change:` trailer
- `test_run_log_entry_serialises_vcs_context`: round-trip a `RunLogEntry` with VCS through serde_json
- `test_run_log_entry_without_vcs_is_backward_compatible`: deserialise old-format JSON (no vcs field), assert `vcs` is `None`
- `test_explain_history_shows_jj_diff_between_runs`: write two run logs with jj ops, mock or conditional jj call
- `test_explain_history_degrades_without_jj`: same setup but ensure no jj on PATH, assert no crash

The existing test `test_resolve_command_change_paths_reads_jj_changes`
already covers the `--change-scope jj` path, so no change needed there.

---

## Execution Estimate

```
vcs.rs (new file):           ~100 lines
main.rs modifications:       ~30 lines (stamp + RunLogEntry + history)
new tests:                   ~150 lines
total:                       ~280 lines

risk:                        low
  - no external dependencies added
  - all new code paths degrade to None/silent fallback
  - existing tests unaffected (new VCS field is optional/serde-default)

estimated effort:            6-8 agent rounds if using agent-spec contract-driven flow
```

---

## Verification

After implementation, the full contract can be verified with:

```bash
agent-spec lifecycle specs/task-jj-vcs-integration.spec \
  --code . \
  --change-scope jj \
  --format json
```

All 12 scenarios should pass. The contract itself can serve as the
first real-world test of the jj integration: lifecycle runs in a jj
repo, the run log records a jj VcsContext, and stamp outputs a
`Spec-Change` trailer.
