#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Docs language-policy guard (task_792a6b6c).
//!
//! CLAUDE.md and AGENTS.md are two entry points for the same instructions and
//! must stay byte-identical. After CJK keywords are hard-rejected, the docs
//! must not re-advertise CJK DSL parser-compatibility, yet must keep the
//! `cjk_allowed_paths` allowlist (descriptions and rejection-test inputs stay
//! Chinese).

use std::fs;
use std::path::PathBuf;

#[test]
fn test_claude_agents_md_synced_no_legacy_compat_line() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let claude = fs::read_to_string(repo.join("CLAUDE.md")).unwrap();
    let agents = fs::read_to_string(repo.join("AGENTS.md")).unwrap();

    assert_eq!(
        claude, agents,
        "CLAUDE.md and AGENTS.md must be byte-identical"
    );
    assert!(
        !claude.contains("Chinese DSL aliases remain parser-compatible for legacy specs"),
        "the legacy CJK parser-compatibility sentence must be removed"
    );
    assert!(
        claude.contains("cjk_allowed_paths"),
        "the Language Policy allowlist must be kept (descriptions stay Chinese)"
    );
}
