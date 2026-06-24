#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage:
  scripts/perf-guard.sh baseline [baseline-name]
  scripts/perf-guard.sh check [baseline-name]
  scripts/perf-guard.sh report [output-file]
  scripts/perf-guard.sh run

Environment:
  WP_PERF_GUARD_LOOPS         Parses per Criterion iteration. Default: 1000.
  WP_PERF_GUARD_SAMPLE_SIZE   Criterion sample size. Default: 30.
  WP_PERF_GUARD_WARMUP        Warm-up time in seconds. Default: 2.
  WP_PERF_GUARD_MEASURE       Measurement time in seconds. Default: 4.
  WP_PERF_GUARD_REPORT        Default report path. Default: target/perf_guard_report.md.

Examples:
  scripts/perf-guard.sh baseline main
  scripts/perf-guard.sh check main
  scripts/perf-guard.sh report
  WP_PERF_GUARD_LOOPS=2000 scripts/perf-guard.sh run
USAGE
}

mode="${1:-run}"
baseline="${2:-main}"

sample_size="${WP_PERF_GUARD_SAMPLE_SIZE:-30}"
warmup="${WP_PERF_GUARD_WARMUP:-2}"
measure="${WP_PERF_GUARD_MEASURE:-4}"

common_args=(
  bench
  --bench
  perf_guard
  --
  --sample-size
  "$sample_size"
  --warm-up-time
  "$warmup"
  --measurement-time
  "$measure"
)

case "$mode" in
  baseline)
    cargo "${common_args[@]}" --save-baseline "$baseline"
    ;;
  check)
    cargo "${common_args[@]}" --baseline "$baseline"
    ;;
  run)
    cargo "${common_args[@]}"
    ;;
  report)
    output="${2:-${WP_PERF_GUARD_REPORT:-target/perf_guard_report.md}}"
    node - "$output" <<'NODE'
const fs = require("fs");
const path = require("path");

const output = process.argv[2];
const root = path.join("target", "criterion", "perf_guard");

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, "utf8"));
}

function listDirs(dir) {
  if (!fs.existsSync(dir)) return [];
  return fs.readdirSync(dir)
    .map((name) => path.join(dir, name))
    .filter((item) => fs.statSync(item).isDirectory());
}

function newestRunDir(caseDir) {
  const dirs = listDirs(caseDir)
    .filter((dir) => fs.existsSync(path.join(dir, "new", "estimates.json")))
    .sort((a, b) => {
      const am = fs.statSync(path.join(a, "new", "estimates.json")).mtimeMs;
      const bm = fs.statSync(path.join(b, "new", "estimates.json")).mtimeMs;
      return bm - am;
    });
  return dirs[0];
}

function nsHuman(ns) {
  if (!Number.isFinite(ns)) return "";
  if (ns >= 1e6) return `${(ns / 1e6).toFixed(3)} ms`;
  if (ns >= 1e3) return `${(ns / 1e3).toFixed(3)} us`;
  return `${ns.toFixed(2)} ns`;
}

function pct(value) {
  if (!Number.isFinite(value)) return "";
  const signed = value >= 0 ? "+" : "";
  return `${signed}${(value * 100).toFixed(2)}%`;
}

function md(value) {
  return String(value ?? "").replaceAll("|", "\\|");
}

const rows = [];
for (const caseDir of listDirs(root)) {
  const caseName = path.basename(caseDir);
  if (caseName === "report") continue;
  const runDir = newestRunDir(caseDir);
  if (!runDir) continue;

  const loops = path.basename(runDir);
  const estimates = readJson(path.join(runDir, "new", "estimates.json"));
  const meanNs = estimates.mean?.point_estimate;
  const slopeNs = estimates.slope?.point_estimate;
  let change = "";
  let changeValue = Number.NaN;
  const changeFile = path.join(runDir, "change", "estimates.json");
  if (fs.existsSync(changeFile)) {
    const changeEst = readJson(changeFile);
    changeValue = changeEst.mean?.point_estimate;
    change = pct(changeValue);
  }

  rows.push({
    caseName,
    loops,
    meanNs,
    throughputValue: meanNs ? Number(loops) * 1e9 / meanNs : Number.NaN,
    changeValue,
    mean: nsHuman(meanNs),
    slope: nsHuman(slopeNs),
    throughput: meanNs ? `${(Number(loops) * 1e9 / meanNs).toFixed(0)} parse/s` : "",
    change,
  });
}

rows.sort((a, b) => a.caseName.localeCompare(b.caseName));

const rowsWithMean = rows.filter((row) => Number.isFinite(row.meanNs));
const fastest = [...rowsWithMean].sort((a, b) => b.throughputValue - a.throughputValue)[0];
const slowest = [...rowsWithMean].sort((a, b) => a.throughputValue - b.throughputValue)[0];
const changedRows = rows.filter((row) => Number.isFinite(row.changeValue));
const regressions = changedRows.filter((row) => row.changeValue > 0);
const improvements = changedRows.filter((row) => row.changeValue < 0);

const lines = [];
lines.push("# wp-lang Perf Guard Report");
lines.push("");
lines.push(`Generated: ${new Date().toISOString()}`);
lines.push("");
lines.push("## Summary");
lines.push("");
lines.push(`- Cases: ${rows.length}`);
if (fastest) {
  lines.push(`- Fastest: ${fastest.caseName} (${fastest.throughput})`);
}
if (slowest) {
  lines.push(`- Slowest: ${slowest.caseName} (${slowest.throughput})`);
}
if (changedRows.length > 0) {
  const worst = [...changedRows].sort((a, b) => b.changeValue - a.changeValue)[0];
  const best = [...changedRows].sort((a, b) => a.changeValue - b.changeValue)[0];
  lines.push(`- Baseline comparison: ${regressions.length} slower, ${improvements.length} faster, ${changedRows.length} compared`);
  lines.push(`- Worst change: ${worst.caseName} (${pct(worst.changeValue)})`);
  lines.push(`- Best change: ${best.caseName} (${pct(best.changeValue)})`);
} else {
  lines.push("- Baseline comparison: unavailable; run `scripts/perf-guard.sh check <baseline>` first.");
}
lines.push("");
lines.push("## Details");
lines.push("");
lines.push("| case | loops | mean | slope | throughput | change vs baseline |");
lines.push("| --- | ---: | ---: | ---: | ---: | ---: |");
for (const row of rows) {
  lines.push(`| ${md(row.caseName)} | ${md(row.loops)} | ${md(row.mean)} | ${md(row.slope)} | ${md(row.throughput)} | ${md(row.change)} |`);
}
lines.push("");
lines.push("Notes:");
lines.push("- Times are from Criterion `new/estimates.json`.");
lines.push("- `change vs baseline` is present after running `scripts/perf-guard.sh check <baseline>`.");
lines.push("- Full Criterion reports remain under `target/criterion/perf_guard/`.");
lines.push("");

fs.mkdirSync(path.dirname(output), { recursive: true });
fs.writeFileSync(output, lines.join("\n"));
console.log(output);
NODE
    ;;
  -h|--help|help)
    usage
    ;;
  *)
    usage >&2
    exit 2
    ;;
esac
