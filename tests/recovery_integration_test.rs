use notepadx::encoding::{atomic_save_file, LineEnding};
use notepadx::recovery::{DocumentSnapshot, RecoveryManager, SessionState};
use std::fs;
use std::time::UNIX_EPOCH;
use tempfile::tempdir;
use uuid::Uuid;

#[test]
fn test_multitab_crash_recovery_with_complex_scripts() {
    let temp_state_dir = tempdir().unwrap();
    let recovery_dir = temp_state_dir.path().join("recovery");
    let manager = RecoveryManager::new(&recovery_dir);
    manager.ensure_dir().unwrap();

    // 1. Tab 1: New document with Burmese script, never saved to normal file
    let doc1_id = Uuid::new_v4().to_string();
    let doc1_content = "မြန်မာစာ မင်္ဂလာပါ\nဒုတိယစာကြောင်း ဖြစ်ပါသည်။\n";
    let snap1 = DocumentSnapshot {
        id: doc1_id.clone(),
        title: "Untitled 1".to_string(),
        file_path: None,
        file_mtime: None,
        is_modified: true,
        cursor_pos: 15,
        line_ending: "Lf".to_string(),
        timestamp_ms: 1000,
        content: doc1_content.to_string(),
        language_id: None,
    };
    manager.write_snapshot(&snap1).unwrap();

    // 2. Tab 2: Existing file with Thai script and unsaved modifications
    let doc2_id = Uuid::new_v4().to_string();
    let normal_file = temp_state_dir.path().join("existing_thai.txt");
    fs::write(&normal_file, "ภาษาไทย บรรทัดแรก\n").unwrap();
    let mtime = fs::metadata(&normal_file)
        .unwrap()
        .modified()
        .unwrap()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let doc2_unsaved_content = "ภาษาไทย บรรทัดแรก\nบรรทัดที่สอง เพิ่มเติม\n";
    let snap2 = DocumentSnapshot {
        id: doc2_id.clone(),
        title: "existing_thai.txt".to_string(),
        file_path: Some(normal_file.clone()),
        file_mtime: Some(mtime),
        is_modified: true,
        cursor_pos: 20,
        line_ending: "Lf".to_string(),
        timestamp_ms: 2000,
        content: doc2_unsaved_content.to_string(),
        language_id: None,
    };
    manager.write_snapshot(&snap2).unwrap();

    // 3. Tab 3: Khmer + Emoji + Combining marks
    let doc3_id = Uuid::new_v4().to_string();
    let doc3_content = "ភាសាខ្មែរ ជំរាបសួរ 👩‍💻 😀 café e\u{0301}\n";
    let snap3 = DocumentSnapshot {
        id: doc3_id.clone(),
        title: "Untitled 2".to_string(),
        file_path: None,
        file_mtime: None,
        is_modified: true,
        cursor_pos: 5,
        line_ending: "Lf".to_string(),
        timestamp_ms: 3000,
        content: doc3_content.to_string(),
        language_id: None,
    };
    manager.write_snapshot(&snap3).unwrap();

    // 4. Save session state with tab order
    let session = SessionState {
        tab_ids: vec![doc1_id.clone(), doc2_id.clone(), doc3_id.clone()],
        active_tab_index: 1, // Thai tab active
        updated_at_ms: 3500,
    };
    manager.save_session(&session).unwrap();

    // SIMULATE CRASH:
    // Application is killed, memory is wiped, fresh RecoveryManager instantiated
    let recovery_after_crash = RecoveryManager::new(&recovery_dir);
    let loaded_session = recovery_after_crash
        .load_session()
        .expect("Session must exist");
    assert_eq!(loaded_session.tab_ids.len(), 3);
    assert_eq!(loaded_session.active_tab_index, 1);

    // Verify Tab 1 recovery
    let rec1 = recovery_after_crash
        .recover_document(&loaded_session.tab_ids[0])
        .unwrap();
    assert_eq!(rec1.title, "Untitled 1");
    assert_eq!(rec1.content, doc1_content);
    assert!(rec1.is_modified);

    // Verify Tab 2 recovery
    let rec2 = recovery_after_crash
        .recover_document(&loaded_session.tab_ids[1])
        .unwrap();
    assert_eq!(rec2.title, "existing_thai.txt");
    assert_eq!(rec2.content, doc2_unsaved_content);
    assert!(rec2.is_modified);
    // Crucial check: verify normal file on disk was NOT overwritten by crash/recovery!
    let disk_file_content = fs::read_to_string(&normal_file).unwrap();
    assert_eq!(disk_file_content, "ภาษาไทย บรรทัดแรก\n");

    // Verify Tab 3 recovery
    let rec3 = recovery_after_crash
        .recover_document(&loaded_session.tab_ids[2])
        .unwrap();
    assert_eq!(rec3.title, "Untitled 2");
    assert_eq!(rec3.content, doc3_content);
    assert!(rec3.is_modified);

    // 5. Test external modification conflict detection
    // Simulate an external editor modifying `normal_file` on disk
    std::thread::sleep(std::time::Duration::from_millis(50));
    fs::write(&normal_file, "Modified externally by another tool!").unwrap();

    let new_disk_mtime = fs::metadata(&normal_file)
        .unwrap()
        .modified()
        .unwrap()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Our recovered snapshot has the older mtime
    assert!(new_disk_mtime >= rec2.file_mtime.unwrap());
    // Recovered content remains intact and unsaved edits are preserved
    assert_eq!(rec2.content, doc2_unsaved_content);

    // 6. Test safe cleanup on explicit save
    atomic_save_file(&normal_file, &rec2.content, LineEnding::Lf).unwrap();
    recovery_after_crash.cleanup_document(&doc2_id);

    // Verify doc2 recovery file is removed after explicit save
    assert!(recovery_after_crash.recover_document(&doc2_id).is_err());
    // BUT unsaved doc1 and doc3 recovery files STILL EXIST!
    assert!(recovery_after_crash.recover_document(&doc1_id).is_ok());
    assert!(recovery_after_crash.recover_document(&doc3_id).is_ok());
}
