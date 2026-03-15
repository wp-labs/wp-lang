# Changelog

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
