---
name: release-readiness
description: Prepares and verifies a native Linux Rust desktop application for local use, including release builds, launchers, dependency documentation, and final acceptance checks.
---

# Release Readiness Skill

## When to use
Use before delivering a build, creating a launcher, preparing installation instructions, or deciding whether the editor is ready for use.

## Procedure
1. Inspect the target distribution, architecture, installed libraries, and Rust toolchain.
2. Verify project metadata, dependencies, license notices, and lockfile policy.
3. Run formatting, tests, Clippy, and a release build.
4. Launch the actual release executable and test key workflows.
5. Verify that required runtime libraries are present and document any system package requirements.
6. Provide a repeatable build/run procedure.
7. If adding a desktop entry or icon, verify the paths and launch command.
8. Confirm that the recovery directory is persistent and writable by the current user.
9. Report unresolved issues and environmental limitations.

## Acceptance checklist
- [ ] Release build succeeds.
- [ ] Application starts on the target Linux environment.
- [ ] Create, open, edit, save, and Save As work.
- [ ] Multiple tabs keep independent document state.
- [ ] Font-size shortcuts work.
- [ ] System theme is followed without a manual theme toggle.
- [ ] Burmese, Thai, and Khmer render and edit correctly in the actual GUI.
- [ ] Invalid UTF-8 and likely binary files are handled safely.
- [ ] Unsaved text recovers after forced process termination.
- [ ] Recovery errors do not silently discard text.
- [ ] Tests and static checks have actual recorded results.
- [ ] Memory measurements are recorded with a reproducible method.
- [ ] Build and launch instructions are included.

Do not mark an item complete based only on source-code inspection when runtime verification is required.
