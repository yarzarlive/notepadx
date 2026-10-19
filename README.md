# NotepadX

> A fast, lightweight native Linux text editor built in Rust with GTK4 and Libadwaita.

> [!NOTE]  
> **Created with AI**: NotepadX was architected, engineered, and developed through AI-assisted pair programming using Google DeepMind's Antigravity AI coding assistant, working in close collaboration with the human author.

---

## Overview

**NotepadX** is a modern, lightweight native desktop text editor designed specifically for Linux. It combines the speed, memory safety, and concurrency guarantees of **Rust** with the sleek, modern design language of **GTK4** and **Libadwaita**.

Unlike heavy Electron-based or web-wrapped editors, NotepadX launches instantly, maintains a tiny memory footprint, respects system desktop conventions, and guarantees durable safety for your documents.

---

## Key Features

### 🔤 First-Class Unicode & Complex Script Support
- **Full complex-script rendering & editing**: Built and verified for flawless shaping and cursor navigation in Southeast Asian scripts such as **Burmese / Myanmar** (မင်္ဂလာပါ), **Thai** (ภาษาไทย), and **Khmer** (ភាសាខ្មែរ).
- **Grapheme cluster accuracy**: Robust handling of Unicode combining marks, zero-width characters, and multi-byte emoji.
- **Line ending & encoding preservation**: Automatically detects and cleanly preserves `LF` (Unix) and `CRLF` (Windows) line endings without silent normalization or corruption.

### 🛡️ Crash-Safe Durable Recovery
- **Zero data loss design**: Periodically writes atomic document snapshots and transaction journals to user-state storage.
- **Crash consistency**: If your machine reboots or power is interrupted, NotepadX automatically recovers unsaved tabs, buffers, and cursor positions upon next launch.
- **Atomic file saves**: Saves replace files via temporary files and sync calls to prevent file truncation or torn writes.

### 📑 Modern Multi-Tab Experience
- Open, edit, and switch between multiple documents simultaneously.
- Clean tab management with close buttons, modified state indicators (`*`), and tab overview.
- Smart keyboard shortcuts for rapid navigation (`Ctrl+N`, `Ctrl+O`, `Ctrl+S`, `Ctrl+W`, `Ctrl+Shift+S`, etc.).

### ⚙️ Live Font & Typography Customization
- **Font Settings dialog**: Easily customize font family, point size, and relative line-height multiplier.
- **Transparent overlay**: The modal overlay backdrop is transparent so you can see live typography updates on your actual document text in real time.
- **Zoom shortcuts**: Instant keyboard scaling (`Ctrl++` to zoom in, `Ctrl+-` to zoom out, `Ctrl+0` to reset).

### 🎨 Native GNOME & Libadwaita Integration
- **Automatic theme adaptation**: Automatically follows system light and dark mode preferences without requiring manual theme toggles.
- **Responsive design**: Uses native Libadwaita widgets (`ToolbarView`, `PreferencesDialog`, `ActionRow`, `SpinRow`).

### 💻 Syntax Highlighting & Language Detection
- Powered by **GtkSourceView 5**.
- Supports syntax highlighting for popular programming and markup languages (Rust, Python, C, C++, JavaScript, TypeScript, HTML, CSS, Markdown, Shell, and more).
- Language auto-detection from file extensions with manual language override via **Settings > Languages**.
- Toggleable line numbers, current line highlight, and syntax highlighting.

### 🔍 Find and Replace
- Integrated floating search and replace bar (`Ctrl+F`, `Ctrl+H`).
- Case-sensitive search option.
- Smooth forward and backward match navigation with match count notifications.

---

## Keyboard Shortcuts

| Shortcut | Action |
| :--- | :--- |
| `Ctrl + N` | New Tab |
| `Ctrl + O` | Open File |
| `Ctrl + S` | Save File |
| `Ctrl + Shift + S` | Save As... |
| `Ctrl + W` | Close Tab |
| `Ctrl + F` | Find |
| `Ctrl + H` | Replace |
| `Ctrl + +` / `Ctrl + =` | Zoom In |
| `Ctrl + -` | Zoom Out |
| `Ctrl + 0` | Reset Zoom |
| `Escape` | Close Find/Replace bar |

---

## Requirements & External Dependencies

NotepadX relies on standard native Linux GNOME libraries for high-performance rendering, complex-script shaping, and syntax highlighting. 

> [!IMPORTANT]  
> **Note on External Libraries**: In accordance with open-source best practices, C runtime libraries, development headers, and build tools are **not** bundled in this repository. Users and contributors install them on their own using their distribution's standard package manager as shown below.

### 1. Install System Dependencies

#### Debian / Ubuntu / Linux Mint / Pop!_OS
```bash
sudo apt update
sudo apt install -y pkg-config build-essential libgtk-4-dev libadwaita-1-dev libgtksourceview-5-dev
```

#### Fedora / RHEL
```bash
sudo dnf install -y pkgconf-pkg-config gcc gtk4-devel libadwaita-devel gtksourceview5-devel
```

#### Arch Linux / Manjaro
```bash
sudo pacman -S --needed base-devel pkgconf gtk4 libadwaita gtksourceview5
```

#### openSUSE
```bash
sudo zypper install -y -t pattern devel_basis
sudo zypper install -y pkg-config gtk4-devel libadwaita-devel gtksourceview5-devel
```

#### Alpine Linux
```bash
sudo apk add build-base pkgconf gtk4-dev libadwaita-dev gtksourceview5-dev
```

### 2. Rust Toolchain
Ensure you have a modern Rust toolchain installed (1.75+ recommended):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

---

## Building and Installing

### 1. Clone the repository
```bash
git clone https://github.com/your-username/notepadx.git
cd notepadx
```

### 2. Quick Automated Install (Recommended)
You can build and install NotepadX directly to your local user environment (`~/.local/bin` and desktop launcher):
```bash
./install.sh
```

### 3. Manual Build
To run in development mode:
```bash
cargo run
```

To compile an optimized release binary:
```bash
cargo build --release
```
The compiled binary will be placed at `target/release/notepadx`.

To run the full test suite:
```bash
cargo test
```

---

## Architecture & Technology Stack

- **Language**: [Rust](https://www.rust-lang.org/) (Safe, memory-efficient systems programming)
- **UI Toolkit**: [GTK4](https://www.gtk.org/) + [Libadwaita](https://gnome.pages.gitlab.gnome.org/libadwaita/)
- **Text Engine**: [GtkSourceView 5](https://gitlab.gnome.org/GNOME/gtksourceview)
- **Serialization**: [Serde](https://serde.rs/) (JSON persistence for recovery and user preferences)
- **State Storage**: Standard XDG directories (`~/.local/state/notepadx` for crash journals and `~/.config/notepadx` for settings)

---

## License

This project is open-source software licensed under the [MIT License](LICENSE).
