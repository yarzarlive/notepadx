use crate::error::{NotepadxError, Result};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    Crlf,
}

/// Detect whether bytes represent likely binary content.
/// Standard heuristic: presence of null bytes (`0x00`) in the inspected slice,
/// or high ratio of non-printable control characters.
pub fn is_binary_content(bytes: &[u8]) -> bool {
    let inspect_len = bytes.len().min(8192);
    let slice = &bytes[..inspect_len];

    if slice.contains(&0) {
        return true;
    }

    // Check ratio of control characters (excluding common whitespace: \t, \n, \r, \x0c)
    let mut control_count = 0;
    for &b in slice {
        if b < 0x20 && b != b'\t' && b != b'\n' && b != b'\r' && b != 0x0C {
            control_count += 1;
        }
    }

    if inspect_len > 0 && (control_count * 100 / inspect_len) > 20 {
        return true;
    }

    false
}

/// Detect primary line ending used in text.
pub fn detect_line_ending(text: &str) -> LineEnding {
    let mut crlf_count = 0;
    let mut lf_count = 0;
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\r' {
            if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                crlf_count += 1;
                i += 2;
                continue;
            }
        } else if bytes[i] == b'\n' {
            lf_count += 1;
        }
        i += 1;
    }

    if crlf_count > lf_count {
        LineEnding::Crlf
    } else {
        LineEnding::Lf
    }
}

/// Normalize text to internal representation (LF) while recording original line ending.
pub fn to_internal_text(content: &str) -> (String, LineEnding) {
    let detected = detect_line_ending(content);
    if detected == LineEnding::Crlf {
        // Replace \r\n with \n
        (content.replace("\r\n", "\n"), LineEnding::Crlf)
    } else {
        (content.to_string(), LineEnding::Lf)
    }
}

/// Convert internal LF text back to target line ending for saving.
pub fn to_external_bytes(text: &str, line_ending: LineEnding) -> Vec<u8> {
    match line_ending {
        LineEnding::Lf => text.as_bytes().to_vec(),
        LineEnding::Crlf => {
            // Replace \n with \r\n (avoiding doubling existing \r\n)
            let mut out = String::with_capacity(text.len() + text.len() / 20);
            let mut chars = text.chars().peekable();
            while let Some(ch) = chars.next() {
                if ch == '\r' {
                    if chars.peek() == Some(&'\n') {
                        out.push_str("\r\n");
                        chars.next();
                    } else {
                        out.push('\r');
                    }
                } else if ch == '\n' {
                    out.push_str("\r\n");
                } else {
                    out.push(ch);
                }
            }
            out.into_bytes()
        }
    }
}

/// Read text file with encoding validation and binary check.
pub fn read_text_file<P: AsRef<Path>>(path: P) -> Result<(String, LineEnding)> {
    let path_ref = path.as_ref();
    let mut file = File::open(path_ref)?;
    let mut raw_bytes = Vec::new();
    file.read_to_end(&mut raw_bytes)?;

    if is_binary_content(&raw_bytes) {
        return Err(NotepadxError::BinaryFile(path_ref.to_path_buf()));
    }

    let text = match std::str::from_utf8(&raw_bytes) {
        Ok(valid_str) => valid_str,
        Err(_) => return Err(NotepadxError::InvalidUtf8(path_ref.to_path_buf())),
    };

    Ok(to_internal_text(text))
}

/// Durably save text to a file using atomic write, sync_all, rename, and directory sync.
pub fn atomic_save_file<P: AsRef<Path>>(
    path: P,
    content: &str,
    line_ending: LineEnding,
) -> Result<()> {
    let target_path = path.as_ref();
    let parent_dir = target_path
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    // Ensure parent directory exists
    fs::create_dir_all(&parent_dir)?;

    let temp_name = format!(
        ".{}.tmp-{}",
        target_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("notepadx_save"),
        uuid::Uuid::new_v4()
    );
    let temp_path = parent_dir.join(temp_name);

    let bytes = to_external_bytes(content, line_ending);

    // Write content to temporary file
    {
        let mut temp_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)?;

        temp_file.write_all(&bytes)?;
        temp_file.flush()?;
        // Durable sync to disk storage
        temp_file.sync_all()?;
    }

    // Atomic rename
    if let Err(e) = fs::rename(&temp_path, target_path) {
        let _ = fs::remove_file(&temp_path);
        return Err(NotepadxError::Io(e));
    }

    // Synchronize parent directory to guarantee directory entry persistence
    if let Ok(dir_file) = File::open(&parent_dir) {
        let _ = dir_file.sync_all();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_binary_detection() {
        let text = b"Hello, this is a plain text file without nulls.\n";
        assert!(!is_binary_content(text));

        let mut binary = text.to_vec();
        binary.push(0x00);
        binary.extend_from_slice(b"binary data");
        assert!(is_binary_content(&binary));
    }

    #[test]
    fn test_line_ending_detection() {
        assert_eq!(detect_line_ending("Line 1\nLine 2\n"), LineEnding::Lf);
        assert_eq!(detect_line_ending("Line 1\r\nLine 2\r\n"), LineEnding::Crlf);
        assert_eq!(
            detect_line_ending("Line 1\r\nLine 2\nLine 3\r\n"),
            LineEnding::Crlf
        );
    }

    #[test]
    fn test_complex_script_preservation() {
        // Myanmar, Thai, Khmer, emoji, combining sequences
        let samples = vec![
            "မြန်မာစာ မင်္ဂလာပါ",
            "ภาษาไทย",
            "ភាសាខ្មែរ",
            "café e\u{0301} 👩‍💻 😀",
            "Mixed: မြန်မာစာ + Thai ภาษาไทย + Khmer ភាសាខ្မែရ + Symbols 💡 123",
        ];

        for sample in samples {
            let (internal, _) = to_internal_text(sample);
            assert_eq!(internal, sample);

            let bytes = to_external_bytes(&internal, LineEnding::Lf);
            let restored = std::str::from_utf8(&bytes).unwrap();
            assert_eq!(restored, sample);
        }
    }

    #[test]
    fn test_atomic_save_and_read() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_save.txt");

        let content = "First line\nမြန်မာစာ\nภาษาไทย\nភាសាខ្មែរ\n";
        atomic_save_file(&file_path, content, LineEnding::Lf).unwrap();

        let (read_content, line_ending) = read_text_file(&file_path).unwrap();
        assert_eq!(read_content, content);
        assert_eq!(line_ending, LineEnding::Lf);
    }

    #[test]
    fn test_crlf_preservation() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("crlf.txt");

        let content = "Line 1\r\nLine 2\r\nLine 3\r\n";
        fs::write(&file_path, content.as_bytes()).unwrap();

        let (read_content, line_ending) = read_text_file(&file_path).unwrap();
        assert_eq!(line_ending, LineEnding::Crlf);
        assert_eq!(read_content, "Line 1\nLine 2\nLine 3\n");

        let save_path = dir.path().join("crlf_saved.txt");
        atomic_save_file(&save_path, &read_content, line_ending).unwrap();

        let raw = fs::read(&save_path).unwrap();
        assert_eq!(raw, content.as_bytes());
    }
}
