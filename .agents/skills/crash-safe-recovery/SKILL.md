---
name: crash-safe-recovery
description: Designs, implements, and tests durable unsaved-document recovery, atomic checkpoints, journals, filesystem synchronization, session restoration, and crash consistency.
---

# Crash-Safe Recovery Skill

## When to use
Use whenever modifying autosave, recovery, session persistence, document closing, save logic, checkpointing, journals, or filesystem writes.

## Non-negotiable invariant
Never delete or invalidate the last known-good recovery copy until a newer valid copy has been written and synchronized successfully. Unsaved user text must not be discarded silently.

## Storage and model
- Store recovery data under a persistent per-user state directory, preferably XDG state storage such as `~/.local/state/<application-id>/`, not only in memory or `/tmp`.
- Use unique stable document IDs; two documents with the same filename must never collide.
- Persist enough metadata to restore unsaved documents and session layout.
- Distinguish normal-file save state from recovery persistence state.
- Define how multiple application instances are prevented from corrupting the same session.
- Treat recovery files as untrusted input: validate lengths, versions, checksums if used, and record boundaries before allocation or replay.

## Durability design
Before implementation, document the intended durability boundary and acceptable data-loss window. Choose a journal/checkpoint design appropriate to the workload; do not blindly sync the entire document on every keystroke if a correct incremental journal is more suitable.

Consider and test:
- append-only journal records with framing and integrity checks;
- recovery after a partial/torn final record;
- atomic checkpoint replacement;
- file synchronization (`fdatasync`/`fsync` or appropriate Rust/platform API);
- parent-directory synchronization when needed for durable create/rename/replace;
- write ordering between journal, checkpoint, and metadata;
- disk full, permission errors, I/O failures, and interrupted writes;
- recovery from the latest valid checkpoint plus valid journal records;
- bounded background queues and backpressure;
- stale/corrupt recovery metadata;
- safe cleanup only after a durable replacement is confirmed.

Do not claim that `write()`, `flush()`, `close()`, or rename alone guarantees persistence across sudden power loss. Explain the operating-system/filesystem/storage-device assumptions.

## Startup recovery
- Discover valid sessions and recover unsaved documents.
- Do not automatically overwrite the normal file with recovered text.
- Detect when the on-disk file changed since the recovery state was created; preserve the recovered draft and make the conflict clear.
- Recover never-saved documents.
- Ignore or quarantine malformed records without destroying other valid documents.
- Keep recovery behavior deterministic and idempotent.

## Test plan
Use real filesystem operations and temporary directories. Include:
1. recover an unsaved new document after forced process termination;
2. recover unsaved edits to an existing file;
3. recover several tabs and mixed Unicode scripts;
4. truncate/corrupt the final journal record;
5. interrupt checkpoint replacement at each meaningful step;
6. simulate disk-full and I/O failures where practical;
7. verify the previous checkpoint remains recoverable when a new write fails;
8. verify cleanup does not delete the last valid copy;
9. test conflict behavior when the original file changes externally;
10. repeat recovery to verify idempotence.

A process kill is not a real power-loss test. Report simulated process crashes separately from actual reboot/power-loss tests. Do not claim hardware-level guarantees that have not been tested.
