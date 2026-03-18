---
name: wpl-rule-check
description: Draft, revise, and validate WPL parsing rules and sample payloads with `wpl-check`. Use when Codex needs to write or fix `rule.wpl`, `sample.txt`, built-in WPL examples, field/group structure, package rule selection, or run `wpl-check syntax` and `wpl-check sample` to verify WPL parsing behavior.
---

# WPL Rule Check

Write WPL around a concrete sample first, then validate with `wpl-check`.

Prefer the bundled wrapper script:

- `scripts/run-wpl-check.sh`
- `scripts/import-wp-rule-example.sh`

It resolves `wpl-check` in this order:

1. `WPL_CHECK_BIN` if set
2. `wpl-check` from `PATH`
3. a local `wp-lang` checkout via `cargo run --features wpl-check-cli --bin wpl-check`
4. `WPL_CHECK_MANIFEST_PATH` if pointed at a `wp-lang` `Cargo.toml`

If none of these work, stop and tell the user how to install `wpl-check`.

If the task is about how to write WPL, read:

- `references/how-to-write-wpl.md` first for the distilled writing workflow
- `references/wpl-grammar-reference.md` second for the portable syntax skeleton
- `references/wpl-quick-patterns.md` third for fast drafting reminders
- `references/wpl-examples.md` fourth for ready-made examples and adaptation templates
- `references/wp-rule-repo.md` when the user wants real shared-rule examples from `https://github.com/wp-labs/wp-rule`
- `references/cross-model-usage.md` when porting this skill to Claude, Gemini, Cursor, Cline, Continue, or other agent frameworks

## Workflow

1. Start from sample data, not from abstract rule ideas.
2. Decide the target shape:
   - Single rule: `rule ... { ... }`
   - Package: `package ... { rule ... }`
   - Expression only: `( ... )`
3. Prefer storing the working pair as `rule.wpl` and `sample.txt`.
4. If the task needs a reusable example, copy and adapt one from this skill's `examples/` directory.
5. If the user wants a real shared-library example from `wp-rule`, first run `scripts/import-wp-rule-example.sh` to materialize a local `rule.wpl` / `sample.txt` pair.
6. Validate syntax before sample parsing.
7. Validate sample parsing before editing surrounding docs or tests.

## Authoring Rules

- Keep rules incremental. Parse the most stable prefix first, then extend.
- Prefer explicit field names for values the user cares about.
- Use `_` only for data that is intentionally ignored.
- Keep the field order explicit: `type [subfields] [:name] [format] [separator] {| pipe}`.
- Put `:name` before the format:
  - `time/clf:time<[,]>`
  - `http/request:request"`
- Treat `opt(...)`, `alt(...)`, `some_of(...)`, `seq(...)`, and `not(...)` as group-level constructs.
- Use `opt(type)@key` only for optional JSON/KV subfields.
- Do not write `one_of(...)`; use `alt(...)`.
- When structure is known, prefer explicit validation mode instead of relying on auto detection:
  - `--rule`
  - `--package`
  - `--expr`
- For package input, always decide whether `--rule-name` is needed.
- Read `references/how-to-write-wpl.md` before writing a new rule from scratch.
- Open `references/wpl-grammar-reference.md` when the exact syntax shape is unclear.
- When you are unsure about separators, quoting, or repeat syntax, open `references/wpl-quick-patterns.md` before inventing syntax.
- When the input looks like CSV, nginx access log, package-based selection, or simple line logs, start from `references/wpl-examples.md` instead of writing from scratch.
- When the user explicitly wants team-style or production-style examples, open `references/wp-rule-repo.md` and use `wp-rule` as the external example source before inventing a new layout.
- When working from `wp-rule`, prefer converting the target example with `scripts/import-wp-rule-example.sh` before manual edits.
- If you are working inside the `wp-lang` repository, you may optionally cross-check `docs/zh/06-grammar-reference.md`, but this skill must remain usable without that repo.

## Validation Commands

Run through `scripts/run-wpl-check.sh` unless the user explicitly asked for the raw command.

If the source example lives in `wp-rule`, first materialize a local working pair:

```bash
scripts/import-wp-rule-example.sh /path/to/wp-rule example_name
scripts/import-wp-rule-example.sh /path/to/wp-rule/models/wpl/example_name /tmp/example_name
```

Syntax only:

```bash
scripts/run-wpl-check.sh syntax --rule path/to/rule.wpl
scripts/run-wpl-check.sh syntax --package path/to/rule.wpl
scripts/run-wpl-check.sh syntax --expr path/to/rule.wpl
```

Parse one sample:

```bash
scripts/run-wpl-check.sh sample --rule path/to/rule.wpl path/to/sample.txt
scripts/run-wpl-check.sh sample --package --rule-name rule_name path/to/rule.wpl path/to/sample.txt
```

Directory shorthand with defaults:

```bash
scripts/run-wpl-check.sh syntax ./csv_demo
scripts/run-wpl-check.sh sample --rule ./csv_demo
```

Quick inline sample only when the user does not want a file:

```bash
scripts/run-wpl-check.sh sample --rule --data '42,alice,' path/to/rule.wpl
```

## Reading Failures

When `wpl-check sample` fails, use the diagnostic fields directly:

- `reason`: current parser expectation
- `target`: selected rule or package/rule
- `offset`: byte offset in parser input
- `line` and `column`: rendered position
- `near`: nearby snippet
- `^`: exact failure pointer on the rendered line

Use `scripts/run-wpl-check.sh syntax --print ...` when the normalized WPL shape matters.

## Repository Conventions

- This skill is self-contained.
- Tool wrapper lives under `scripts/`.
- `wp-rule` import helper lives under `scripts/`.
- Writing references live under `references/`.
- Built-in examples live under `examples/`.
- Cross-model adaptation material:
  - `references/cross-model-usage.md`
  - `references/portable-system-prompt.md`
- External example-library note:
  - `references/wp-rule-repo.md`
- Portable syntax reference:
  - `references/wpl-grammar-reference.md`
- Example cases bundled with the skill:
  - `examples/csv_demo`
  - `examples/log_line`
  - `examples/nginx_access_clf`
  - `examples/nginx_access_brace_time`
  - `examples/package_demo`
  - `examples/kvarr_array_time`
- If you are maintaining `wp-lang` itself, you can mirror useful examples into the repository's own `examples/wpl-check/`.

## Completion Criteria

Before finishing:

1. Ensure the rule parses in the intended mode.
2. Ensure the sample parses, or the failure is intentional and clearly explained.
3. If you added or changed reusable examples, keep `rule.wpl` and `sample.txt` together.
4. If behavior changed, update nearby tests or add one focused regression test.
