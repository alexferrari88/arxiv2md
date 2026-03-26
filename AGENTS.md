# Repository Guidelines

## Project Structure & Module Organization
`arxiv2md` is a small Rust CLI. Core code lives in `src/`, with `main.rs` as the binary entry point and `lib.rs` wiring the internal modules together. The pipeline is split by concern: `cli.rs` parses flags, `pipeline.rs` orchestrates conversion, and format-specific logic lives in files such as `html.rs`, `latex.rs`, `pdf.rs`, `markdown.rs`, and `metadata.rs`. Integration tests live in `tests/cli.rs`; keep module-local unit tests beside the code they cover with `#[cfg(test)]`.

## Build, Test, and Development Commands
Use Cargo for all routine work:

- `cargo build` builds the CLI in debug mode.
- `cargo run -- 2501.11120 --frontmatter` runs the tool locally against an arXiv ID.
- `cargo test` runs unit and integration tests.
- `cargo fmt` formats the codebase with Rustfmt.
- `cargo clippy --all-targets --all-features -D warnings` checks for lint regressions before review.
- `cargo install --path .` installs the local binary for manual testing.

For full fallback coverage, install `pandoc` and keep it on `PATH`.

## Coding Style & Naming Conventions
Follow standard Rust style: 4-space indentation, trailing commas where Rustfmt expects them, and one responsibility per module. Use `snake_case` for functions, variables, and module files; `PascalCase` for types and enums; and `SCREAMING_SNAKE_CASE` for constants. Prefer small parsing and transformation helpers over large monolithic functions. Run `cargo fmt` before opening a PR.

## Testing Guidelines
This repository uses Rust’s built-in test framework plus `assert_cmd` and `predicates` for CLI coverage. Add focused unit tests near pure logic, and integration tests in `tests/` for user-facing behavior such as flags, stderr messages, and exit codes. Name tests after the behavior they verify, for example `invalid_id_fails` or `help_succeeds`. Run `cargo test` locally before submitting changes.

## Commit & Pull Request Guidelines
Current history uses concise Conventional Commit subjects, for example `feat: add first implementation`. Keep using that style (`feat:`, `fix:`, `docs:`, `test:`) and write subjects in the imperative mood. Pull requests should summarize the behavior change, note any new CLI flags or fallback-path changes, and include representative commands or output snippets when they help reviewers validate the result.
