#!/usr/bin/env bash
set -euo pipefail

find_wp_lang_repo_root() {
  local start="$1"
  local dir
  dir="$(cd "$start" && pwd)"

  while [[ "$dir" != "/" ]]; do
    if [[ -f "$dir/Cargo.toml" ]] && grep -q '^name = "wp-lang"' "$dir/Cargo.toml"; then
      printf '%s\n' "$dir"
      return 0
    fi
    dir="$(dirname "$dir")"
  done

  return 1
}

if [[ $# -eq 0 ]]; then
  echo "Usage: $0 <wpl-check args...>" >&2
  exit 2
fi

if [[ -n "${WPL_CHECK_BIN:-}" ]]; then
  exec "$WPL_CHECK_BIN" "$@"
fi

if command -v wpl-check >/dev/null 2>&1; then
  exec wpl-check "$@"
fi

repo_root=""

if repo_root="$(find_wp_lang_repo_root "$PWD" 2>/dev/null)"; then
  :
elif repo_root="$(find_wp_lang_repo_root "$(dirname "${BASH_SOURCE[0]}")/../../.." 2>/dev/null)"; then
  :
elif [[ -n "${WPL_CHECK_MANIFEST_PATH:-}" ]] && [[ -f "${WPL_CHECK_MANIFEST_PATH:-}" ]]; then
  exec cargo run --manifest-path "$WPL_CHECK_MANIFEST_PATH" --features wpl-check-cli --bin wpl-check -- "$@"
fi

if [[ -n "$repo_root" ]]; then
  exec cargo run --manifest-path "$repo_root/Cargo.toml" --features wpl-check-cli --bin wpl-check -- "$@"
fi

cat >&2 <<'EOF'
wpl-check is not available.

Use one of the following:
  1. Set WPL_CHECK_BIN to an installed wpl-check executable
  2. Install the binary:
       cargo install wp-lang --features wpl-check-cli --bin wpl-check
  3. Set WPL_CHECK_MANIFEST_PATH to a local wp-lang Cargo.toml and rerun
EOF
exit 1
