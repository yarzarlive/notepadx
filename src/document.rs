use crate::encoding::LineEnding;
use crate::recovery::DocumentSnapshot;
use gtk4::prelude::*;
use sourceview5::prelude::*;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub struct Document {
    pub id: String,
    pub title: String,
    pub file_path: Option<PathBuf>,
    pub file_mtime: Option<u64>,
    pub line_ending: LineEnding,
    pub is_modified: bool,
    pub buffer: sourceview5::Buffer,
    pub view: sourceview5::View,
    pub journal_seq: u64,
    pub last_synced_content: String,
    pub language_id: Option<String>,
}

pub fn setup_style_scheme(buffer: &sourceview5::Buffer) {
    let sm = sourceview5::StyleSchemeManager::default();
    let style_mgr = libadwaita::StyleManager::default();
    let scheme_id = if style_mgr.is_dark() {
        "Adwaita-dark"
    } else {
        "Adwaita"
    };
    if let Some(scheme) = sm.scheme(scheme_id) {
        buffer.set_style_scheme(Some(&scheme));
    }
}

fn create_source_view(buffer: &sourceview5::Buffer) -> sourceview5::View {
    sourceview5::View::builder()
        .buffer(buffer)
        .monospace(true)
        .show_line_numbers(true)
        .highlight_current_line(true)
        .wrap_mode(gtk4::WrapMode::WordChar)
        .left_margin(12)
        .right_margin(12)
        .top_margin(8)
        .bottom_margin(8)
        .vexpand(true)
        .hexpand(true)
        .tab_width(4)
        .indent_width(4)
        .auto_indent(true)
        .build()
}

impl Document {
    pub fn new_empty(title: String) -> Self {
        let buffer = sourceview5::Buffer::new(None);
        buffer.set_enable_undo(true);
        buffer.set_highlight_syntax(true);
        setup_style_scheme(&buffer);

        let view = create_source_view(&buffer);

        Self {
            id: Uuid::new_v4().to_string(),
            title,
            file_path: None,
            file_mtime: None,
            line_ending: LineEnding::Lf,
            is_modified: false,
            buffer,
            view,
            journal_seq: 0,
            last_synced_content: String::new(),
            language_id: None,
        }
    }

    pub fn from_file<P: AsRef<Path>>(path: P, content: &str, line_ending: LineEnding) -> Self {
        let path_ref = path.as_ref();
        let title = path_ref
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled")
            .to_string();

        let mtime = std::fs::metadata(path_ref)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs());

        let buffer = sourceview5::Buffer::new(None);
        buffer.set_text(content);
        buffer.set_enable_undo(true);
        buffer.set_highlight_syntax(true);
        setup_style_scheme(&buffer);

        let lm = sourceview5::LanguageManager::default();
        let guessed_lang = lm.guess_language(Some(path_ref), None);
        let language_id = if let Some(ref lang) = guessed_lang {
            buffer.set_language(Some(lang));
            Some(lang.id().to_string())
        } else {
            None
        };

        let view = create_source_view(&buffer);

        Self {
            id: Uuid::new_v4().to_string(),
            title,
            file_path: Some(path_ref.to_path_buf()),
            file_mtime: mtime,
            line_ending,
            is_modified: false,
            buffer,
            view,
            journal_seq: 0,
            last_synced_content: content.to_string(),
            language_id,
        }
    }

    pub fn from_snapshot(snapshot: DocumentSnapshot) -> Self {
        let line_ending = if snapshot.line_ending == "Crlf" {
            LineEnding::Crlf
        } else {
            LineEnding::Lf
        };

        let buffer = sourceview5::Buffer::new(None);
        buffer.set_text(&snapshot.content);
        buffer.set_enable_undo(true);
        buffer.set_highlight_syntax(true);
        setup_style_scheme(&buffer);

        let lm = sourceview5::LanguageManager::default();
        let lang = if let Some(ref lang_id) = snapshot.language_id {
            lm.language(lang_id)
        } else if let Some(ref path) = snapshot.file_path {
            lm.guess_language(Some(path), None)
        } else {
            None
        };

        let language_id = if let Some(ref l) = lang {
            buffer.set_language(Some(l));
            Some(l.id().to_string())
        } else {
            None
        };

        let view = create_source_view(&buffer);

        Self {
            id: snapshot.id,
            title: snapshot.title,
            file_path: snapshot.file_path,
            file_mtime: snapshot.file_mtime,
            line_ending,
            is_modified: snapshot.is_modified,
            buffer,
            view,
            journal_seq: 0,
            last_synced_content: snapshot.content,
            language_id,
        }
    }

    pub fn get_text(&self) -> String {
        let start = self.buffer.start_iter();
        let end = self.buffer.end_iter();
        self.buffer.text(&start, &end, false).to_string()
    }

    pub fn to_snapshot(&self) -> DocumentSnapshot {
        let content = self.get_text();
        let cursor_iter = self.buffer.iter_at_mark(&self.buffer.get_insert());
        let cursor_pos = cursor_iter.offset();

        let line_ending_str = match self.line_ending {
            LineEnding::Lf => "Lf",
            LineEnding::Crlf => "Crlf",
        };

        DocumentSnapshot {
            id: self.id.clone(),
            title: self.title.clone(),
            file_path: self.file_path.clone(),
            file_mtime: self.file_mtime,
            is_modified: self.is_modified,
            cursor_pos,
            line_ending: line_ending_str.to_string(),
            timestamp_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            content,
            language_id: self.language_id.clone(),
        }
    }

    pub fn mark_saved(&mut self, path: PathBuf, line_ending: LineEnding) {
        let title = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled")
            .to_string();

        let mtime = std::fs::metadata(&path)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs());

        if self.language_id.is_none() {
            let lm = sourceview5::LanguageManager::default();
            if let Some(lang) = lm.guess_language(Some(&path), None) {
                self.buffer.set_language(Some(&lang));
                self.language_id = Some(lang.id().to_string());
            }
        }

        self.file_path = Some(path);
        self.file_mtime = mtime;
        self.title = title;
        self.line_ending = line_ending;
        self.is_modified = false;
        self.last_synced_content = self.get_text();
    }
}
