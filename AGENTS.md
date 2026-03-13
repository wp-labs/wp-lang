# Repository Guidelines

## Project Structure & Module Organization
`wp-lang` is a single Rust library crate. Public entry points live in `src/lib.rs`. The main subsystems are `src/ast/` for WPL syntax trees, `src/parser/` for WPL source parsing, `src/eval/` for runtime execution and builtins, `src/generator/` for generation helpers, and `src/precompile.rs` for compilation helpers. Benchmarks live in `benches/`. Keep tests beside the code they verify.

## Build, Test, and Development Commands
Use `cargo check` for fast validation, `cargo build` for a full compile, and `cargo test` for the co-located unit suite. Before sending changes, run `cargo fmt --all` and `cargo clippy --all-targets --all-features -- -D warnings`. Use `cargo bench` when changing parser or runtime hot paths.

## Coding Style & Naming Conventions
Follow default Rust formatting. Use `snake_case` for modules and functions, `UpperCamelCase` for types and enums, and `SCREAMING_SNAKE_CASE` for constants. Keep public APIs WPL-oriented; low-level syntax-agnostic parsing belongs in `wp-primitives`, not here.

## Testing Guidelines
Add unit tests beside the implementation under `#[cfg(test)] mod tests`. Name tests `test_*` and cover parser correctness, runtime behavior, and failure paths. When changing performance-sensitive code in `eval/value/parser` or `parser/`, update Criterion benchmarks when useful.

## Commit & Pull Request Guidelines
Use concise imperative commit subjects under 70 characters, for example `Add WPL package parser for nested annotations`. PRs should explain the WPL behavior being changed and list the verification commands that were run.

## Security & Configuration Tips
Treat all input as untrusted source text or payload data. Prefer structured parser/runtime errors over panics. Keep dependencies published and versioned; avoid introducing new monorepo-only path dependencies because this crate is intended to stay independently releasable.
