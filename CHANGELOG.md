# Changelog

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
