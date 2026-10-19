# Project: Build a Lightweight Linux Notepad in Rust

## 1. Your Role

Act as a senior Rust systems engineer and CTO with extensive experience in native Linux desktop applications, text-editor architecture, Unicode processing, filesystem durability, performance optimization, and AI-assisted software engineering.

Your task is to design, implement, compile, test, and deliver a production-quality Linux text editor inspired by the simplicity and usability of Windows Notepad.

This is a real application-development task, not a prototype, tutorial, or demonstration.

Use sound engineering judgment. Research and verify technical assumptions when necessary. Do not blindly follow an unsuitable implementation approach just because it is easier to generate.

**Your priorities, in order, are:**

1. Correctness and preservation of user data.
2. Correct Unicode and complex-script text rendering and editing.
3. Low memory consumption and responsive operation.
4. Reliability and maintainability.
5. A simple, familiar desktop user experience.

Do not sacrifice data integrity or text correctness for minor performance improvements.

## 2. Work Autonomously

You are authorized to inspect the Linux environment, install the required development tools and dependencies, create the project, write the source code, compile it, run tests, and troubleshoot build or runtime failures.

Follow this workflow:

1. Inspect the existing operating system, architecture, available resources, installed Rust toolchain, and relevant development dependencies.
2. Select an appropriate GUI toolkit, text-editing implementation, and Unicode text-processing approach.
3. Explain the proposed architecture and any significant trade-offs briefly before implementation.
4. Set up the development environment and initialize the project.
5. Implement the application incrementally, keeping the project buildable throughout development.
6. Run automated tests and launch the actual GUI application.
7. Test the important user workflows, persistence behavior, Unicode handling, and performance.
8. Fix discovered problems rather than merely documenting them.
9. Produce a working Linux executable and a convenient way to launch the application.

Do not stop after creating a plan, generating source files, or achieving a successful compilation. Continue until the application has been implemented and verified to the extent possible in the available environment.

If an environmental limitation prevents a test, identify the limitation and perform the remaining tests.

Do not ask me to perform routine development tasks that you can safely perform yourself.

Before destructive system changes, overwriting existing user files, or performing actions outside the scope of this project, obtain my approval. Prefer isolated project dependencies and reversible changes.

## 3. Technology and Architecture

Implement the application primarily in Rust.

Choose a lightweight, actively maintained native Linux GUI toolkit suitable for a responsive desktop text editor. Evaluate reasonable options such as GTK4 with an appropriate Rust binding, or another suitable native toolkit.

Do not select a framework solely because it is popular. Consider actual memory overhead, Unicode shaping support, keyboard handling, native theme integration, packaging complexity, and maintenance requirements.

Avoid embedding a full browser engine or using Electron.

Before committing to a text-editing widget or library, verify that it supports the required Unicode scripts, cursor navigation, selection, editing, clipboard operations, and font fallback. Do not assume that a widget supports these capabilities merely because it accepts UTF-8 strings.

Use Rust's ownership and borrowing model appropriately. Prefer clear module boundaries and straightforward abstractions over unnecessary complexity.

Keep GUI code, text-buffer logic, file I/O, session management, recovery storage, and theme detection sufficiently separated to enable testing.

Avoid unnecessary dependencies, excessive background threads, duplicated text buffers, excessive copying, and unbounded memory growth.

Use safe Rust by default. Do not introduce `unsafe` unless it is genuinely necessary and justified.

Choose dependency versions compatible with the installed toolchain, and verify that the selected libraries can actually be built in this environment.

## 4. Core Editor Features

### 4.1 Text editing

Implement the following:

- Create a new, empty document.
- Open existing text files.
- Edit, select, copy, cut, paste, undo, and redo text.
- Save using Ctrl+S.
- Save As using Ctrl+Shift+S.
- Open files using Ctrl+O.
- Create a new document using Ctrl+N.
- Close the current tab.
- Close the application normally.
- Support standard keyboard navigation and selection.
- Support multiline text, long lines, and empty files.
- Display a modified/unsaved indicator when appropriate.

Use familiar keyboard shortcuts and standard Linux desktop conventions.

Do not add unrelated features such as rich-text formatting, syntax highlighting, plugins, or an IDE interface unless required for a core feature.

### 4.2 Multiple tabs

Support multiple documents in a single application window.

Each tab must maintain its own:

- Text content.
- File path, if associated with a saved file.
- Modified/unsaved status.
- Undo/redo history, where practical.
- Cursor position and selection.
- Scroll position.
- Recovery state.

Switching between tabs must not corrupt or replace another document's contents.

Closing an unsaved tab must never silently discard its contents. Preserve it through recovery storage, and implement an appropriate warning or confirmation when necessary.

When the application is reopened, restore the previous session, including unsaved documents and their tab order.

### 4.3 Unicode and text-file compatibility

Unicode support is mandatory.

Use UTF-8 internally where appropriate, while preserving the contents of supported files correctly.

The application must correctly handle:

- ASCII and ordinary Unicode text.
- Burmese/Myanmar script.
- Thai script.
- Khmer script.
- Other scripts requiring complex shaping.
- Combining marks and diacritics.
- Emoji and supplementary Unicode characters.
- Mixed-script documents.
- Unicode characters that occupy different numbers of bytes.
- Multiline text containing different scripts.
- Cursor movement, deletion, selection, and insertion around combining sequences.

Do not treat a Unicode character, UTF-8 byte, grapheme cluster, and rendered glyph as interchangeable concepts.

Use a text layout and shaping engine that supports appropriate script shaping and font fallback, such as Pango/HarfBuzz where applicable.

Verify that the chosen widget and toolkit actually use the necessary shaping and fallback facilities. Merely linking a shaping library is insufficient.

For editing operations, avoid splitting UTF-8 code points or corrupting combining sequences. Implement cursor movement and deletion according to the toolkit's appropriate text-navigation semantics.

Test representative Burmese, Thai, and Khmer samples for rendering, cursor movement, insertion, deletion, selection, and saving/reopening.

### 4.4 Opening text files

The application should open ordinary text files regardless of filename extension.

Do not restrict file selection to `.txt` files.

Handle files with no extension, unfamiliar extensions, empty contents, long lines, and large files as safely as practical.

Detect or validate UTF-8 input. If a file contains invalid UTF-8 or appears to use another encoding, do not silently corrupt or discard its contents.

Provide a safe, understandable behavior for unsupported encodings. If conversion is offered, make it explicit and preserve the original file until the user chooses to save a conversion.

Detect likely binary content and warn before interpreting it as text. Do not overwrite binary files automatically.

Preserve line endings where practical, including LF and CRLF. Do not introduce unnecessary changes to the file's contents when opening and saving an otherwise unmodified document.

Opening or saving files must handle errors such as missing files, permission denial, full disks, and unavailable directories without crashing or silently losing text.

For very large files, avoid unnecessary copies and benchmark memory consumption. If the selected editing architecture cannot efficiently handle extremely large files, document the limitation and establish a reasonable tested file-size target rather than claiming unlimited support.

## 5. Critical Requirement: Durable Recovery of Unsaved Text

This is one of the most important features of the application.

Every document must have recoverable content while it is being edited, even when it has never been saved to a normal user-selected file.

The recovery system must work across:

- Normal application closure.
- Unexpected process termination.
- Application crashes.
- Operating-system crashes and sudden reboots.
- Temporary power loss, to the extent supported by the filesystem and storage hardware.

Do not rely solely on application shutdown handlers, exit callbacks, or an in-memory session object. These do not execute reliably during a crash or sudden power loss.

### 5.1 Persistent recovery storage

Use a persistent, per-user recovery directory appropriate for Linux, preferably under the XDG state directory, such as:

`~/.local/state/<application-id>/`

Do not store the only recovery copy in `/tmp`, which may be cleared during reboot.

Store sufficient information to recover every open document, including:

- Document identity.
- Associated file path, if any.
- Current text content or a replayable edit journal.
- Tab order.
- Cursor and selection state where practical.
- Recovery metadata needed to identify the latest valid state.

Ensure that two documents with identical names do not overwrite each other's recovery data.

Handle multiple application instances safely, or explicitly prevent concurrent instances from corrupting the same recovery session.

### 5.2 Durability requirements

Design a durable write-ahead journal or another rigorously justified recovery mechanism.

The recovery mechanism must address:

- Frequent edits.
- Atomic replacement of checkpoint files.
- File synchronization using appropriate operating-system primitives such as `fdatasync` or `fsync`.
- Synchronization of the containing directory when required to make file creation or replacement durable.
- Interrupted or partially written journal records.
- Torn writes and corrupted recovery metadata.
- Recovery from the latest valid checkpoint plus any valid subsequent journal records.
- Cleanup of obsolete recovery data without deleting the latest valid copy.
- Disk-full and I/O errors.
- Application restart while recovery data is incomplete.

Do not assume that renaming a file or calling a high-level write function alone guarantees persistence after sudden power loss.

Do not delete the last valid recovery copy until its replacement has been successfully written and synchronized.

### 5.3 Recovery latency and data-loss window

Use a recovery strategy that minimizes data loss without making typing unacceptably slow.

Define precisely when an edit is considered durably committed. If necessary, use an append-only journal, checkpoints, and carefully designed synchronization to balance responsiveness and durability.

Do not claim zero data loss if the implementation has an unsynchronized write window.

Because durability is more important than small performance gains, benchmark the cost of the chosen synchronization strategy and optimize it without silently weakening the stated guarantee.

Any guarantee is subject to the filesystem, operating system, storage device, and hardware honoring synchronization requests.

### 5.4 Startup recovery

On startup:

1. Discover valid recovery sessions.
2. Identify documents that were not cleanly closed.
3. Restore the latest valid text for each document.
4. Restore the tab arrangement and other saved session information where possible.
5. Handle damaged or incomplete recovery records safely.
6. Avoid silently overwriting an existing file on disk with recovered content.

For documents that have an associated file path, detect if the on-disk file has changed since the recovery snapshot. If the contents differ, recover the unsaved text into the editor and avoid automatic overwriting.

Recovery must also work for documents that have never been saved to a normal file.

### 5.5 Recovery cleanup

Keep recovery data for unsaved documents and documents requiring crash recovery.

After a document has been explicitly saved and its recovery state has been safely reconciled, obsolete recovery data may be removed.

Never remove the only valid copy of unsaved content merely because the application is closing.

Implement a clear distinction between a document being saved to its normal file and its recovery state being safely persisted.

### 5.6 Mandatory recovery testing

Test the following scenarios:

- Type text into a new document, terminate the application unexpectedly, reopen it, and verify that the text is recovered.
- Edit an existing file without saving, terminate the application unexpectedly, reopen it, and verify that the unsaved version is recovered.
- Edit several tabs and recover all of them.
- Recover a document containing Burmese, Thai, Khmer, emoji, and combining marks.
- Interrupt recovery writes and verify that incomplete records do not replace valid data.
- Simulate a full-disk or write failure where practical.
- Test recovery after an actual process kill.
- Test forced reboot or equivalent crash-recovery scenarios where the environment permits.

A process kill is not equivalent to a real power-loss test. Clearly distinguish simulated tests from verified power-loss behavior.

## 6. Font Size Controls

Support changing the displayed text font size using:

- `Ctrl` + `+` to increase the font size.
- `Ctrl` + `-` to decrease the font size.
- Support the usual keyboard variants for the plus key where the GUI toolkit distinguishes them.

The font-size change must:

- Apply consistently to all tabs.
- Preserve the underlying text exactly.
- Affect display only, not insert or delete text.
- Have sensible minimum and maximum limits.
- Remain responsive when displaying long documents.

Persist the selected font size between application launches.

Use the normal desktop font-rendering and shaping pipeline. Font-size changes must not break complex-script rendering or font fallback.

## 7. Automatic Dark and Light Themes

The application must automatically follow the user's Linux desktop appearance preference.

Requirements:

- When the system uses a light theme, use an appropriate light editor theme.
- When the system uses a dark theme, use an appropriate dark editor theme.
- Detect theme changes while the application is running and update the interface when supported by the desktop environment.
- Use appropriate foreground, background, selection, cursor, and tab colors.
- Maintain readable contrast in both modes.
- Avoid hard-coded colors that prevent the interface from following the system theme.

There must be no manual Dark/Light switch in the application.

Use the GUI toolkit's native theme integration or an appropriate desktop appearance preference mechanism. Verify actual behavior on the installed Linux desktop rather than assuming every desktop environment exposes theme preferences identically.

If system preference detection is unavailable, define a sensible toolkit-native fallback and document the limitation. Do not add a manual theme selector.

## 8. Memory Efficiency and Responsiveness

The application must be genuinely lightweight.

Optimize for normal desktop usage with multiple open documents and sustained typing.

Requirements:

- Avoid unnecessary heap allocations and text-buffer copies.
- Avoid storing redundant complete copies of document contents.
- Do not repeatedly serialize the entire document after every keystroke if an efficient incremental strategy is appropriate.
- Avoid unbounded queues of pending disk writes.
- Avoid busy loops, excessive polling, and unnecessary background threads.
- Ensure that recovery I/O does not freeze the GUI.
- Keep document state and recovery metadata compact.
- Use suitable buffering and incremental processing for large documents.
- Avoid retaining obsolete text snapshots indefinitely.
- Ensure that tab switching and ordinary editing remain responsive.

Do not introduce complex rope, piece-table, or other specialized text-buffer structures without evidence that they improve this application's actual workload.

Choose the simplest architecture that satisfies the correctness and performance requirements.

Measure actual idle memory consumption and memory consumption with multiple tabs and large documents. Record the operating system, build configuration, document sizes, tab counts, and measurement method.

Establish reasonable performance targets after inspecting the environment. Report measured results honestly; do not invent benchmarks or claim that the application is more memory-efficient than alternatives without evidence.

## 9. Linux Integration and Delivery

Inspect the installed Linux distribution before choosing installation commands.

Install the necessary Rust toolchain, compiler, build tools, GUI development libraries, and other required dependencies.

Prefer reproducible project dependencies and document the commands needed to rebuild the application.

Create a normal Linux desktop application experience, including where applicable:

- A working executable.
- A desktop launcher.
- An application name and icon.
- Appropriate application metadata.
- A convenient launch command.
- A clean build and release procedure.

Do not modify the system's boot configuration or make unrelated system changes.

Prefer installing project artifacts in a project-local or user-local location unless a system-wide installation is explicitly justified.

Keep all source files, configuration, tests, and build instructions in a clearly organized project directory.

The application must run natively on Linux and must not depend on a Windows compatibility layer.

## 10. Code Quality and Testing

Use the following quality gates where applicable:

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`
- A successful release build.

Resolve warnings and errors rather than suppressing them indiscriminately.

Add automated tests for the text-buffer logic, recovery journal, atomic checkpointing, file handling, encoding validation, and session restoration.

Test error paths as well as successful operations.

Test important GUI workflows manually or through suitable GUI automation when available.

Do not consider compilation alone sufficient proof of correctness.

Avoid `unwrap()` and `expect()` in ordinary runtime paths where an error can occur. Handle expected filesystem, encoding, and recovery errors explicitly.

Do not use mocks as a substitute for all filesystem durability testing. Use real temporary files and filesystem operations for the relevant tests.

Do not mark tests as passing unless they have actually run successfully.

## 11. Engineering Rules for AI-Assisted Development

Follow these rules throughout the project:

- Inspect the codebase before modifying existing files.
- Establish the architecture before implementing complex features.
- Prefer incremental, testable changes.
- Verify library APIs and dependency compatibility rather than inventing methods or assuming examples are current.
- Avoid unnecessary rewrites and unrelated features.
- Do not create placeholder implementations for required functionality.
- Do not hide failures behind broad error suppression.
- Do not silently downgrade recovery durability to make benchmarks look better.
- Do not claim Unicode correctness without testing actual scripts.
- Do not claim low memory consumption without measurements.
- Keep documentation consistent with the implementation.
- If a requirement conflicts with another, explain the conflict and select the solution that best preserves the stated priorities.
- If a major technical decision remains uncertain, investigate it before committing to an expensive implementation.
- When a test fails, diagnose and fix the underlying cause instead of weakening the test without justification.

Do not stop to ask for permission for every routine engineering decision. Make reasonable decisions autonomously and document significant trade-offs.

## 12. Definition of Done

The project is complete only when:

1. The Rust application builds successfully in release mode.
2. The application launches and supports normal text-editing workflows.
3. Multiple tabs work independently.
4. Ctrl+Plus and Ctrl+Minus change font size correctly.
5. The interface follows system dark/light appearance without a manual theme switch.
6. Burmese, Thai, and Khmer text renders and remains correct through editing and saving/reopening.
7. Ordinary text files can be opened without extension restrictions, with safe handling of invalid encodings and binary content.
8. Unsaved documents survive unexpected process termination and recover on the next launch.
9. Recovery writes use a documented, tested durability strategy.
10. Relevant automated tests pass.
11. Actual memory and responsiveness measurements are recorded.
12. Build and launch instructions are provided.
13. Known limitations are explicitly documented.

## 13. Final Report

When finished, provide a concise engineering report containing:

- The selected GUI toolkit and text-editing architecture, with reasons.
- The project directory and important source files.
- Installed development dependencies.
- Commands to build, test, and run the application.
- The location and format of recovery data.
- The durability guarantees and remaining limitations.
- The Unicode and complex-script tests performed.
- Actual memory measurements and the methodology used.
- Test and build results.
- Any incomplete requirements or known defects.

**Final instruction:** Start by inspecting the Linux environment and selecting a technically appropriate architecture. Then implement, build, launch, test, and refine the application. Deliver a functioning editor, not merely a proposed design or a collection of source files.