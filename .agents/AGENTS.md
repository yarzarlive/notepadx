# Project-wide Agent Instructions — Rust Linux Notepad

## Mission
Build and maintain a lightweight native Linux text editor in Rust. The editor must support multiple tabs, Unicode and complex-script editing, automatic system light/dark appearance, font-size shortcuts, and durable recovery of unsaved text.

## Priority order
1. Preserve user data and avoid silent corruption.
2. Correct Unicode editing and rendering, including Burmese/Myanmar, Thai, and Khmer.
3. Correctness and reliability.
4. Responsiveness and measured memory efficiency.
5. Simplicity and maintainability.

## Mandatory engineering behavior
- Inspect the existing repository and environment before making changes.
- Verify dependency APIs and compatibility against current project versions; do not invent APIs.
- Make focused, incremental changes. Keep the project buildable.
- Run relevant tests and report actual results. Never claim a test, benchmark, build, or GUI check ran unless it did.
- Do not replace a real implementation with a placeholder or weaken a test merely to make it pass.
- Handle filesystem, parsing, and recovery errors explicitly. Avoid `unwrap()`/`expect()` on ordinary runtime paths.
- Prefer safe Rust. Any `unsafe` must be necessary, minimal, documented, and justified.
- Do not add unrelated features, unnecessary dependencies, or speculative abstractions.
- Do not silently discard, truncate, normalize, or overwrite user text.
- Ask before destructive system changes or overwriting unrelated user files. Routine project-local development work may proceed autonomously.

## Project invariants
- Recovery data must live in persistent per-user state storage, not only in memory or `/tmp`.
- Do not delete the last known-good recovery copy before a replacement has been written and synchronized successfully.
- A successful normal-file save and a durable recovery checkpoint are different states; model them separately.
- UTF-8 bytes, Unicode scalar values, grapheme clusters, and rendered glyphs are not interchangeable.
- UTF-8 validity alone does not guarantee correct complex-script shaping.
- The application has no manual dark/light theme selector; it follows the system preference.
- Performance claims require measurements with the workload and method documented.

## Definition of done
A task is complete only when implementation, relevant tests, formatting/linting, and any applicable GUI checks have actually been performed. Report limitations and unverified behavior explicitly.
