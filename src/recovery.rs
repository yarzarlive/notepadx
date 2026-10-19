use crate::error::{NotepadxError, Result};
use crc32fast::Hasher;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

const JOURNAL_MAGIC: &[u8; 4] = b"NXJR";
const RECORD_HEADER_SIZE: usize = 4 + 8 + 1 + 8 + 4 + 4; // 29 bytes

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSnapshot {
    pub id: String,
    pub title: String,
    pub file_path: Option<PathBuf>,
    pub file_mtime: Option<u64>,
    pub is_modified: bool,
    pub cursor_pos: i32,
    pub line_ending: String,
    pub timestamp_ms: u64,
    pub content: String,
    #[serde(default)]
    pub language_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub tab_ids: Vec<String>,
    pub active_tab_index: usize,
    pub updated_at_ms: u64,
}

pub struct RecoveryManager {
    recovery_dir: PathBuf,
}

impl RecoveryManager {
    pub fn new<P: AsRef<Path>>(dir: P) -> Self {
        Self {
            recovery_dir: dir.as_ref().to_path_buf(),
        }
    }

    pub fn default_dir() -> PathBuf {
        let base = dirs::state_dir().unwrap_or_else(|| {
            dirs::home_dir()
                .map(|h| h.join(".local/state"))
                .unwrap_or_else(|| PathBuf::from("."))
        });
        base.join("notepadx").join("recovery")
    }

    pub fn default_manager() -> Self {
        Self::new(Self::default_dir())
    }

    #[allow(dead_code)]
    pub fn recovery_dir(&self) -> &Path {
        &self.recovery_dir
    }

    pub fn ensure_dir(&self) -> Result<()> {
        if !self.recovery_dir.exists() {
            fs::create_dir_all(&self.recovery_dir)?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    fn snapshot_path(&self, doc_id: &str) -> PathBuf {
        self.recovery_dir.join(format!("doc_{}.snap", doc_id))
    }

    fn journal_path(&self, doc_id: &str) -> PathBuf {
        self.recovery_dir.join(format!("doc_{}.journal", doc_id))
    }

    fn session_path(&self) -> PathBuf {
        self.recovery_dir.join("session.json")
    }

    /// Durably save a document snapshot.
    /// Invariant: Write to temp file -> fsync -> atomic rename -> fsync parent dir.
    pub fn write_snapshot(&self, snapshot: &DocumentSnapshot) -> Result<()> {
        self.ensure_dir()?;
        let target_path = self.snapshot_path(&snapshot.id);
        let temp_path =
            self.recovery_dir
                .join(format!("doc_{}.snap.tmp-{}", snapshot.id, Uuid::new_v4()));

        let json_bytes = serde_json::to_vec(snapshot)?;

        // Write temp snapshot file and sync
        {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp_path)?;
            file.write_all(&json_bytes)?;
            file.flush()?;
            file.sync_all()?;
        }

        // Atomic replace
        if let Err(e) = fs::rename(&temp_path, &target_path) {
            let _ = fs::remove_file(&temp_path);
            return Err(NotepadxError::Io(e));
        }

        // Synchronize recovery directory to guarantee directory entry persistence
        if let Ok(dir_file) = File::open(&self.recovery_dir) {
            let _ = dir_file.sync_all();
        }

        // Truncate journal file after successful snapshot since snapshot has latest content
        let journal_file_path = self.journal_path(&snapshot.id);
        if journal_file_path.exists() {
            let _ = fs::remove_file(&journal_file_path);
        }

        Ok(())
    }

    /// Append an incremental edit record to the document's write-ahead journal.
    /// Framed format: [MAGIC(4B)][SEQ(8B)][RECORD_TYPE(1B)][TIMESTAMP(8B)][LEN(4B)][CRC32(4B)][PAYLOAD]
    #[allow(dead_code)]
    pub fn append_journal_record(&self, doc_id: &str, seq: u64, content: &str) -> Result<()> {
        self.ensure_dir()?;
        let path = self.journal_path(doc_id);
        let mut file = OpenOptions::new().create(true).append(true).open(&path)?;

        let payload = content.as_bytes();
        let payload_len = payload.len() as u32;

        let mut hasher = Hasher::new();
        hasher.update(payload);
        let crc = hasher.finalize();

        let mut record = Vec::with_capacity(RECORD_HEADER_SIZE + payload.len());
        record.extend_from_slice(JOURNAL_MAGIC);
        record.extend_from_slice(&seq.to_le_bytes());
        record.push(1); // 1 = Full text update
        record.extend_from_slice(&Self::now_ms().to_le_bytes());
        record.extend_from_slice(&payload_len.to_le_bytes());
        record.extend_from_slice(&crc.to_le_bytes());
        record.extend_from_slice(payload);

        file.write_all(&record)?;
        file.flush()?;
        // Sync journal append
        file.sync_data()?;

        Ok(())
    }

    /// Read document state by loading snapshot and replaying valid journal records.
    pub fn recover_document(&self, doc_id: &str) -> Result<DocumentSnapshot> {
        let snap_path = self.snapshot_path(doc_id);
        if !snap_path.exists() {
            return Err(NotepadxError::DocumentNotFound(doc_id.to_string()));
        }

        let snap_bytes = fs::read(&snap_path)?;
        let mut snapshot: DocumentSnapshot = serde_json::from_slice(&snap_bytes)?;

        // Replay any valid records from journal if present
        let journal_path = self.journal_path(doc_id);
        if journal_path.exists() {
            if let Ok(mut jfile) = File::open(&journal_path) {
                let mut buffer = Vec::new();
                if jfile.read_to_end(&mut buffer).is_ok() {
                    let mut cursor = 0;
                    while cursor + RECORD_HEADER_SIZE <= buffer.len() {
                        let magic = &buffer[cursor..cursor + 4];
                        if magic != JOURNAL_MAGIC {
                            // Invalid magic or torn record, stop replay safely
                            break;
                        }
                        let _seq =
                            u64::from_le_bytes(buffer[cursor + 4..cursor + 12].try_into().unwrap());
                        let _rec_type = buffer[cursor + 12];
                        let ts = u64::from_le_bytes(
                            buffer[cursor + 13..cursor + 21].try_into().unwrap(),
                        );
                        let len = u32::from_le_bytes(
                            buffer[cursor + 21..cursor + 25].try_into().unwrap(),
                        ) as usize;
                        let expected_crc = u32::from_le_bytes(
                            buffer[cursor + 25..cursor + 29].try_into().unwrap(),
                        );

                        let payload_start = cursor + RECORD_HEADER_SIZE;
                        let payload_end = payload_start + len;

                        if payload_end > buffer.len() {
                            // Incomplete/torn payload write at end, stop replay
                            break;
                        }

                        let payload = &buffer[payload_start..payload_end];
                        let mut hasher = Hasher::new();
                        hasher.update(payload);
                        if hasher.finalize() != expected_crc {
                            // Corrupted payload, stop replay
                            break;
                        }

                        if let Ok(text) = std::str::from_utf8(payload) {
                            snapshot.content = text.to_string();
                            snapshot.is_modified = true;
                            snapshot.timestamp_ms = ts;
                        }

                        cursor = payload_end;
                    }
                }
            }
        }

        Ok(snapshot)
    }

    /// Durably save session metadata (tab order and active tab).
    pub fn save_session(&self, session: &SessionState) -> Result<()> {
        self.ensure_dir()?;
        let target = self.session_path();
        let temp = self
            .recovery_dir
            .join(format!("session.json.tmp-{}", Uuid::new_v4()));

        let json = serde_json::to_vec_pretty(session)?;
        {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)?;
            file.write_all(&json)?;
            file.flush()?;
            file.sync_all()?;
        }

        if let Err(e) = fs::rename(&temp, &target) {
            let _ = fs::remove_file(&temp);
            return Err(NotepadxError::Io(e));
        }

        if let Ok(dir_file) = File::open(&self.recovery_dir) {
            let _ = dir_file.sync_all();
        }

        Ok(())
    }

    /// Load saved session metadata.
    pub fn load_session(&self) -> Option<SessionState> {
        let path = self.session_path();
        if path.exists() {
            if let Ok(bytes) = fs::read(&path) {
                if let Ok(session) = serde_json::from_slice::<SessionState>(&bytes) {
                    return Some(session);
                }
            }
        }
        None
    }

    /// Remove recovery data for a cleanly closed or saved document.
    pub fn cleanup_document(&self, doc_id: &str) {
        let snap = self.snapshot_path(doc_id);
        let journal = self.journal_path(doc_id);
        let _ = fs::remove_file(snap);
        let _ = fs::remove_file(journal);
    }

    /// Discover all recoverable document IDs in the recovery directory.
    pub fn list_recoverable_docs(&self) -> Vec<String> {
        let mut doc_ids = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.recovery_dir) {
            for entry in entries.flatten() {
                if let Some(file_name) = entry.file_name().to_str() {
                    if file_name.starts_with("doc_") && file_name.ends_with(".snap") {
                        let id = &file_name[4..file_name.len() - 5];
                        doc_ids.push(id.to_string());
                    }
                }
            }
        }
        doc_ids
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_atomic_snapshot_recovery() {
        let dir = tempdir().unwrap();
        let manager = RecoveryManager::new(dir.path());

        let snapshot = DocumentSnapshot {
            id: Uuid::new_v4().to_string(),
            title: "Untitled 1".to_string(),
            file_path: None,
            file_mtime: None,
            is_modified: true,
            cursor_pos: 10,
            line_ending: "Lf".to_string(),
            timestamp_ms: RecoveryManager::now_ms(),
            content: "Recovered test text\nမြန်မာစာ\nภาษาไทย\n".to_string(),
            language_id: None,
        };

        manager.write_snapshot(&snapshot).unwrap();
        assert!(manager.recovery_dir().exists());

        let recovered = manager.recover_document(&snapshot.id).unwrap();
        assert_eq!(recovered.id, snapshot.id);
        assert_eq!(recovered.content, snapshot.content);
        assert!(recovered.is_modified);
    }

    #[test]
    fn test_journal_replay_and_torn_record() {
        let dir = tempdir().unwrap();
        let manager = RecoveryManager::new(dir.path());
        let doc_id = Uuid::new_v4().to_string();

        let snapshot = DocumentSnapshot {
            id: doc_id.clone(),
            title: "Test Journal".to_string(),
            file_path: None,
            file_mtime: None,
            is_modified: true,
            cursor_pos: 0,
            line_ending: "Lf".to_string(),
            timestamp_ms: RecoveryManager::now_ms(),
            content: "Initial snapshot".to_string(),
            language_id: None,
        };
        manager.write_snapshot(&snapshot).unwrap();

        // Append two valid journal records
        manager.append_journal_record(&doc_id, 1, "Edit 1").unwrap();
        manager
            .append_journal_record(&doc_id, 2, "Edit 2: မြန်မာစာ")
            .unwrap();

        // Verify recovery has Edit 2
        let recovered = manager.recover_document(&doc_id).unwrap();
        assert_eq!(recovered.content, "Edit 2: မြန်မာစာ");

        // Simulate a torn/interrupted write at the end of journal
        let jpath = manager.journal_path(&doc_id);
        {
            let mut f = OpenOptions::new().append(true).open(&jpath).unwrap();
            // Incomplete header bytes
            f.write_all(b"NXJR\x03\x00\x00").unwrap();
            f.flush().unwrap();
        }

        // Replay should safely ignore the torn record and still recover Edit 2!
        let recovered_after_torn = manager.recover_document(&doc_id).unwrap();
        assert_eq!(recovered_after_torn.content, "Edit 2: မြန်မာစာ");
    }

    #[test]
    fn test_session_persistence() {
        let dir = tempdir().unwrap();
        let manager = RecoveryManager::new(dir.path());

        let session = SessionState {
            tab_ids: vec!["doc-1".to_string(), "doc-2".to_string()],
            active_tab_index: 1,
            updated_at_ms: RecoveryManager::now_ms(),
        };

        manager.save_session(&session).unwrap();

        let loaded = manager.load_session().unwrap();
        assert_eq!(loaded.tab_ids, session.tab_ids);
        assert_eq!(loaded.active_tab_index, 1);
    }

    #[test]
    fn test_cleanup_removes_data() {
        let dir = tempdir().unwrap();
        let manager = RecoveryManager::new(dir.path());
        let doc_id = "test-clean";

        let snapshot = DocumentSnapshot {
            id: doc_id.to_string(),
            title: "Untitled".to_string(),
            file_path: None,
            file_mtime: None,
            is_modified: false,
            cursor_pos: 0,
            line_ending: "Lf".to_string(),
            timestamp_ms: 0,
            content: "Saved".to_string(),
            language_id: None,
        };
        manager.write_snapshot(&snapshot).unwrap();
        assert!(manager.snapshot_path(doc_id).exists());

        manager.cleanup_document(doc_id);
        assert!(!manager.snapshot_path(doc_id).exists());
    }
}
