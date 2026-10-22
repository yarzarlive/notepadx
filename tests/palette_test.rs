use gtk4::prelude::*;
use libadwaita::Application;
use notepadx::desktop_integration::*;
use notepadx::settings::{
    AppSettings, DEFAULT_PALETTE_HOTKEY, DEFAULT_PALETTE_WIDTH_FRACTION,
    MAX_PALETTE_WIDTH_FRACTION, MIN_PALETTE_WIDTH_FRACTION,
};
use notepadx::ui::palette::{calculate_geometry_for_screen, PaletteWindow};
use std::env;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_palette_screen_geometry_calculations() {
    // 1080p single monitor
    let (x, y, w, h) = calculate_geometry_for_screen(0, 0, 1920, 1080, 0.3333);
    assert_eq!(w, 640);
    assert_eq!(h, 1080);
    assert_eq!(x, 1920 - 640);
    assert_eq!(y, 0);

    // 4K Ultra HD monitor
    let (x, y, w, h) = calculate_geometry_for_screen(0, 0, 3840, 2160, 0.25);
    assert_eq!(w, 960);
    assert_eq!(h, 2160);
    assert_eq!(x, 3840 - 960);
    assert_eq!(y, 0);

    // Secondary monitor with X offset
    let (x, y, w, h) = calculate_geometry_for_screen(1920, 100, 1920, 1080, 0.3333);
    assert_eq!(w, 640);
    assert_eq!(h, 1080);
    assert_eq!(x, 1920 + (1920 - 640));
    assert_eq!(y, 100);

    // Clamping checks
    let (_x, _y, w_low, _h) = calculate_geometry_for_screen(0, 0, 1000, 1000, 0.05);
    assert_eq!(w_low, (1000.0 * MIN_PALETTE_WIDTH_FRACTION) as i32);

    let (_x, _y, w_high, _h) = calculate_geometry_for_screen(0, 0, 1000, 1000, 0.95);
    assert_eq!(w_high, (1000.0 * MAX_PALETTE_WIDTH_FRACTION) as i32);

    let (_x, _y, w_nan, _h) = calculate_geometry_for_screen(0, 0, 1000, 1000, f32::NAN);
    assert_eq!(w_nan, (1000.0 * DEFAULT_PALETTE_WIDTH_FRACTION) as i32);
}

#[test]
fn test_palette_settings_json_serde() {
    let mut settings = AppSettings::default();
    assert!((settings.palette_width_fraction - DEFAULT_PALETTE_WIDTH_FRACTION).abs() < 0.001);
    assert!(!settings.palette_pinned);
    assert!(settings.palette_close_on_escape);
    assert_eq!(settings.palette_hotkey, DEFAULT_PALETTE_HOTKEY);

    settings.palette_width_fraction = 0.40;
    settings.palette_pinned = true;
    settings.palette_close_on_escape = false;
    settings.palette_hotkey = "<Primary><Alt>P".to_string();

    let json = serde_json::to_string_pretty(&settings).unwrap();
    let deserialized: AppSettings = serde_json::from_str(&json).unwrap();

    assert!((deserialized.palette_width_fraction - 0.40).abs() < 0.001);
    assert!(deserialized.palette_pinned);
    assert!(!deserialized.palette_close_on_escape);
    assert_eq!(deserialized.palette_hotkey, "<Primary><Alt>P");
}

#[test]
fn test_desktop_integration_helpers() {
    // 1. Parsing empty
    assert!(parse_custom_keybindings("@as []").is_empty());
    assert!(parse_custom_keybindings("[]").is_empty());

    // 2. Formatting & Dedup
    let (list, changed) = add_keybinding_path_if_missing("@as []", NOTEPADX_KEYBINDING_PATH);
    assert!(changed);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0], NOTEPADX_KEYBINDING_PATH);

    let formatted = format_custom_keybindings(&list);
    assert_eq!(formatted, format!("['{}']", NOTEPADX_KEYBINDING_PATH));

    let (list2, changed2) = add_keybinding_path_if_missing(&formatted, NOTEPADX_KEYBINDING_PATH);
    assert!(!changed2);
    assert_eq!(list2.len(), 1);

    // 3. Service file installation
    let tmp = tempdir().unwrap();
    let installed_file = install_systemd_service(tmp.path()).unwrap();
    assert!(installed_file.exists());
    let contents = fs::read_to_string(&installed_file).unwrap();
    assert!(contents.contains("ExecStart=%h/.local/bin/notepadx --daemon"));
    assert!(contents.contains("Restart=on-failure"));
    assert!(contents.contains("Environment=GDK_BACKEND=wayland,x11"));
}

#[test]
fn test_gtk_palette_window_lifecycle_and_actions() {
    if gtk4::init().is_err() {
        eprintln!("Skipping GTK GUI tests: display not available");
        return;
    }

    let tmp_config = tempdir().unwrap();
    env::set_var("XDG_CONFIG_HOME", tmp_config.path());

    let app = Application::builder()
        .application_id("com.notepadx.TestPaletteLifecycle")
        .flags(gtk4::gio::ApplicationFlags::NON_UNIQUE)
        .build();
    let _ = app.register(None::<&gtk4::gio::Cancellable>);

    let palette = PaletteWindow::new(&app);

    // 1. Initial state: hidden by default
    assert!(!palette.is_visible());
    assert!(!palette.is_pinned());

    // 2. Toggle on -> becomes visible
    palette.toggle();
    assert!(palette.is_visible());

    // 3. Pinning state
    palette.set_pinned(true);
    assert!(palette.is_pinned());
    assert!(palette.settings().borrow().palette_pinned);

    palette.set_pinned(false);
    assert!(!palette.is_pinned());
    assert!(!palette.settings().borrow().palette_pinned);

    // 4. Verify editor components are active inside palette
    let editor = palette.editor_window();
    assert!(editor.is_palette());
    assert!(editor.active_buffer_and_view().is_some());

    if let Some((buffer, view)) = editor.active_buffer_and_view() {
        buffer.set_text("Palette note text persistent");
        assert_eq!(
            buffer.text(&buffer.start_iter(), &buffer.end_iter(), true),
            "Palette note text persistent"
        );
        view.grab_focus();
    }

    // 5. Hide -> invisible
    palette.hide();
    assert!(!palette.is_visible());
}

#[test]
fn test_dock_undock_toggle_menu_and_action() {
    if gtk4::init().is_err() {
        eprintln!("Skipping GTK GUI tests: display not available");
        return;
    }

    let tmp_config = tempdir().unwrap();
    env::set_var("XDG_CONFIG_HOME", tmp_config.path());

    let app = Application::builder()
        .application_id("com.notepadx.TestDockUndock")
        .flags(gtk4::gio::ApplicationFlags::NON_UNIQUE)
        .build();
    let _ = app.register(None::<&gtk4::gio::Cancellable>);

    let win = notepadx::ui::window::NotepadxWindow::new(&app);

    // Initial state: normal window (undocked)
    assert!(!win.is_palette());
    assert!(!win.window().has_css_class("notepadx-palette"));

    // 1. Toggle dock -> docked as overlay palette
    win.toggle_dock();
    assert!(win.is_palette());
    assert!(win.window().has_css_class("notepadx-palette"));
    assert!(win.window().has_css_class("palette-elevation"));
    assert!(win.window().is_visible());

    // 2. Toggle dock -> undocked back to standard window
    win.toggle_dock();
    assert!(!win.is_palette());
    assert!(!win.window().has_css_class("notepadx-palette"));
    assert!(!win.window().has_css_class("palette-elevation"));
    assert!(win.window().is_visible());

    // 3. Trigger via action "win.toggle-dock"
    let action_activated =
        gtk4::prelude::WidgetExt::activate_action(win.window(), "win.toggle-dock", None);
    assert!(action_activated.is_ok());
    assert!(win.is_palette());
    assert!(win.window().has_css_class("notepadx-palette"));
    assert!(win.window().is_visible());

    // Trigger action again -> undocked
    let action_activated2 =
        gtk4::prelude::WidgetExt::activate_action(win.window(), "win.toggle-dock", None);
    assert!(action_activated2.is_ok());
    assert!(!win.is_palette());
    assert!(!win.window().has_css_class("notepadx-palette"));
    assert!(win.window().is_visible());
}
