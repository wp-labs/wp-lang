# wpl-check Examples

Each example lives in its own directory so the WPL source and sample payload stay together.

## `csv_demo`

Syntax check:

```bash
cargo run --bin wpl-check -- syntax examples/wpl-check/csv_demo/rule.wpl
cargo run --bin wpl-check -- syntax examples/wpl-check/csv_demo
```

Run one sample payload against a single rule:

```bash
cargo run --bin wpl-check -- sample --rule examples/wpl-check/csv_demo/rule.wpl examples/wpl-check/csv_demo/sample.txt
cargo run --bin wpl-check -- sample --rule examples/wpl-check/csv_demo
```

## `package_demo`

Run one sample payload against a package rule:

```bash
cargo run --bin wpl-check -- sample --package --rule-name csv_user examples/wpl-check/package_demo
```

## `log_line`

Log parsing example:

```bash
cargo run --bin wpl-check -- sample --rule examples/wpl-check/log_line/rule.wpl examples/wpl-check/log_line/sample.txt
```
