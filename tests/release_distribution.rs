//! Static contract tests for the GitHub Release distribution artifacts.

use std::fs;
use std::path::PathBuf;

fn read_repo_file(path: &str) -> String {
    let full_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path);
    fs::read_to_string(&full_path)
        .unwrap_or_else(|error| panic!("{path} should exist and be readable: {error}"))
}

#[test]
fn release_workflow_covers_supported_targets() {
    let workflow = read_repo_file(".github/workflows/release.yml");

    for (target, runner) in [
        ("aarch64-apple-darwin", "macos-14"),
        ("x86_64-unknown-linux-musl", "ubuntu-22.04"),
        ("x86_64-unknown-linux-gnu", "ubuntu-22.04"),
        ("aarch64-unknown-linux-gnu", "ubuntu-22.04-arm"),
    ] {
        assert!(
            workflow.contains(&format!("target: {target}")),
            "release workflow should contain target {target}"
        );
        assert!(
            workflow.contains(&format!("os: {runner}")),
            "release workflow should run {target} on {runner}"
        );
    }

    assert!(workflow.contains("workflow_dispatch:"));
    assert!(workflow.contains("tags:"));
    assert!(workflow.contains("- \"v*\""));
}

#[test]
fn release_archives_are_single_binary_with_sha256() {
    let workflow = read_repo_file(".github/workflows/release.yml");

    for required in [
        "install -m 0755 \"$binary\" package/specwright",
        "tar -C package -czf \"dist/${archive}\" specwright",
        "tar -tzf \"dist/${archive}\"",
        "sha256sum \"$archive\"",
        "shasum -a 256 \"$archive\"",
        "dist/*.tar.gz",
        "dist/*.sha256",
    ] {
        assert!(
            workflow.contains(required),
            "release workflow should contain packaging contract: {required}"
        );
    }
}

#[test]
fn release_workflow_rejects_version_mismatches_and_overwrite_flags() {
    let workflow = read_repo_file(".github/workflows/release.yml");

    for required in [
        "cargo metadata --locked --no-deps --format-version 1",
        "test \"$GITHUB_REF_NAME\" = \"v${version}\"",
        "if: github.ref_type == 'tag'",
        "gh release create \"$TAG\"",
        "--verify-tag",
        "--fail-on-no-commits",
    ] {
        assert!(
            workflow.contains(required),
            "release workflow should contain publish safeguard: {required}"
        );
    }

    assert!(
        !workflow.contains("--clobber"),
        "release workflow must not overwrite published assets"
    );
}

#[test]
fn readme_documents_pinned_binary_and_cargo_installation() {
    let readme = read_repo_file("README.md");

    for required in [
        "releases/download/v2.1.1/specwright-aarch64-apple-darwin.tar.gz",
        "releases/download/v2.1.1/specwright-x86_64-unknown-linux-musl.tar.gz",
        "cargo install --git https://github.com/BUNotesAI/specwright --locked",
        "--tag v2.1.1",
        "2.x",
    ] {
        assert!(
            readme.contains(required),
            "README installation contract should contain: {required}"
        );
    }

    let floating_release_url = ["releases", "latest", "download"].join("/");
    assert!(
        !readme.contains(&floating_release_url),
        "README automation must use an exact release tag"
    );
}

#[test]
fn release_workflow_runs_clean_install_without_rust() {
    let workflow = read_repo_file(".github/workflows/release.yml");

    for required in [
        "container: debian:bookworm-slim",
        "if command -v cargo >/dev/null 2>&1; then",
        "if command -v rustc >/dev/null 2>&1; then",
        "releases/download/${TAG}/specwright-x86_64-unknown-linux-musl.tar.gz",
        "tar -xz -C /usr/local/bin",
        "test \"$(specwright --version)\" = \"specwright ${VERSION}\"",
    ] {
        assert!(
            workflow.contains(required),
            "clean installation workflow should contain: {required}"
        );
    }
}
