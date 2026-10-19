---
name: rust-engineering
description: Applies production-grade Rust engineering practices for this Linux text editor, including module design, ownership, error handling, dependency selection, and safe incremental implementation.
---

# Rust Engineering Skill

## When to use
Use when creating or changing Rust code, selecting crates, refactoring modules, handling errors, or reviewing implementation quality.

## Workflow
1. Inspect `Cargo.toml`, `Cargo.lock`, the existing module tree, toolchain version, and relevant code before editing.
2. Identify the smallest coherent change and the invariants it must preserve.
3. Check crate documentation and versions before relying on an API. Prefer dependencies already in the project when suitable.
4. Implement in small, buildable steps.
5. Add or update tests alongside behavior changes.
6. Run formatting, tests, Clippy, and a release build when feasible.
7. Report commands run, results, and remaining uncertainty.

## Design rules
- Use ownership and borrowing to make lifetimes and resource ownership clear.
- Prefer typed errors or `thiserror`-style error enums for library/domain code; add context at application boundaries where useful.
- Propagate recoverable errors with `Result` and `?`; do not panic on expected I/O, encoding, or user-input failures.
- Avoid broad `Box<dyn Error>` in core modules when typed errors improve handling.
- Keep GUI event handlers thin. Move file I/O, recovery logic, and domain behavior into testable modules.
- Separate concerns such as document state, file encoding, session state, recovery persistence, and GUI presentation.
- Avoid needless `clone()`, repeated full-buffer copies, premature micro-optimization, and unnecessary shared mutable state.
- Use `Arc`, locks, channels, and background workers only when their ownership/concurrency benefits are clear.
- Keep `unsafe` out unless a verified requirement makes it necessary. If used, minimize its scope and document the safety argument.
- Do not add a framework or abstraction layer without a concrete need.
- Avoid suppressing compiler or Clippy warnings without a specific, documented reason.

## Dependency discipline
- Check maintenance status, supported platforms, licensing, transitive dependency cost, and compatibility with the installed toolchain.
- Avoid duplicate crates that solve the same problem.
- Commit `Cargo.lock` for this application binary unless the project deliberately adopts another reproducible-build policy.
- Do not upgrade unrelated dependencies during a feature change.

## Verification checklist
- `cargo fmt --check`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo build --release`

Run commands supported by the actual project; if a command fails, diagnose it and report the real result rather than claiming success.
