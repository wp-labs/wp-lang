# Changelog
## [0.3.3] - 2026-05-26

### Fixed
- **UTF-8 handling in `copy_raw`**: `RawCopy` annotation function now uses `String::from_utf8_lossy()` instead of strict `std::str::from_utf8()` when converting `Bytes` and `ArcBytes` raw data to chars fields. Invalid UTF-8 byte sequences are now replaced with U+FFFD replacement characters rather than raising a parse error.

### Added
- **Invalid UTF-8 tests**: Test coverage for `RawCopy` handling of invalid UTF-8 bytes in both `Bytes` and `ArcBytes` variants.

## [0.3.2] - 2026-05-26

### Fixed
- **kvarr bracket-delimited fields**: When `<[,]>` bracket-delimited field input is empty, return empty string instead of raising a parse error. kvarr value parser now returns `Null` for empty values, and bracket-scope fields whose extracted content is empty are handled gracefully instead of failing.

### Changed
- **Dependencies**: Upgrade `wp-connector-api` 0.9 → 0.10, `wp-error` 0.9 → 0.10, `wp-log` 0.3 → 0.4, `wp-specs` 0.9 → 0.10.

### Added
- **kvarr test cases**: Additional test coverage for kvarr bracket-delimited field parsing with empty input.

## [0.3.0] - 2026-05-04

### Changed
- **orion-error 0.7 → 0.8**: Migrate error-building callsites from the removed `owe()` / `ErrorOweBase` / `UvsFrom` / `ContextRecord` / `testcase::TestAssert` APIs to the 0.8 equivalents:
  - `.owe(Reason::from_conf())` → `.source_err(Reason::core_conf(), detail)` for `io::Error` / `toml::Error` sources.
  - `.owe(Reason::from_data())` → `.source_raw_err(Reason::data_error(), detail)` for third-party `StdError` types.
  - `UvsFrom` delegating constructors (`from_conf()`, `from_data()`, `from_not_found()`) → `#[derive(OrionError)]` delegate constructors (`core_conf()`, `data_error()`, `not_found_error()`).
  - `runtime::ContextRecord` → `OperationContext::record()` / `record_field()` (now inherent methods).
  - `traits_ext::ToStructError` → `conversion::ToStructError`.
  - `testcase::TestAssert` → `dev::testing::TestAssert`.
  - `UvsReason` → `UnifiedReason`.
- **Dependencies**: Upgrade `wp-parse-api` 0.9 → 0.10; `orion-error` from crate path to crates.io `0.8` with `toml` + `serde_json` features.

## [0.2.0] - 2026-05-04

### Changed
- **Error handling**: Replace `anyhow::Result` with structured `WplCodeError` / `WplCodeResult` based on `orion-error::StructError<WplCodeReason>`, providing stable error codes and categories for all parse and runtime paths.
- **Dependencies**: Upgrade `orion-error` 0.6 → 0.7, `wp-parse-api` 0.8 → 0.9, `wp-connector-api` 0.8 → 0.9, `wp-error` 0.8 → 0.9, `wp-specs` 0.8 → 0.9, `ipnet` 2.11 → 2.12; remove `anyhow` dependency.
- **orion-error API migration**: Migrate error-building callsites from `.want()` / `.with()` to `.doing()` / `.with_context()` to match orion-error 0.7 API.

### Added
- **`WplEvaluator::proc_ref()`**: New method that borrows `&RawData` and only clones when preorder pipes are non-empty, eliminating per-rule payload clone on the multi-rule error path (~7% throughput improvement).
- **Preview truncation**: Long input truncated to 80 characters in parse-error detail messages, reducing error-string allocation cost for large payloads.
- **Multi-rule error benchmark**: New `benches/multi_rule_error.rs` comparing `proc()` vs `proc_ref()` across 5–30 failing rules, plus partial-match scenarios.
- **Performance analysis doc**: `docs/benchmark/parse-error-performance.md` with benchmark methodology, results, and baseline comparison workflow.
- **Baseline snapshot**: `docs/benchmark/baselines/v1-proc_ref/` committed for future performance regression comparison via `cargo bench -- --baseline v1-proc_ref`.

### Fixed
- **Parser refactor**: Various parser functions updated to work with the new error types and orion-error 0.7 API.

## 0.1.10 - 2026-04-16
- Merge the `bad_json` matcher fix back into the release line so valid JSON and plain text no longer get captured as `bad_json` fallback output.
- Include the field-level group-pipe fix from `0.1.8`, preserving the `take(...)`-selected field when a group parser is chained after a field pipe.
- Keep the `0.1.9` in-crate `idcard` implementation changes while carrying forward the parser behavior fixes that were missing from the published `0.1.9` package.

## [0.1.9] - 2026-04-12
- Vendor the `idcard` implementation into `wp-lang/src/idcard`, removing the external `idcard` crate dependency while preserving the existing mainland / Hong Kong / Macau / Taiwan identity-card validation and fake-data helpers used by the runtime parser.
- Keep the physical `id_card` parser behavior unchanged while switching it to the new in-crate implementation, so downstream users no longer pull the legacy `idcard` dependency graph into `wp-lang`.

## [0.1.8] - 2026-04-03
- Fix field-level group pipes so they keep using the field selected by `take(...)` instead of incorrectly falling back to the last parsed field.
- Add regression coverage for nested JSON-log parsing through `json(chars@log) | take(log) | (...)`, ensuring the second-stage group parser consumes the selected field payload.
- Preserve the existing `take(...)` plus field-pipe behavior while fixing the `take(...)` plus group-pipe execution path.

## [0.1.7] - 2026-04-01
- Fix `bad_json` so it only matches JSON-like payloads that fail strict JSON parsing, instead of incorrectly accepting valid JSON or plain text.
- Add an explicit `bad_json does not match valid json input` diagnostic detail when `bad_json` is tried against valid JSON.
- Add regression coverage for valid-JSON rejection, plain-text rejection, and the end-to-end `|json_like| -> bad_json` fallback path.

## 0.1.6 - 2026-03-31
- Add the preorder `json_like` pipe for lightweight JSON-like sniffing before full parsing, plus coverage for plain text, valid JSON, and broken JSON-like payloads.
- Teach `json(...)` to run the same lightweight JSON-like sniff internally so plain text is rejected faster without requiring an explicit `|json_like|` guard.
- Add the `bad_json` field parser alias, which preserves the original payload as a `chars` field for JSON-like inputs that fail strict JSON parsing.
- Add Criterion benchmarks comparing `json_like` and `json` on plain-text and broken JSON-like inputs.
- Update the Chinese and English WPL docs with `json_like` / `json` / `bad_json` routing guidance and end-to-end examples for valid-vs-broken JSON fallback rules.

## 0.1.4 - 2026-03-18
- Extract the reusable `wpl-check` execution pipeline into `wpl::check`, including request types, source validation, sample execution, default `rule.wpl` / `sample.txt` resolution, and high-level `run_*_request` entry points for downstream integration.
- Split the `wpl-check` binary, examples, and `wpl-rule-check` skill packaging into the companion `wpl-check` project, leaving `wp-lang` with the reusable `wpl::check` library API only.
- Add a dedicated checker implementation guide in `docs/zh/09-checker-guide.md`.
- Gate checker support behind the explicit `check` feature, leaving the default feature set empty so downstream users do not compile checker code unless they opt in.
- Fix sample target validation ordering so package rule-selection errors are reported before unrelated sample file I/O failures.

## 0.1.3 - 2026-03-15
- Improve `kvarr` parsing so pattern-separated key/value data can keep bracket-prefixed values like `command=() aaa, action=permit` without regressing array or interval payload parsing.
- Add a regression test for `kvarr{,\\s(\\S=)}` with free-text values that begin with bracketed segments.
- Refresh the Chinese WPL docs and sync the core authoring guidance to `docs/en`, fixing outdated syntax examples, invalid group forms, and stale source path references.
- Expand `tools/skills/wpl-rule-check/` into a self-contained publishable skill with bundled references, examples, and cross-model usage guides.
- Add portable skill references for non-Codex agents, including a compact WPL grammar reference and a reusable system prompt.

## 0.1.2 - 2026-03-15
- Add the `wpl-check` CLI for fast WPL validation with explicit `syntax` and `sample` subcommands.
- Make `sample` use `rule.wpl` and `sample.txt` as defaults, and support directory-based example layouts under `examples/wpl-check/`.
- Extract reusable WPL and sample payload fixtures from tests into `examples/wpl-check/` for manual verification.
- Improve `wpl-check` sample diagnostics with offset, line and column, nearby input preview, and `^` pointer rendering.
- Fix `wpl-check` path handling so explicit sample file paths are not rebased unexpectedly.
- Fix `wpl-check --auto` so annotated single-rule WPL is inferred and reported correctly.
- Reject `--rule-name` for non-package sources before reading sample input, avoiding misleading file errors.

## 0.1.0 - 2026-03-13
- Initial standalone release extracted from the `wp-motor` workspace.
- Includes the WPL AST, parser, runtime evaluator, builtins, and generation helpers.
