---
name: linux-native-gui
description: Guides implementation and debugging of a lightweight native Linux desktop GUI, including toolkit choice, system theme integration, keyboard shortcuts, packaging, and responsive UI behavior.
---

# Linux Native GUI Skill

## When to use
Use when choosing or changing the GUI toolkit, window/tab behavior, keyboard shortcuts, theme detection, desktop integration, or GUI event handling.

## Choose the toolkit based on verified requirements
Evaluate a suitable native Linux toolkit, for example GTK4 with Rust bindings or another actively maintained option. Verify actual behavior and compatibility before committing:
- memory and startup overhead;
- complex-script shaping and font fallback;
- text selection, clipboard, undo/redo, and keyboard handling;
- dark/light preference integration;
- availability of required system libraries and packaging requirements.

Do not use Electron or embed a browser engine for this simple native editor. Do not assume a toolkit feature exists without checking the installed version and documentation.

## GUI architecture
- Keep event handlers short and responsive.
- Never perform long blocking disk operations on the UI thread if they can reasonably be moved to a worker.
- Use bounded queues or explicit backpressure for background work; do not let pending recovery writes grow without limit.
- Ensure UI state updates are marshalled through the toolkit's supported mechanism.
- Avoid excessive polling and busy loops.
- Keep per-tab state isolated: text/document identity, path, dirty state, cursor/selection, scroll state, and undo/redo state where supported.
- Closing a tab or window must not silently discard unsaved text.

## Required appearance and shortcuts
- Follow the Linux desktop's system light/dark preference automatically.
- Do not add a manual dark/light switch.
- Respond to preference changes while running where supported by the desktop environment.
- Provide a documented fallback if the current environment does not expose the preference.
- Implement Ctrl++ and Ctrl+- with appropriate keyboard variants for the toolkit and keyboard layout.
- Font size is presentation state, must not mutate document text, and should persist between launches.

## Desktop delivery
- Build a native Linux executable.
- Provide a practical launch command and, where feasible, a desktop entry and icon.
- Inspect the installed distribution before installing packages or choosing commands.
- Prefer project-local or user-local installation. Do not make unrelated system changes.
- Record required system libraries and reproducible build steps.

## GUI verification
Test actual launch and interactions where possible:
- create/open/save documents;
- create, switch, and close multiple tabs;
- unsaved-close behavior;
- font-size shortcuts;
- system theme changes;
- clipboard, selection, undo/redo;
- complex-script rendering and editing;
- responsiveness during recovery I/O.

A successful compile is not proof that the GUI works. State which checks were manual, automated, or blocked by the environment.
