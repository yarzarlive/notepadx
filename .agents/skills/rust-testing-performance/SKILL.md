---
name: rust-testing-performance
description: Builds a practical Rust test strategy and measures correctness, memory use, responsiveness, large-file behavior, and release-build performance without fabricated claims.
---

# Rust Testing and Performance Skill

## When to use
Use when adding tests, diagnosing regressions, benchmarking, investigating memory use, validating release readiness, or reporting project quality.

## Testing strategy
- Add unit tests for pure logic and integration tests for behavior across modules.
- Test error paths, not only the happy path.
- Use temporary directories and real filesystem operations for file and recovery logic.
- Keep regression tests for every discovered data-loss or text-corruption bug.
- Use deterministic test data and avoid timing-sensitive assertions unless timing is the subject of the test.
- Do not mock away the filesystem behavior that a test is meant to validate.
- GUI automation may supplement but does not replace direct tests of recovery and file logic.

## Required checks
Run applicable project commands:
```bash
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build --release
```

If the project does not support one of these commands or features, explain why and run the appropriate alternative. Do not report commands as successful if they were not executed.

## Performance method
Measure a release build where practical. Record:
- OS and kernel/desktop environment;
- Rust toolchain and build profile;
- method/tool used to measure memory;
- idle memory after startup;
- memory with several tabs;
- memory with a documented large-file workload;
- responsiveness during typing and recovery persistence;
- whether filesystem cache or other processes may affect measurements.

Use available tools such as `/usr/bin/time -v`, `ps`, or `/usr/bin/time` only when appropriate, and explain what each measurement means. RSS includes more than Rust heap allocations; do not equate RSS directly with application-owned text memory.

Do not invent numbers, compare incomparable measurements, or claim superiority without a controlled baseline. Establish targets from observed results and the actual environment.

## Large files and resource safety
- Test empty, small, medium, and large files within the stated target.
- Avoid unbounded queues and redundant whole-document copies.
- Ensure a failed allocation or file read does not corrupt recovery state.
- Document limits when a selected widget or buffer architecture cannot handle extremely large files well.

## Completion report
Report:
- exact commands executed;
- pass/fail status;
- tests added;
- measured figures with workload and method;
- GUI checks performed;
- failures and remaining limitations.

Never weaken or remove a meaningful test simply to obtain a green build.
