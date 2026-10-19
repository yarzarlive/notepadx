---
name: unicode-text-editing
description: Ensures Unicode-safe file handling and correct editing/rendering of Burmese, Thai, Khmer, combining marks, emoji, mixed scripts, and text with different line endings.
---

# Unicode and Text Editing Skill

## When to use
Use for text-buffer behavior, rendering, file opening/saving, encoding detection, cursor movement, selection, deletion, clipboard operations, or script-related bugs.

## Core model
- Treat UTF-8 as an encoding, not as a cursor-position model.
- UTF-8 byte offsets, Unicode scalar values, grapheme clusters, shaping clusters, and rendered glyph positions are different concepts.
- Use the toolkit's established text model and navigation semantics where appropriate; do not implement cursor movement by incrementing arbitrary UTF-8 byte offsets.
- Do not split a UTF-8 code point when inserting, deleting, slicing, or saving.
- Combining marks may need to be navigated and deleted according to grapheme-aware or toolkit-native behavior.
- Text shaping and bidirectional layout are separate from encoding. UTF-8 validity alone is insufficient.

## Complex scripts
Verify the complete rendering and editing stack for:
- Burmese/Myanmar;
- Thai;
- Khmer;
- combining marks and diacritics;
- emoji and supplementary-plane characters;
- mixed scripts and punctuation;
- bidirectional text where applicable.

Use a proven shaping/layout stack such as Pango/HarfBuzz through the selected toolkit where appropriate. Verify font fallback and actual rendering; merely adding a shaping dependency does not prove the text widget uses it.

Do not normalize Unicode text automatically. Canonically equivalent sequences can have different code-point representations, and silent normalization can change file contents.

## File opening and saving
- Support ordinary text files regardless of extension.
- Validate UTF-8 before treating bytes as UTF-8.
- If input is invalid UTF-8 or likely another encoding, never silently replace invalid bytes or save a lossy conversion over the original. Present a safe, explicit behavior.
- Detect likely binary content and warn before editing; do not silently overwrite binary files.
- Preserve LF/CRLF line endings where practical. Avoid rewriting line endings or encoding in an unmodified document without need.
- If the file is opened and not modified, avoid changing its bytes on disk.
- Handle empty files, no-extension files, long lines, permission errors, missing paths, and read/write failures.
- A failed save must leave the original file and recoverable content intact.

## Test corpus
Include tests with representative strings, for example:
- Burmese: `မြန်မာစာ မင်္ဂလာပါ`
- Thai: `ภาษาไทย`
- Khmer: `ភាសាខ្មែរ`
- Combining marks and emoji: `café e\u0301 👩‍💻 😀`
- Mixed text: `Hello မြန်မာစာ ภาษาไทย ភាសាខ្មែរ 123`

For each script, test rendering in the real GUI and test insertion, cursor navigation, selection, deletion, clipboard operations where possible, save, and reopen. Unit tests on UTF-8 storage alone are insufficient.
