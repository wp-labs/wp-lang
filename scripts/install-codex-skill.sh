#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "Usage: $0 <skill-name> [codex-home]" >&2
  exit 2
fi

skill_name="$1"
codex_home="${2:-$HOME/.codex}"
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
src_dir="$repo_root/tools/skills/$skill_name"
dst_dir="$codex_home/skills/$skill_name"

if [[ ! -d "$src_dir" ]]; then
  echo "skill source not found: $src_dir" >&2
  exit 1
fi

mkdir -p "$(dirname "$dst_dir")"
rm -rf "$dst_dir"
cp -R "$src_dir" "$dst_dir"

echo "installed $skill_name -> $dst_dir"
