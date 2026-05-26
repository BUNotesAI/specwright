#!/usr/bin/env bash
set -euo pipefail

SKILL_DIR="${AGENT_SPEC_SKILL_DIR:-${HOME}/.claude/skills}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
INSTALL_CLI="${AGENT_SPEC_INSTALL_CLI:-1}"

echo "=== agent-spec skills installer ==="
echo

# Step 1: Install CLI from this checkout
if [[ "${INSTALL_CLI}" != "0" ]]; then
  echo "[..] Installing agent-spec CLI from ${SCRIPT_DIR}..."
  if command -v cargo &>/dev/null; then
    cargo install --path "${SCRIPT_DIR}" --force
    CURRENT=$(agent-spec --version 2>/dev/null || echo "unknown")
    echo "[ok] agent-spec CLI installed: ${CURRENT}"
  else
    echo "[!!] cargo not found. Install Rust first: https://rustup.rs"
    echo "     Then run: cargo install --path ${SCRIPT_DIR} --force"
    exit 1
  fi
else
  echo "[skip] CLI install skipped because AGENT_SPEC_INSTALL_CLI=0"
fi

echo

# Step 2: Install skills
mkdir -p "${SKILL_DIR}"

for skill in agent-spec-tool-first agent-spec-authoring agent-spec-estimate; do
  SRC="${SCRIPT_DIR}/skills/${skill}"
  DST="${SKILL_DIR}/${skill}"

  if [ ! -d "${SRC}" ]; then
    echo "[skip] ${skill} — not found in ${SCRIPT_DIR}/skills/"
    continue
  fi

  # Copy (overwrite) to ensure latest version
  rm -rf "${DST}"
  cp -r "${SRC}" "${DST}"
  echo "[ok] ${skill} → ${DST}"
done

echo
echo "Done. All agent-spec skills are ready."
echo "Skills target: ${SKILL_DIR}"
echo "Verify with: ls ${SKILL_DIR}/agent-spec-*"
