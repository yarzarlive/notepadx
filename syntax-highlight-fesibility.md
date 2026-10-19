Adding syntax highlighting to NotepadX is **highly feasible**, but the architectural choice of *how* to implement it will dictate performance, complexity, and maintenance.

Here is a breakdown of the two primary practical approaches, their trade-offs, and an architectural recommendation:

---

### Option A: `GtkSourceView 5` (The Native GNOME Standard) — *Recommended*

`GtkSourceView` is the official GNOME library specifically designed for text editing atop GTK4 and Libadwaita (powers GNOME Text Editor, Builder, and Gedit).

* **How it works**:
  * `sourceview5::Buffer` subclasses `GtkTextBuffer`.
  * `sourceview5::View` subclasses `GtkTextView`.
  * Existing code (atomic saves, crash recovery, find & replace, UTF-8/script handling) remains almost identical because it directly inherits from `GtkTextBuffer`.
* **Language Support & Themes**:
  * **Languages**: Built-in support for over 150 languages out of the box (including Rust, Python, C/C++, JavaScript, Markdown, SQL, INI, YAML, JSON, Shell).
  * **Themes**: Uses standard `.xml` style schemes. Reuses hundreds of existing themes from GNOME/Gedit/GitHub (Solarized, Dracula, Cobalt, Monokai, Kate, Adwaita, etc.).
  * Automatically coordinates with Libadwaita’s system light/dark mode preference.
* **Pros**:
  * **Zero tag management overhead**: Highlighting, bracket matching, line numbers, and indentation are handled natively in C/Pango on the render path.
  * **Complex scripts**: Complex script shaping (Burmese, Thai, Khmer) is preserved because layout is handled at the Pango layer without fragmented `GtkTextTag` collisions.
  * **Memory & Performance**: Asynchronous rendering optimized for large files; zero latency on keystrokes.
* **Trade-offs**:
  * Introduces `libgtksourceview-5-dev` (`libgtksourceview-5-0`) as a system dependency. (Ubuntu 24.04 / Debian already provide this standard in default package repos).

---

### Option B: `syntect` (Pure Rust + Sublime Text / TextMate Themes)

`syntect` is a pure Rust syntax highlighting engine using Oniguruma regexes and Sublime Text syntax definitions.

* **How it works**:
  * You retain plain `GtkTextView` and `GtkTextBuffer`.
  * On text changes, a background thread tokenizes the text and applies `GtkTextTag` styling (foreground colors, bold, italic) across character spans.
* **Language Support & Themes**:
  * Reuses Sublime Text (`.sublime-syntax`) definitions and TextMate/VSCode (`.tmTheme`) color schemes.
  * Can embed default language sets (C, Rust, Python, SQL, Markdown, INI, etc.) directly into the binary.
* **Pros**:
  * No external GTK C libraries beyond GTK4 and Libadwaita.
  * Access to the vast TextMate/Sublime `.tmTheme` ecosystem.
* **Trade-offs**:
  * **High GTK bridge complexity**: You must manually create, cache, diff, and apply thousands of `GtkTextTag` spans on the GTK main loop.
  * **Performance risk on large files**: Clearing and reapplying tags across thousands of lines can cause UI stutter or high memory churn unless an incremental viewport-only highlighting engine is written.
  * **Complex script boundary hazards**: Applying tag boundaries in the middle of grapheme clusters or combining marks can trigger shaping artifacts in Burmese/Thai/Khmer text.

---

### Option C: `tree-sitter` (AST-based parsing)

* High accuracy, but requires compiling individual C grammars per language and writing a custom highlighting query evaluator to map to GTK tags. 
* Overkill and unnecessary complexity for a fast, lightweight notepad.

---

### Feasibility Summary & Wise Recommendation

| Criteria | Option A: `GtkSourceView 5` | Option B: `syntect` |
| :--- | :--- | :--- |
| **Theme & Language Reusability** | **Huge** (Gedit, GNOME, Kate XML themes + 150+ langs) | **Huge** (Sublime, VSCode `.tmTheme` + `.sublime-syntax`) |
| **Code Changes to NotepadX** | **Low** (`Buffer`/`View` are drop-in subclasses) | **High** (Requires custom tag management & parsing pipeline) |
| **Keystroke Performance** | **Native** (Runs in render loop, no lag) | **Medium/Low** (Tag application overhead on main loop) |
| **Complex Script Safety** | **High** (Tested GNOME Pango shaping pipeline) | **Medium** (Risk of tag boundaries breaking syllable shaping) |
| **System Dependencies** | Requires `libgtksourceview-5-dev` | Pure Rust dependencies |

**Verdict**:
If you are open to installing `libgtksourceview-5-dev` on the host Linux system, **`GtkSourceView 5` is the superior, reliable choice**. It provides instant access to hundreds of open-source themes and languages (including Markdown, INI, SQL, and all major languages) while maintaining top performance and data integrity.