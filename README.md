# wp-lang

![CI](https://github.com/wp-labs/wp-lang/workflows/CI/badge.svg)
[![codecov](https://codecov.io/gh/wp-labs/wp-lang/graph/badge.svg?token=6SVCXBHB6B)](https://codecov.io/gh/wp-labs/wp-lang)
![License](https://img.shields.io/badge/License-Apache%202.0-green.svg)

## Overview
`wp-lang` is the standalone WPL language crate from the Warp Parse stack. It contains the WPL AST, parser, runtime evaluator, built-in value parsers, preprocessing hooks, and generation utilities needed to parse and execute WPL rules.

The crate sits above `wp-primitives`: `wp-primitives` handles syntax-agnostic parsing helpers, while `wp-lang` owns language semantics, WPL-specific AST types, builtins, and runtime behavior.

## What It Provides
- WPL AST types such as `WplRule`, `WplExpress`, `WplField`, and `WplPackage`
- Source parsers for rules, packages, annotations, fields, separators, and functions
- Runtime evaluation through `WplEvaluator`
- Built-in parsing and transformation pipeline units
- Value parsers for common formats such as JSON, KV, arrays, time, IP, and protocol text
- Precompile helpers for turning parsed rules into executable units

## Quick Start
```toml
[dependencies]
wp-lang = "0.1"
```

```rust
use wp_primitives::Parser;
use wpl::wpl_express;

let mut code = "(digit:id, chars:name)";
let expr = wpl_express.parse_next(&mut code).unwrap();

assert!(expr.statement.first_group().is_some());
```

## Public API Notes
- The package name is `wp-lang`
- The Rust library name stays `wpl`

This keeps downstream imports concise while exposing a publishable package name.

## Design Boundary
- `wp-primitives`: syntax-agnostic parsing primitives
- `wp-lang`: WPL syntax, AST, builtins, runtime evaluation, and generation
- Typed data semantics continue to rely on the published `wp-model-core`, `wp-parse-api`, and related crates

## Development
Run:
- `cargo check`
- `cargo test`
- `cargo fmt --all`
- `cargo clippy --all-targets --all-features -- -D warnings`

Benchmarks live under `benches/` and use Criterion.

## License
Distributed under the Apache License 2.0. See [`LICENSE`](LICENSE).
