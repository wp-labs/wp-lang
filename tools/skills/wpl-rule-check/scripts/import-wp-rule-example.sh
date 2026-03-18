#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'EOF'
Usage:
  import-wp-rule-example.sh <wp-rule-repo> <example-name> [out-dir]
  import-wp-rule-example.sh <wp-rule-example-dir> [out-dir]

Examples:
  import-wp-rule-example.sh /path/to/wp-rule nginx_access
  import-wp-rule-example.sh /path/to/wp-rule/models/wpl/nginx_access /tmp/nginx_access

The script copies:
  parse.wpl  -> rule.wpl
  sample.dat -> sample.txt
EOF
}

if [[ $# -lt 1 || $# -gt 3 ]]; then
  usage
  exit 2
fi

src_arg="$1"
arg2="${2:-}"
arg3="${3:-}"

resolve_example_dir() {
  local path="$1"
  local maybe_name="$2"

  if [[ -f "$path/parse.wpl" ]]; then
    printf '%s\n' "$path"
    return 0
  fi

  if [[ -n "$maybe_name" && -f "$path/models/wpl/$maybe_name/parse.wpl" ]]; then
    printf '%s\n' "$path/models/wpl/$maybe_name"
    return 0
  fi

  return 1
}

example_dir=""
out_dir=""

if example_dir="$(resolve_example_dir "$src_arg" "$arg2" 2>/dev/null)"; then
  if [[ -f "$src_arg/parse.wpl" ]]; then
    out_dir="${arg2:-}"
  else
    out_dir="${arg3:-}"
  fi
else
  echo "could not locate a wp-rule example from: $src_arg ${arg2:+$arg2}" >&2
  usage
  exit 1
fi

parse_file="$example_dir/parse.wpl"
sample_file="$example_dir/sample.dat"

if [[ ! -f "$sample_file" ]]; then
  echo "missing sample.dat: $sample_file" >&2
  exit 1
fi

if [[ -z "$out_dir" ]]; then
  out_dir="$(mktemp -d "${TMPDIR:-/tmp}/wpl-rule-check.XXXXXX")"
else
  mkdir -p "$out_dir"
fi

cp "$parse_file" "$out_dir/rule.wpl"
cp "$sample_file" "$out_dir/sample.txt"

cat <<EOF
Imported wp-rule example:
  source: $example_dir
  output: $out_dir

Next commands:
  scripts/run-wpl-check.sh syntax "$out_dir"
  scripts/run-wpl-check.sh sample --rule "$out_dir"
EOF
