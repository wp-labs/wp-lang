# wpl-check Examples

Each example lives in its own directory so the WPL source and sample payload stay together.

## `csv_demo`

Syntax check:

```bash
wpl-check syntax examples/wpl-check/csv_demo/rule.wpl
wpl-check syntax examples/wpl-check/csv_demo
```

Run one sample payload against a single rule:

```bash
wpl-check sample --rule examples/wpl-check/csv_demo/rule.wpl examples/wpl-check/csv_demo/sample.txt
wpl-check sample --rule examples/wpl-check/csv_demo
```

## `package_demo`

Run one sample payload against a package rule:

```bash
wpl-check sample --package --rule-name csv_user examples/wpl-check/package_demo
```

## `log_line`

Log parsing example:

```bash
wpl-check sample --rule examples/wpl-check/log_line/rule.wpl examples/wpl-check/log_line/sample.txt
```

## `nginx_access_clf`

Nginx access log with CLF time in `[]`:

```bash
wpl-check sample --rule examples/wpl-check/nginx_access_clf
```

## `nginx_access_brace_time`

Nginx-style line with time wrapped by `{}`:

```bash
wpl-check sample --rule examples/wpl-check/nginx_access_brace_time
```
