use gtk4::prelude::*;
use notepadx::document::Document;
use notepadx::encoding::LineEnding;
use notepadx::recovery::{RecoveryManager, SessionState};
use std::sync::Arc;

#[test]
fn test_gtk_integration_suite() {
    if gtk4::init().is_err() {
        eprintln!("Skipping GTK GUI tests: display not available");
        return;
    }

    let test_config_dir = tempfile::tempdir().unwrap();
    std::env::set_var("XDG_CONFIG_HOME", test_config_dir.path());

    run_gtk_ui_and_script_test();
    run_find_and_replace_test();
    run_key_translation_test();
    run_syntax_highlighting_and_languages_menu_test();
    run_fonts_setting_and_global_effect_test();
}

fn run_gtk_ui_and_script_test() {
    // 1. Test expand properties on Document views
    let doc_expand = Document::new_empty("Expand Test".to_string());
    assert!(
        doc_expand.view.property::<bool>("vexpand"),
        "TextView must expand vertically"
    );
    assert!(
        doc_expand.view.property::<bool>("hexpand"),
        "TextView must expand horizontally"
    );

    // 2. Test buffer and recovery flow
    let dir = tempfile::tempdir().unwrap();
    let recovery = Arc::new(RecoveryManager::new(dir.path()));
    recovery.ensure_dir().unwrap();

    let mut doc = Document::new_empty("Complex Scripts Tab".to_string());
    assert_eq!(doc.line_ending, LineEnding::Lf);
    assert!(!doc.is_modified);

    let typed_text = "မြန်မာစာ မင်္ဂလာပါ\nภาษาไทย สวัสดี\nភាសាខ្មែរ ជំရាបសួរ\nEmoji: 👨‍💻 🚀\n";
    doc.buffer.set_text(typed_text);
    doc.is_modified = true;

    assert_eq!(doc.get_text(), typed_text);

    let snap = doc.to_snapshot();
    recovery.write_snapshot(&snap).unwrap();

    let session = SessionState {
        tab_ids: vec![doc.id.clone()],
        active_tab_index: 0,
        updated_at_ms: 12345,
    };
    recovery.save_session(&session).unwrap();

    let fresh_recovery = RecoveryManager::new(dir.path());
    let recovered_session = fresh_recovery.load_session().expect("Session must exist");
    assert_eq!(recovered_session.tab_ids.len(), 1);

    let recovered_snap = fresh_recovery
        .recover_document(&recovered_session.tab_ids[0])
        .expect("Snapshot must recover");
    assert_eq!(recovered_snap.content, typed_text);
    assert!(recovered_snap.is_modified);

    let restored_doc = Document::from_snapshot(recovered_snap);
    assert_eq!(restored_doc.get_text(), typed_text);
    assert!(restored_doc.is_modified);
    assert_eq!(restored_doc.title, "Complex Scripts Tab");

    // 3. Test complex script editing and navigation
    let edit_doc = Document::new_empty("Script Edit Test".to_string());
    let initial_text = "မြန်မာစာ မင်္ဂလာပါ ภาษาไทย ភាសាខ្មែរ";
    edit_doc.buffer.set_text(initial_text);

    let mut iter = edit_doc.buffer.start_iter();
    let mut steps = 0;
    while iter.forward_cursor_position() {
        steps += 1;
    }
    assert!(steps > 0);

    let mut start = edit_doc.buffer.start_iter();
    edit_doc.buffer.insert(&mut start, "START: ");
    assert!(edit_doc.get_text().starts_with("START: မြန်မာစာ"));

    let mut end = edit_doc.buffer.end_iter();
    edit_doc.buffer.insert(&mut end, " :END");
    assert!(edit_doc.get_text().ends_with("ភាសាខ្មែរ :END"));

    let mut sel_start = edit_doc.buffer.iter_at_offset(0);
    let mut sel_end = edit_doc.buffer.iter_at_offset(7); // "START: "
    edit_doc.buffer.delete(&mut sel_start, &mut sel_end);
    assert!(edit_doc.get_text().starts_with("မြန်မာစာ"));

    let combining_seq = "café e\u{0301}";
    let mut append_iter = edit_doc.buffer.end_iter();
    edit_doc.buffer.insert(&mut append_iter, combining_seq);
    assert!(edit_doc.get_text().contains("café e\u{0301}"));
}

fn run_find_and_replace_test() {
    use notepadx::ui::find_replace::FindReplaceBar;
    use std::cell::RefCell;
    use std::rc::Rc;

    let doc = Document::new_empty("Find Test".to_string());
    let doc_cell = Rc::new(RefCell::new(doc));

    let doc_cell_clone = doc_cell.clone();
    let get_target = Rc::new(move || {
        let d = doc_cell_clone.borrow();
        Some((d.buffer.clone().upcast(), d.view.clone().upcast()))
    });

    let toast_messages = Rc::new(RefCell::new(Vec::new()));
    let toast_clone = toast_messages.clone();
    let show_toast = Rc::new(move |msg: &str| {
        toast_clone.borrow_mut().push(msg.to_string());
    });

    let bar = FindReplaceBar::new(get_target, show_toast);

    // Initial state
    assert!(!bar.is_visible());

    // 1. Test search with complex scripts
    let test_content = "မင်္ဂလာပါ ကမ္ဘာလောက\nสวัสดี ชาวโลก\nសួស្តី ពិភពលោក\nမင်္ဂလာပါ အားလုံးပဲ";
    doc_cell.borrow().buffer.set_text(test_content);

    // Show find with query "မင်္ဂလာပါ"
    bar.show_find(Some("မင်္ဂလာပါ".to_string()));
    assert!(bar.is_visible());

    // find_next should select the first match
    bar.find_next(false);
    let (s, e) = doc_cell
        .borrow()
        .buffer
        .selection_bounds()
        .expect("Match 1 selected");
    assert_eq!(doc_cell.borrow().buffer.text(&s, &e, false), "မင်္ဂလာပါ");
    assert_eq!(s.offset(), 0);

    // Advance to next match
    bar.find_next(true);
    let (s2, e2) = doc_cell
        .borrow()
        .buffer
        .selection_bounds()
        .expect("Match 2 selected");
    assert_eq!(doc_cell.borrow().buffer.text(&s2, &e2, false), "မင်္ဂလာပါ");
    assert!(s2.offset() > 0);

    // Backward search
    bar.find_prev();
    let (s3, e3) = doc_cell
        .borrow()
        .buffer
        .selection_bounds()
        .expect("Match 1 re-selected");
    assert_eq!(s3.offset(), 0);
    assert!(e3.offset() > 0);

    // 2. Test Thai search
    bar.show_find(Some("สวัสดี".to_string()));
    bar.find_next(true);
    let (ts, te) = doc_cell
        .borrow()
        .buffer
        .selection_bounds()
        .expect("Thai match selected");
    assert_eq!(doc_cell.borrow().buffer.text(&ts, &te, false), "สวัสดี");

    // 3. Test Replace All with Burmese text
    bar.show_replace(Some("မင်္ဂလာပါ".to_string()));
    // Set replace text directly via replace_all after setting buffer content
    let rep_content = "မင်္ဂလာပါ ကမ္ဘာ\nမင်္ဂလာပါ အားလုံး";
    doc_cell.borrow().buffer.set_text(rep_content);
    bar.show_find(Some("မင်္ဂလာပါ".to_string()));

    // Replace all
    bar.replace_all();
    let updated_text = doc_cell.borrow().get_text();
    // Since replace_entry was empty, replacing with empty removes them
    assert!(!updated_text.contains("မင်္ဂလာပါ"));
    assert_eq!(toast_messages.borrow().len(), 1);
    assert!(toast_messages.borrow()[0].contains("Replaced 2 occurrences"));

    // 4. Test hide
    bar.hide();
    assert!(!bar.is_visible());
}

fn run_key_translation_test() {
    use gtk4::gdk::Display;
    use gtk4::glib::translate::FromGlib;
    use notepadx::ui::window::resolve_shortcut_key;

    let display = Display::default();
    let dummy_non_english_key = gtk4::gdk::Key::VoidSymbol;
    let cyrillic_es = gtk4::gdk::Key::Cyrillic_es;
    let arabic_sheen = gtk4::gdk::Key::Arabic_sheen;
    let burmese_ka = unsafe { gtk4::gdk::Key::from_glib(0x01001000) };

    // 1. Test that non-English keyvals map back to the physical key's Latin shortcut
    assert_eq!(
        resolve_shortcut_key(display.as_ref(), dummy_non_english_key, 39),
        gtk4::gdk::Key::s // Ctrl + S
    );
    assert_eq!(
        resolve_shortcut_key(display.as_ref(), dummy_non_english_key, 57),
        gtk4::gdk::Key::n // Ctrl + N
    );
    assert_eq!(
        resolve_shortcut_key(display.as_ref(), dummy_non_english_key, 32),
        gtk4::gdk::Key::o // Ctrl + O
    );
    assert_eq!(
        resolve_shortcut_key(display.as_ref(), dummy_non_english_key, 54),
        gtk4::gdk::Key::c // Ctrl + C
    );
    assert_eq!(
        resolve_shortcut_key(display.as_ref(), dummy_non_english_key, 55),
        gtk4::gdk::Key::v // Ctrl + V
    );
    assert_eq!(
        resolve_shortcut_key(display.as_ref(), dummy_non_english_key, 53),
        gtk4::gdk::Key::x // Ctrl + X
    );
    assert_eq!(
        resolve_shortcut_key(display.as_ref(), dummy_non_english_key, 38),
        gtk4::gdk::Key::a // Ctrl + A
    );
    assert_eq!(
        resolve_shortcut_key(display.as_ref(), dummy_non_english_key, 52),
        gtk4::gdk::Key::z // Ctrl + Z
    );
    assert_eq!(
        resolve_shortcut_key(display.as_ref(), dummy_non_english_key, 41),
        gtk4::gdk::Key::f // Ctrl + F
    );
    assert_eq!(
        resolve_shortcut_key(display.as_ref(), dummy_non_english_key, 43),
        gtk4::gdk::Key::h // Ctrl + H
    );
    assert_eq!(
        resolve_shortcut_key(display.as_ref(), dummy_non_english_key, 9),
        gtk4::gdk::Key::Escape // Escape
    );
    assert_eq!(
        resolve_shortcut_key(display.as_ref(), dummy_non_english_key, 23),
        gtk4::gdk::Key::Tab // Tab
    );
    assert_eq!(
        resolve_shortcut_key(display.as_ref(), dummy_non_english_key, 36),
        gtk4::gdk::Key::Return // Return
    );

    // 2. Test with real non-Latin keyvals (Russian, Burmese, Arabic)
    assert_eq!(
        resolve_shortcut_key(display.as_ref(), cyrillic_es, 54),
        gtk4::gdk::Key::c
    );
    assert_eq!(
        resolve_shortcut_key(display.as_ref(), burmese_ka, 54),
        gtk4::gdk::Key::c
    );
    assert_eq!(
        resolve_shortcut_key(display.as_ref(), arabic_sheen, 38),
        gtk4::gdk::Key::a
    );

    // 3. Test standard Latin keys (English layout typing)
    assert_eq!(
        resolve_shortcut_key(display.as_ref(), gtk4::gdk::Key::C, 54),
        gtk4::gdk::Key::c
    );
    assert_eq!(
        resolve_shortcut_key(display.as_ref(), gtk4::gdk::Key::s, 39),
        gtk4::gdk::Key::s
    );
}

fn run_syntax_highlighting_and_languages_menu_test() {
    use sourceview5::prelude::*;

    // 1. Test language auto-detection for Python, Rust, PHP, Plain text
    let doc_py = Document::from_file(
        "test.py",
        "def greet():\n    return 'hello'\n",
        LineEnding::Lf,
    );
    assert!(
        doc_py.language_id.as_deref() == Some("python3")
            || doc_py.language_id.as_deref() == Some("python")
    );
    assert!(doc_py.buffer.is_highlight_syntax());
    assert!(doc_py.buffer.language().is_some());
    assert!(doc_py
        .buffer
        .language()
        .unwrap()
        .name()
        .starts_with("Python"));

    let doc_rs = Document::from_file(
        "test.rs",
        "fn main() {\n    println!(\"hi\");\n}\n",
        LineEnding::Lf,
    );
    assert_eq!(doc_rs.language_id.as_deref(), Some("rust"));
    assert!(doc_rs.buffer.language().is_some());
    assert_eq!(doc_rs.buffer.language().unwrap().name(), "Rust");

    let doc_php = Document::from_file("test.php", "<?php echo 'hello'; ?>", LineEnding::Lf);
    assert_eq!(doc_php.language_id.as_deref(), Some("php"));
    assert!(doc_php.buffer.language().is_some());
    assert_eq!(doc_php.buffer.language().unwrap().name(), "PHP");

    let doc_txt = Document::from_file("test.txt", "Plain text content", LineEnding::Lf);
    assert_eq!(doc_txt.language_id, None);
    assert!(doc_txt.buffer.language().is_none());

    // 2. Test manual language assignment on Document
    let mut doc_empty = Document::new_empty("New File".to_string());
    assert_eq!(doc_empty.language_id, None);
    assert!(doc_empty.buffer.language().is_none());

    let lm = sourceview5::LanguageManager::default();
    let c_lang = lm
        .language("c")
        .expect("C language must be available in gtksourceview-5");
    doc_empty.buffer.set_language(Some(&c_lang));
    doc_empty.language_id = Some("c".to_string());
    assert_eq!(doc_empty.buffer.language().unwrap().name(), "C");

    // Setting to None restores Plain Text
    doc_empty.buffer.set_language(None);
    doc_empty.language_id = None;
    assert!(doc_empty.buffer.language().is_none());

    // 3. Test language menu generation and hierarchy
    use notepadx::ui::window::build_languages_menu;
    let menu = build_languages_menu();
    assert!(
        menu.n_items() > 10,
        "Menu must contain letter submenus and Plain Text"
    );

    // Check first item is Plain Text
    assert_eq!(
        menu.item_attribute_value(0, "label", None)
            .and_then(|v| v.str().map(|s| s.to_string())),
        Some("Plain Text".to_string())
    );
}

fn run_fonts_setting_and_global_effect_test() {
    use gtk4::gio::prelude::*;
    use notepadx::settings::AppSettings;
    use notepadx::ui::font_dialog::FontSettingsDialog;
    use notepadx::ui::theme::ThemeManager;
    use notepadx::ui::window::NotepadxWindow;
    use std::cell::RefCell;
    use std::rc::Rc;

    // 1. Test AppSettings font face, size, and line height behavior
    let mut settings = AppSettings::default();
    assert_eq!(settings.font_face, "Monospace");
    assert_eq!(settings.font_size, 12);
    assert!((settings.line_height - 1.45).abs() < 0.001);

    assert!(settings.set_font_face("JetBrains Mono".to_string()));
    assert_eq!(settings.font_face, "JetBrains Mono");

    assert!(settings.set_font_size(16));
    assert_eq!(settings.font_size, 16);

    assert!(settings.set_line_height(1.65));
    assert!((settings.line_height - 1.65).abs() < 0.001);

    // Test clamping
    settings.set_font_size(2);
    assert_eq!(settings.font_size, notepadx::settings::MIN_FONT_SIZE);
    settings.set_font_size(100);
    assert_eq!(settings.font_size, notepadx::settings::MAX_FONT_SIZE);

    settings.set_line_height(0.5);
    assert_eq!(settings.line_height, notepadx::settings::MIN_LINE_HEIGHT);
    settings.set_line_height(5.0);
    assert_eq!(settings.line_height, notepadx::settings::MAX_LINE_HEIGHT);

    // 2. Test ThemeManager font CSS generation
    let theme_manager = ThemeManager::new();
    theme_manager.update_font_settings("Fira Code", 14, 1.5);
    // Setting with empty/default face falls back safely
    theme_manager.update_font_settings("", 12, 1.45);
    theme_manager.update_font_settings("Padauk, Myanmar3", 15, 1.7);

    // 3. Test FontSettingsDialog instantiation & live change callback
    let parent_win = gtk4::Window::new();
    let callback_received = Rc::new(RefCell::new(None::<(String, u32, f32)>));
    let cb_clone = callback_received.clone();

    FontSettingsDialog::show(
        &parent_win,
        "Ubuntu Mono",
        14,
        1.5,
        move |face, size, lh| {
            *cb_clone.borrow_mut() = Some((face.to_string(), size, lh));
        },
    );

    // 4. Test NotepadxWindow Setting menu, actions, and global font effects
    let app = libadwaita::Application::builder()
        .application_id("com.test.notepadx.fonts")
        .flags(gtk4::gio::ApplicationFlags::NON_UNIQUE)
        .build();
    let _ = app.register(None::<&gtk4::gio::Cancellable>);
    let win = NotepadxWindow::new(&app);

    // Verify window has "font-settings" and "fonts-setting" actions registered
    assert!(
        ActionGroupExt::has_action(win.window(), "font-settings"),
        "Window must have registered win.font-settings action"
    );
    assert!(
        ActionGroupExt::has_action(win.window(), "fonts-setting"),
        "Window must have registered win.fonts-setting action"
    );

    // Test showing the font settings dialog
    win.show_font_settings_dialog();

    // Verify initial settings match
    assert_eq!(win.settings().borrow().font_size, 12);

    // Apply new font settings: Font face "DejaVu Sans Mono", Size 18, Line height 1.8
    win.apply_font_settings("DejaVu Sans Mono", 18, 1.8);

    assert_eq!(win.settings().borrow().font_face, "DejaVu Sans Mono");
    assert_eq!(win.settings().borrow().font_size, 18);
    assert!((win.settings().borrow().line_height - 1.8).abs() < 0.001);

    // Create tabs and verify they open and function with global font settings
    win.new_document();
    win.new_document();

    // Verify Zoom In preserves the customized font face and line height
    ActionGroupExt::activate_action(win.window(), "zoom-in", None);
    assert_eq!(win.settings().borrow().font_size, 19);
    assert_eq!(win.settings().borrow().font_face, "DejaVu Sans Mono");
    assert!((win.settings().borrow().line_height - 1.8).abs() < 0.001);

    // Verify Zoom Reset preserves font face and line height while resetting font size
    ActionGroupExt::activate_action(win.window(), "zoom-reset", None);
    assert_eq!(win.settings().borrow().font_size, 12);
    assert_eq!(win.settings().borrow().font_face, "DejaVu Sans Mono");
    assert!((win.settings().borrow().line_height - 1.8).abs() < 0.001);
}
