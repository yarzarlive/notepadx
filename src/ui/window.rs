use crate::document::Document;
use crate::encoding::{atomic_save_file, read_text_file};
use crate::error::NotepadxError;
use crate::recovery::{RecoveryManager, SessionState};
use crate::settings::AppSettings;
use crate::ui::find_replace::{BufferViewProvider, FindReplaceBar, ToastFn};
use crate::ui::theme::ThemeManager;
use glib::clone;
use glib::translate::IntoGlib;
use gtk4::prelude::*;
use gtk4::{
    gio, Box as GtkBox, EventControllerKey, EventControllerScroll, EventControllerScrollFlags,
    FileDialog, HeaderBar, MenuButton, Orientation, ScrolledWindow,
};
use libadwaita::prelude::*;
use libadwaita::{
    AboutDialog, AlertDialog, Application, ApplicationWindow, TabBar, TabPage, TabView, Toast,
    ToastOverlay,
};
use sourceview5::prelude::*;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

thread_local! {
    static ACTIVE_WINDOWS: RefCell<Vec<Rc<NotepadxWindow>>> = const { RefCell::new(Vec::new()) };
}

pub struct NotepadxWindow {
    window: ApplicationWindow,
    tab_view: TabView,
    toast_overlay: ToastOverlay,
    documents: Rc<RefCell<Vec<Document>>>,
    recovery_manager: Arc<RecoveryManager>,
    settings: Rc<RefCell<AppSettings>>,
    theme_manager: Rc<ThemeManager>,
    untitled_counter: Rc<RefCell<u32>>,
    save_timeout_scheduled: Rc<RefCell<bool>>,
    find_replace_bar: Rc<FindReplaceBar>,
    recent_menu: gio::Menu,
}

impl NotepadxWindow {
    pub fn new(app: &Application) -> Rc<Self> {
        let recovery_manager = Arc::new(RecoveryManager::default_manager());
        let _ = recovery_manager.ensure_dir();

        let settings = Rc::new(RefCell::new(AppSettings::load()));
        let theme_manager = Rc::new(ThemeManager::new());
        theme_manager.update_font_settings(
            &settings.borrow().font_face,
            settings.borrow().font_size,
            settings.borrow().line_height,
        );

        let window = ApplicationWindow::builder()
            .application(app)
            .title("NotepadX")
            .default_width(settings.borrow().window_width)
            .default_height(settings.borrow().window_height)
            .build();

        theme_manager.ensure_display_provider(&gtk4::prelude::WidgetExt::display(&window));

        if settings.borrow().is_maximized {
            window.maximize();
        }

        let tab_view = TabView::new();
        tab_view.set_vexpand(true);
        tab_view.set_hexpand(true);

        let tab_bar = TabBar::builder().view(&tab_view).autohide(false).build();

        let toast_overlay = ToastOverlay::new();
        toast_overlay.set_vexpand(true);
        toast_overlay.set_hexpand(true);

        // Header bar layout with Text Menus: Files, Edit, View, Help
        let header_bar = HeaderBar::new();

        // Files Menu
        let files_menu = gio::Menu::new();
        files_menu.append(Some("New File"), Some("win.new-document"));
        files_menu.append(Some("Open"), Some("win.open-file"));

        let recent_menu = gio::Menu::new();
        files_menu.append_submenu(Some("Recent Files"), &recent_menu);

        files_menu.append(Some("Save"), Some("win.save-file"));
        files_menu.append(Some("Save As .."), Some("win.save-as"));
        let files_close_section = gio::Menu::new();
        files_close_section.append(Some("Close Tab"), Some("win.close-tab"));
        files_close_section.append(Some("Exit"), Some("win.exit"));
        files_menu.append_section(None, &files_close_section);

        let btn_files = MenuButton::builder()
            .label("Files")
            .menu_model(&files_menu)
            .always_show_arrow(false)
            .css_classes(["flat"])
            .build();

        // Edit Menu
        let edit_menu = gio::Menu::new();
        let undo_redo_section = gio::Menu::new();
        undo_redo_section.append(Some("Undo"), Some("win.undo"));
        undo_redo_section.append(Some("Redo"), Some("win.redo"));
        edit_menu.append_section(None, &undo_redo_section);

        let clipboard_section = gio::Menu::new();
        clipboard_section.append(Some("Cut"), Some("win.cut"));
        clipboard_section.append(Some("Copy"), Some("win.copy"));
        clipboard_section.append(Some("Paste"), Some("win.paste"));
        clipboard_section.append(Some("Select All"), Some("win.select-all"));
        edit_menu.append_section(None, &clipboard_section);

        let search_section = gio::Menu::new();
        search_section.append(Some("Find..."), Some("win.find"));
        search_section.append(Some("Find and Replace..."), Some("win.replace"));
        edit_menu.append_section(None, &search_section);

        let btn_edit = MenuButton::builder()
            .label("Edit")
            .menu_model(&edit_menu)
            .always_show_arrow(false)
            .css_classes(["flat"])
            .build();

        // View Menu
        let view_menu = gio::Menu::new();
        view_menu.append(Some("Zoom In"), Some("win.zoom-in"));
        view_menu.append(Some("Zoom Out"), Some("win.zoom-out"));
        view_menu.append(Some("Reset Zoom"), Some("win.zoom-reset"));

        let btn_view = MenuButton::builder()
            .label("View")
            .menu_model(&view_menu)
            .always_show_arrow(false)
            .css_classes(["flat"])
            .build();

        // Settings Menu
        let setting_menu = gio::Menu::new();
        let font_section = gio::Menu::new();
        font_section.append(Some("Font Settings"), Some("win.font-settings"));
        setting_menu.append_section(None, &font_section);

        let languages_menu = build_languages_menu();
        setting_menu.append_submenu(Some("Languages"), &languages_menu);

        let view_options_section = gio::Menu::new();
        view_options_section.append(Some("Show Line Numbers"), Some("win.toggle-line-numbers"));
        view_options_section.append(
            Some("Highlight Current Line"),
            Some("win.toggle-highlight-line"),
        );
        view_options_section.append(
            Some("Syntax Highlighting"),
            Some("win.toggle-syntax-highlight"),
        );
        setting_menu.append_section(None, &view_options_section);

        let btn_setting = MenuButton::builder()
            .label("Settings")
            .menu_model(&setting_menu)
            .always_show_arrow(false)
            .css_classes(["flat"])
            .build();

        // Help Menu
        let help_menu = gio::Menu::new();
        help_menu.append(Some("About"), Some("win.about"));

        let btn_help = MenuButton::builder()
            .label("Help")
            .menu_model(&help_menu)
            .always_show_arrow(false)
            .css_classes(["flat"])
            .build();

        let menu_bar_box = GtkBox::new(Orientation::Horizontal, 2);
        menu_bar_box.append(&btn_files);
        menu_bar_box.append(&btn_edit);
        menu_bar_box.append(&btn_view);
        menu_bar_box.append(&btn_setting);
        menu_bar_box.append(&btn_help);

        header_bar.pack_start(&menu_bar_box);

        // Document storage & Find/Replace setup
        let documents = Rc::new(RefCell::new(Vec::<Document>::new()));

        let tab_view_clone = tab_view.clone();
        let docs_clone = documents.clone();
        let get_target: BufferViewProvider = Rc::new(move || {
            let page = tab_view_clone.selected_page()?;
            let idx = tab_view_clone.page_position(&page) as usize;
            let docs = docs_clone.borrow();
            let doc = docs.get(idx)?;
            Some((doc.buffer.clone().upcast(), doc.view.clone().upcast()))
        });

        let toast_overlay_clone = toast_overlay.clone();
        let show_toast: ToastFn = Rc::new(move |msg: &str| {
            let toast = Toast::new(msg);
            toast.set_timeout(2);
            toast_overlay_clone.add_toast(toast);
        });

        let find_replace_bar = FindReplaceBar::new(get_target, show_toast);

        let main_box = GtkBox::new(Orientation::Vertical, 0);
        main_box.set_vexpand(true);
        main_box.set_hexpand(true);
        main_box.append(&header_bar);
        main_box.append(&tab_bar);
        main_box.append(find_replace_bar.container());

        toast_overlay.set_child(Some(&tab_view));
        main_box.append(&toast_overlay);

        window.set_content(Some(&main_box));

        let app_window = Rc::new(Self {
            window,
            tab_view,
            toast_overlay,
            documents,
            recovery_manager,
            settings,
            theme_manager,
            untitled_counter: Rc::new(RefCell::new(1)),
            save_timeout_scheduled: Rc::new(RefCell::new(false)),
            find_replace_bar,
            recent_menu,
        });

        // Retain active window on UI thread to ensure signal closures stay active
        ACTIVE_WINDOWS.with(|w| w.borrow_mut().push(app_window.clone()));

        app_window.setup_actions();
        app_window.setup_signals();
        app_window.setup_shortcuts();
        app_window.rebuild_recent_menu();

        // Restore previous session or create a default document
        app_window.restore_session_or_default();

        app_window.window.present();
        app_window
    }

    fn setup_signals(self: &Rc<Self>) {
        // Tab selection change -> update window title and focus active view
        self.tab_view.connect_selected_page_notify(clone!(
            #[weak(rename_to = win)]
            self,
            move |_| {
                win.update_window_title();
                if let Some(doc) = win.active_document() {
                    doc.view.grab_focus();
                }
            }
        ));

        // Close page request with unsaved confirmation
        self.tab_view.connect_close_page(clone!(
            #[weak(rename_to = win)]
            self,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |tab_view, page| {
                let page_idx = tab_view.page_position(page) as usize;
                let is_dirty = {
                    let docs = win.documents.borrow();
                    docs.get(page_idx).map(|d| d.is_modified).unwrap_or(false)
                };

                if is_dirty {
                    let title = {
                        let docs = win.documents.borrow();
                        docs.get(page_idx)
                            .map(|d| d.title.clone())
                            .unwrap_or_else(|| "Document".to_string())
                    };

                    let dialog = AlertDialog::builder()
                        .heading("Unsaved Changes")
                        .body(format!("Save changes to \"{}\" before closing?", title))
                        .build();

                    dialog.add_response("cancel", "Cancel");
                    dialog.add_response("discard", "Discard");
                    dialog.add_response("save", "Save");
                    dialog.set_response_appearance(
                        "discard",
                        libadwaita::ResponseAppearance::Destructive,
                    );
                    dialog
                        .set_response_appearance("save", libadwaita::ResponseAppearance::Suggested);
                    dialog.set_default_response(Some("save"));
                    dialog.set_close_response("cancel");

                    let win_clone = win.clone();
                    let page_clone = page.clone();
                    dialog.choose(
                        Some(&win.window),
                        None::<&gio::Cancellable>,
                        move |response| {
                            match response.as_str() {
                                "save" => {
                                    if win_clone.save_document_at(page_idx, false) {
                                        win_clone.close_page_internal(&page_clone, page_idx, true);
                                    }
                                }
                                "discard" => {
                                    win_clone.close_page_internal(&page_clone, page_idx, true);
                                }
                                _ => {
                                    // Cancel
                                }
                            }
                        },
                    );

                    glib::Propagation::Stop
                } else {
                    win.close_page_internal(page, page_idx, false);
                    glib::Propagation::Proceed
                }
            }
        ));

        // Window close request
        self.window.connect_close_request(clone!(
            #[weak(rename_to = win)]
            self,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_| win.on_window_close()
        ));
        // Track system dark mode and synchronize syntax highlight theme
        let style_manager = libadwaita::StyleManager::default();
        style_manager.connect_dark_notify(clone!(
            #[weak(rename_to = win)]
            self,
            move |sm| {
                let is_dark = sm.is_dark();
                let scheme_id = if is_dark { "Adwaita-dark" } else { "Adwaita" };
                let scheme_mgr = sourceview5::StyleSchemeManager::default();
                if let Some(scheme) = scheme_mgr.scheme(scheme_id) {
                    for doc in win.documents.borrow().iter() {
                        doc.buffer.set_style_scheme(Some(&scheme));
                    }
                }
            }
        ));
    }

    fn close_page_internal(self: &Rc<Self>, page: &TabPage, page_idx: usize, manual_finish: bool) {
        let doc_id = {
            let mut docs = win_docs_mut(&self.documents);
            if page_idx < docs.len() {
                let doc = docs.remove(page_idx);
                Some(doc.id)
            } else {
                None
            }
        };

        if let Some(id) = doc_id {
            self.recovery_manager.cleanup_document(&id);
        }

        if manual_finish {
            self.tab_view.close_page_finish(page, true);
        }

        self.persist_session();
        self.update_window_title();

        // If all tabs closed, open a new empty document
        if self.documents.borrow().is_empty() {
            self.new_document();
        }
    }

    fn setup_shortcuts(self: &Rc<Self>) {
        let key_controller = EventControllerKey::new();
        key_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);

        key_controller.connect_key_pressed(clone!(
            #[weak(rename_to = win)]
            self,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_, keyval, keycode, state| {
                let display = gtk4::prelude::WidgetExt::display(&win.window);
                let effective_key = resolve_shortcut_key(Some(&display), keyval, keycode);
                let is_ctrl = state.contains(gtk4::gdk::ModifierType::CONTROL_MASK);
                let is_shift = state.contains(gtk4::gdk::ModifierType::SHIFT_MASK);

                if effective_key == gtk4::gdk::Key::Escape && win.find_replace_bar.is_visible() {
                    win.find_replace_bar.hide();
                    return glib::Propagation::Stop;
                }

                if is_ctrl {
                    match effective_key {
                        gtk4::gdk::Key::n => {
                            win.new_document();
                            return glib::Propagation::Stop;
                        }
                        gtk4::gdk::Key::o => {
                            win.open_file_dialog();
                            return glib::Propagation::Stop;
                        }
                        gtk4::gdk::Key::s => {
                            win.save_current_file(is_shift);
                            return glib::Propagation::Stop;
                        }
                        gtk4::gdk::Key::w => {
                            win.close_current_tab();
                            return glib::Propagation::Stop;
                        }
                        gtk4::gdk::Key::q => {
                            win.window.close();
                            return glib::Propagation::Stop;
                        }
                        gtk4::gdk::Key::f => {
                            win.show_find_bar();
                            return glib::Propagation::Stop;
                        }
                        gtk4::gdk::Key::h => {
                            win.show_replace_bar();
                            return glib::Propagation::Stop;
                        }
                        gtk4::gdk::Key::z => {
                            if is_shift {
                                win.redo_active();
                            } else {
                                win.undo_active();
                            }
                            return glib::Propagation::Stop;
                        }
                        gtk4::gdk::Key::y => {
                            win.redo_active();
                            return glib::Propagation::Stop;
                        }
                        gtk4::gdk::Key::x => {
                            win.cut_active();
                            return glib::Propagation::Stop;
                        }
                        gtk4::gdk::Key::c => {
                            win.copy_active();
                            return glib::Propagation::Stop;
                        }
                        gtk4::gdk::Key::v => {
                            win.paste_active();
                            return glib::Propagation::Stop;
                        }
                        gtk4::gdk::Key::a => {
                            win.select_all_active();
                            return glib::Propagation::Stop;
                        }
                        gtk4::gdk::Key::plus | gtk4::gdk::Key::equal | gtk4::gdk::Key::KP_Add => {
                            win.zoom_in();
                            return glib::Propagation::Stop;
                        }
                        gtk4::gdk::Key::minus
                        | gtk4::gdk::Key::underscore
                        | gtk4::gdk::Key::KP_Subtract => {
                            win.zoom_out();
                            return glib::Propagation::Stop;
                        }
                        gtk4::gdk::Key::_0 | gtk4::gdk::Key::KP_0 => {
                            win.zoom_reset();
                            return glib::Propagation::Stop;
                        }
                        gtk4::gdk::Key::Tab | gtk4::gdk::Key::ISO_Left_Tab => {
                            if is_shift {
                                win.tab_view.select_previous_page();
                            } else {
                                win.tab_view.select_next_page();
                            }
                            return glib::Propagation::Stop;
                        }
                        gtk4::gdk::Key::Page_Up => {
                            win.tab_view.select_previous_page();
                            return glib::Propagation::Stop;
                        }
                        gtk4::gdk::Key::Page_Down => {
                            win.tab_view.select_next_page();
                            return glib::Propagation::Stop;
                        }
                        _ => {}
                    }
                }
                glib::Propagation::Proceed
            }
        ));

        self.window.add_controller(key_controller);

        let scroll_controller = EventControllerScroll::new(EventControllerScrollFlags::VERTICAL);
        scroll_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);

        scroll_controller.connect_scroll(clone!(
            #[weak(rename_to = win)]
            self,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |controller, _dx, dy| {
                if controller
                    .current_event_state()
                    .contains(gtk4::gdk::ModifierType::CONTROL_MASK)
                {
                    if dy < 0.0 {
                        win.zoom_in();
                        glib::Propagation::Stop
                    } else if dy > 0.0 {
                        win.zoom_out();
                        glib::Propagation::Stop
                    } else {
                        glib::Propagation::Proceed
                    }
                } else {
                    glib::Propagation::Proceed
                }
            }
        ));

        self.window.add_controller(scroll_controller);
    }

    fn setup_actions(self: &Rc<Self>) {
        let action_new = gio::SimpleAction::new("new-document", None);
        action_new.connect_activate(clone!(
            #[weak(rename_to = win)]
            self,
            move |_, _| {
                win.new_document();
            }
        ));
        self.window.add_action(&action_new);

        let action_open = gio::SimpleAction::new("open-file", None);
        action_open.connect_activate(clone!(
            #[weak(rename_to = win)]
            self,
            move |_, _| {
                win.open_file_dialog();
            }
        ));
        self.window.add_action(&action_open);

        let action_save = gio::SimpleAction::new("save-file", None);
        action_save.connect_activate(clone!(
            #[weak(rename_to = win)]
            self,
            move |_, _| {
                win.save_current_file(false);
            }
        ));
        self.window.add_action(&action_save);

        let action_save_as = gio::SimpleAction::new("save-as", None);
        action_save_as.connect_activate(clone!(
            #[weak(rename_to = win)]
            self,
            move |_, _| {
                win.save_current_file(true);
            }
        ));
        self.window.add_action(&action_save_as);

        let action_close_tab = gio::SimpleAction::new("close-tab", None);
        action_close_tab.connect_activate(clone!(
            #[weak(rename_to = win)]
            self,
            move |_, _| {
                win.close_current_tab();
            }
        ));
        self.window.add_action(&action_close_tab);

        let action_zoom_in = gio::SimpleAction::new("zoom-in", None);
        action_zoom_in.connect_activate(clone!(
            #[weak(rename_to = win)]
            self,
            move |_, _| {
                win.zoom_in();
            }
        ));
        self.window.add_action(&action_zoom_in);

        let action_zoom_out = gio::SimpleAction::new("zoom-out", None);
        action_zoom_out.connect_activate(clone!(
            #[weak(rename_to = win)]
            self,
            move |_, _| {
                win.zoom_out();
            }
        ));
        self.window.add_action(&action_zoom_out);

        let action_zoom_reset = gio::SimpleAction::new("zoom-reset", None);
        action_zoom_reset.connect_activate(clone!(
            #[weak(rename_to = win)]
            self,
            move |_, _| {
                win.zoom_reset();
            }
        ));
        self.window.add_action(&action_zoom_reset);

        // Edit actions on active document
        let action_undo = gio::SimpleAction::new("undo", None);
        action_undo.connect_activate(clone!(
            #[weak(rename_to = win)]
            self,
            move |_, _| {
                win.undo_active();
            }
        ));
        self.window.add_action(&action_undo);

        let action_redo = gio::SimpleAction::new("redo", None);
        action_redo.connect_activate(clone!(
            #[weak(rename_to = win)]
            self,
            move |_, _| {
                win.redo_active();
            }
        ));
        self.window.add_action(&action_redo);

        let action_cut = gio::SimpleAction::new("cut", None);
        action_cut.connect_activate(clone!(
            #[weak(rename_to = win)]
            self,
            move |_, _| {
                win.cut_active();
            }
        ));
        self.window.add_action(&action_cut);

        let action_copy = gio::SimpleAction::new("copy", None);
        action_copy.connect_activate(clone!(
            #[weak(rename_to = win)]
            self,
            move |_, _| {
                win.copy_active();
            }
        ));
        self.window.add_action(&action_copy);

        let action_paste = gio::SimpleAction::new("paste", None);
        action_paste.connect_activate(clone!(
            #[weak(rename_to = win)]
            self,
            move |_, _| {
                win.paste_active();
            }
        ));
        self.window.add_action(&action_paste);

        let action_select_all = gio::SimpleAction::new("select-all", None);
        action_select_all.connect_activate(clone!(
            #[weak(rename_to = win)]
            self,
            move |_, _| {
                win.select_all_active();
            }
        ));
        self.window.add_action(&action_select_all);

        let action_exit = gio::SimpleAction::new("exit", None);
        action_exit.connect_activate(clone!(
            #[weak(rename_to = win)]
            self,
            move |_, _| {
                win.window.close();
            }
        ));
        self.window.add_action(&action_exit);

        let action_find = gio::SimpleAction::new("find", None);
        action_find.connect_activate(clone!(
            #[weak(rename_to = win)]
            self,
            move |_, _| {
                win.show_find_bar();
            }
        ));
        self.window.add_action(&action_find);

        let action_replace = gio::SimpleAction::new("replace", None);
        action_replace.connect_activate(clone!(
            #[weak(rename_to = win)]
            self,
            move |_, _| {
                win.show_replace_bar();
            }
        ));
        self.window.add_action(&action_replace);

        let action_about = gio::SimpleAction::new("about", None);
        action_about.connect_activate(clone!(
            #[weak(rename_to = win)]
            self,
            move |_, _| {
                win.show_about_dialog();
            }
        ));
        self.window.add_action(&action_about);

        let action_open_recent =
            gio::SimpleAction::new("open-recent", Some(glib::VariantTy::STRING));
        action_open_recent.connect_activate(clone!(
            #[weak(rename_to = win)]
            self,
            move |_, param| {
                if let Some(param) = param {
                    if let Some(path_str) = param.str() {
                        win.open_file(Path::new(path_str));
                    }
                }
            }
        ));
        self.window.add_action(&action_open_recent);

        let action_clear_recent = gio::SimpleAction::new("clear-recent", None);
        action_clear_recent.connect_activate(clone!(
            #[weak(rename_to = win)]
            self,
            move |_, _| {
                let mut settings = win.settings.borrow_mut();
                if settings.clear_recent_files() {
                    let _ = settings.save();
                    drop(settings);
                    win.rebuild_recent_menu();
                    win.show_toast("Cleared recent files");
                }
            }
        ));
        self.window.add_action(&action_clear_recent);

        let action_set_language =
            gio::SimpleAction::new("set-language", Some(glib::VariantTy::STRING));
        action_set_language.connect_activate(clone!(
            #[weak(rename_to = win)]
            self,
            move |_, param| {
                if let Some(param) = param {
                    if let Some(val) = param.str() {
                        win.set_active_document_language(val);
                    }
                }
            }
        ));
        self.window.add_action(&action_set_language);

        let action_toggle_line_numbers = gio::SimpleAction::new_stateful(
            "toggle-line-numbers",
            None,
            &self.settings.borrow().show_line_numbers.to_variant(),
        );
        action_toggle_line_numbers.connect_activate(clone!(
            #[weak(rename_to = win)]
            self,
            move |action, _| {
                let current = action.state().and_then(|v| v.get::<bool>()).unwrap_or(true);
                let new_state = !current;
                action.set_state(&new_state.to_variant());
                win.settings.borrow_mut().show_line_numbers = new_state;
                let _ = win.settings.borrow().save();
                for doc in win.documents.borrow().iter() {
                    doc.view.set_show_line_numbers(new_state);
                }
            }
        ));
        self.window.add_action(&action_toggle_line_numbers);

        let action_toggle_highlight_line = gio::SimpleAction::new_stateful(
            "toggle-highlight-line",
            None,
            &self.settings.borrow().highlight_current_line.to_variant(),
        );
        action_toggle_highlight_line.connect_activate(clone!(
            #[weak(rename_to = win)]
            self,
            move |action, _| {
                let current = action.state().and_then(|v| v.get::<bool>()).unwrap_or(true);
                let new_state = !current;
                action.set_state(&new_state.to_variant());
                win.settings.borrow_mut().highlight_current_line = new_state;
                let _ = win.settings.borrow().save();
                for doc in win.documents.borrow().iter() {
                    doc.view.set_highlight_current_line(new_state);
                }
            }
        ));
        self.window.add_action(&action_toggle_highlight_line);

        let action_toggle_syntax_highlight = gio::SimpleAction::new_stateful(
            "toggle-syntax-highlight",
            None,
            &self.settings.borrow().syntax_highlighting.to_variant(),
        );
        action_toggle_syntax_highlight.connect_activate(clone!(
            #[weak(rename_to = win)]
            self,
            move |action, _| {
                let current = action.state().and_then(|v| v.get::<bool>()).unwrap_or(true);
                let new_state = !current;
                action.set_state(&new_state.to_variant());
                win.settings.borrow_mut().syntax_highlighting = new_state;
                let _ = win.settings.borrow().save();
                for doc in win.documents.borrow().iter() {
                    doc.buffer.set_highlight_syntax(new_state);
                }
            }
        ));
        self.window.add_action(&action_toggle_syntax_highlight);

        let action_font_settings = gio::SimpleAction::new("font-settings", None);
        action_font_settings.connect_activate(clone!(
            #[weak(rename_to = win)]
            self,
            move |_, _| {
                win.show_font_settings_dialog();
            }
        ));
        self.window.add_action(&action_font_settings);

        let action_fonts_setting = gio::SimpleAction::new("fonts-setting", None);
        action_fonts_setting.connect_activate(clone!(
            #[weak(rename_to = win)]
            self,
            move |_, _| {
                win.show_font_settings_dialog();
            }
        ));
        self.window.add_action(&action_fonts_setting);
    }

    fn record_recent_file(self: &Rc<Self>, path: PathBuf) {
        let mut settings = self.settings.borrow_mut();
        if settings.add_recent_file(path) {
            let _ = settings.save();
            drop(settings);
            self.rebuild_recent_menu();
        }
    }

    pub fn rebuild_recent_menu(self: &Rc<Self>) {
        self.recent_menu.remove_all();
        let settings = self.settings.borrow();
        if settings.recent_files.is_empty() {
            let empty_item = gio::MenuItem::new(Some("No Recent Files"), None);
            self.recent_menu.append_item(&empty_item);
        } else {
            let files_section = gio::Menu::new();
            for path in settings.recent_files.iter() {
                let display_str = if let Some(home) = dirs::home_dir() {
                    if let Ok(rel) = path.strip_prefix(&home) {
                        format!("~/{}", rel.display())
                    } else {
                        path.display().to_string()
                    }
                } else {
                    path.display().to_string()
                };

                let path_str = path.to_string_lossy();
                let item = gio::MenuItem::new(Some(&display_str), None);
                item.set_action_and_target_value(
                    Some("win.open-recent"),
                    Some(&path_str.as_ref().to_variant()),
                );
                files_section.append_item(&item);
            }
            self.recent_menu.append_section(None, &files_section);

            let clear_section = gio::Menu::new();
            clear_section.append(Some("Clear Recent Files"), Some("win.clear-recent"));
            self.recent_menu.append_section(None, &clear_section);
        }
    }

    pub fn set_active_document_language(self: &Rc<Self>, lang_id: &str) {
        if let Some(page) = self.tab_view.selected_page() {
            let idx = self.tab_view.page_position(&page) as usize;
            let mut docs = self.documents.borrow_mut();
            if let Some(doc) = docs.get_mut(idx) {
                if lang_id == "none" {
                    doc.buffer.set_language(None);
                    doc.language_id = None;
                    drop(docs);
                    self.show_toast("Syntax Highlighting: Plain Text");
                } else {
                    let lm = sourceview5::LanguageManager::default();
                    if let Some(lang) = lm.language(lang_id) {
                        doc.buffer.set_language(Some(&lang));
                        doc.language_id = Some(lang_id.to_string());
                        let name = lang.name();
                        drop(docs);
                        self.show_toast(&format!("Language: {}", name));
                    }
                }
            }
        }
    }

    fn focused_widget(&self) -> Option<gtk4::Widget> {
        gtk4::prelude::RootExt::focus(&self.window)
    }

    pub fn undo_active(self: &Rc<Self>) {
        if let Some(focus) = self.focused_widget() {
            if focus.activate_action("text.undo", None).is_ok() {
                return;
            }
        }
        if let Some((buffer, _)) = self.active_buffer_and_view() {
            if buffer.can_undo() {
                buffer.undo();
            }
        }
    }

    pub fn redo_active(self: &Rc<Self>) {
        if let Some(focus) = self.focused_widget() {
            if focus.activate_action("text.redo", None).is_ok() {
                return;
            }
        }
        if let Some((buffer, _)) = self.active_buffer_and_view() {
            if buffer.can_redo() {
                buffer.redo();
            }
        }
    }

    pub fn cut_active(self: &Rc<Self>) {
        if let Some(focus) = self.focused_widget() {
            if focus.activate_action("clipboard.cut", None).is_ok() {
                return;
            }
        }
        if let Some((buffer, view)) = self.active_buffer_and_view() {
            let clipboard = view.clipboard();
            buffer.cut_clipboard(&clipboard, true);
        }
    }

    pub fn copy_active(self: &Rc<Self>) {
        if let Some(focus) = self.focused_widget() {
            if focus.activate_action("clipboard.copy", None).is_ok() {
                return;
            }
        }
        if let Some((buffer, view)) = self.active_buffer_and_view() {
            let clipboard = view.clipboard();
            buffer.copy_clipboard(&clipboard);
        }
    }

    pub fn paste_active(self: &Rc<Self>) {
        if let Some(focus) = self.focused_widget() {
            if focus.activate_action("clipboard.paste", None).is_ok() {
                return;
            }
        }
        if let Some((buffer, view)) = self.active_buffer_and_view() {
            let clipboard = view.clipboard();
            buffer.paste_clipboard(&clipboard, None, true);
        }
    }

    pub fn select_all_active(self: &Rc<Self>) {
        if let Some(focus) = self.focused_widget() {
            if let Ok(editable) = focus.downcast::<gtk4::Editable>() {
                editable.select_region(0, -1);
                return;
            }
        }
        if let Some((buffer, _)) = self.active_buffer_and_view() {
            buffer.select_range(&buffer.start_iter(), &buffer.end_iter());
        }
    }

    pub fn show_find_bar(self: &Rc<Self>) {
        let selected_text = self.active_buffer_and_view().and_then(|(buf, _)| {
            buf.selection_bounds()
                .map(|(s, e)| buf.text(&s, &e, false).to_string())
        });
        self.find_replace_bar.show_find(selected_text);
    }

    pub fn show_replace_bar(self: &Rc<Self>) {
        let selected_text = self.active_buffer_and_view().and_then(|(buf, _)| {
            buf.selection_bounds()
                .map(|(s, e)| buf.text(&s, &e, false).to_string())
        });
        self.find_replace_bar.show_replace(selected_text);
    }

    pub fn show_about_dialog(self: &Rc<Self>) {
        let dialog = AboutDialog::builder()
            .application_name("NotepadX")
            .application_icon("text-editor")
            .version(env!("CARGO_PKG_VERSION"))
            .developer_name("NotepadX Contributors")
            .comments(
                "A lightweight native Linux text editor in Rust with Unicode complex script support.",
            )
            .license_type(gtk4::License::MitX11)
            .copyright("© 2026 NotepadX Contributors")
            .build();
        dialog.present(Some(&self.window));
    }

    pub fn new_document(self: &Rc<Self>) {
        let count = {
            let mut c = self.untitled_counter.borrow_mut();
            let cur = *c;
            *c += 1;
            cur
        };
        let title = format!("Untitled {}", count);
        let doc = Document::new_empty(title);
        self.add_document_tab(doc);
    }

    fn add_document_tab(self: &Rc<Self>, doc: Document) {
        let settings = self.settings.borrow();
        doc.view.set_show_line_numbers(settings.show_line_numbers);
        doc.view
            .set_highlight_current_line(settings.highlight_current_line);
        doc.buffer
            .set_highlight_syntax(settings.syntax_highlighting);
        drop(settings);
        crate::document::setup_style_scheme(&doc.buffer);

        let scroll = ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Automatic)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .vexpand(true)
            .hexpand(true)
            .child(&doc.view)
            .build();

        let page = self.tab_view.append(&scroll);
        let title_display = if doc.is_modified {
            format!("*{}", doc.title)
        } else {
            doc.title.clone()
        };
        page.set_title(&title_display);

        // Hook buffer change for dirty state, modified indicator, and debounced recovery
        doc.buffer.connect_changed(clone!(
            #[weak(rename_to = win)]
            self,
            #[weak]
            page,
            move |_buffer| {
                let page_idx = win.tab_view.page_position(&page) as usize;
                let need_title_update = {
                    let mut docs = win.documents.borrow_mut();
                    if let Some(doc) = docs.get_mut(page_idx) {
                        let is_first_mod = !doc.is_modified;
                        if is_first_mod {
                            doc.is_modified = true;
                            page.set_title(&format!("*{}", doc.title));
                        }
                        doc.journal_seq += 1;
                        is_first_mod
                    } else {
                        false
                    }
                };

                if need_title_update {
                    win.update_window_title();
                }

                win.schedule_recovery_save();
            }
        ));

        self.documents.borrow_mut().push(doc);
        self.tab_view.set_selected_page(&page);
        if let Some(doc) = self.active_document() {
            doc.view.grab_focus();
        }
        self.update_window_title();
        self.persist_session();
    }

    fn schedule_recovery_save(self: &Rc<Self>) {
        let mut scheduled = self.save_timeout_scheduled.borrow_mut();
        if !*scheduled {
            *scheduled = true;
            glib::timeout_add_local_once(
                std::time::Duration::from_millis(300),
                clone!(
                    #[weak(rename_to = win)]
                    self,
                    move || {
                        *win.save_timeout_scheduled.borrow_mut() = false;
                        win.save_all_recovery_snapshots();
                    }
                ),
            );
        }
    }

    fn save_all_recovery_snapshots(self: &Rc<Self>) {
        let docs = self.documents.borrow();
        for doc in docs.iter() {
            if doc.is_modified {
                let snapshot = doc.to_snapshot();
                let _ = self.recovery_manager.write_snapshot(&snapshot);
            }
        }
        self.persist_session();
    }

    fn persist_session(self: &Rc<Self>) {
        let docs = self.documents.borrow();
        let tab_ids: Vec<String> = docs.iter().map(|d| d.id.clone()).collect();
        let active_idx = if let Some(page) = self.tab_view.selected_page() {
            self.tab_view.page_position(&page) as usize
        } else {
            0
        };

        let session = SessionState {
            tab_ids,
            active_tab_index: active_idx,
            updated_at_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        };
        let _ = self.recovery_manager.save_session(&session);
    }

    pub fn open_file<P: AsRef<Path>>(self: &Rc<Self>, path: P) {
        let path_ref = path.as_ref();
        match read_text_file(path_ref) {
            Ok((content, line_ending)) => {
                let doc = Document::from_file(path_ref, &content, line_ending);
                self.add_document_tab(doc);
                self.record_recent_file(path_ref.to_path_buf());
                self.show_toast(&format!("Opened {}", path_ref.display()));
            }
            Err(NotepadxError::BinaryFile(p)) => {
                let dialog = AlertDialog::builder()
                    .heading("Binary File Detected")
                    .body(format!(
                        "\"{}\" appears to be a binary file. NotepadX cannot open binary files.",
                        p.display()
                    ))
                    .build();
                dialog.add_response("ok", "OK");
                dialog.choose(Some(&self.window), None::<&gio::Cancellable>, |_| {});
            }
            Err(NotepadxError::InvalidUtf8(p)) => {
                let dialog = AlertDialog::builder()
                    .heading("Encoding Error")
                    .body(format!(
                        "\"{}\" does not appear to be valid UTF-8. NotepadX refuses to silently corrupt text files.",
                        p.display()
                    ))
                    .build();
                dialog.add_response("ok", "OK");
                dialog.choose(Some(&self.window), None::<&gio::Cancellable>, |_| {});
            }
            Err(e) => {
                let dialog = AlertDialog::builder()
                    .heading("Error Opening File")
                    .body(format!("Failed to open file: {}", e))
                    .build();
                dialog.add_response("ok", "OK");
                dialog.choose(Some(&self.window), None::<&gio::Cancellable>, |_| {});
            }
        }
    }

    fn open_file_dialog(self: &Rc<Self>) {
        let dialog = FileDialog::builder().title("Open Text File").build();

        dialog.open(
            Some(&self.window),
            None::<&gio::Cancellable>,
            clone!(
                #[weak(rename_to = win)]
                self,
                move |res| {
                    if let Ok(file) = res {
                        if let Some(path) = file.path() {
                            win.open_file(&path);
                        }
                    }
                }
            ),
        );
    }

    pub fn save_current_file(self: &Rc<Self>, save_as: bool) {
        if let Some(page) = self.tab_view.selected_page() {
            let page_idx = self.tab_view.page_position(&page) as usize;
            self.save_document_at(page_idx, save_as);
        }
    }

    fn save_document_at(self: &Rc<Self>, page_idx: usize, save_as: bool) -> bool {
        let has_path = {
            let docs = self.documents.borrow();
            docs.get(page_idx).and_then(|d| d.file_path.clone())
        };

        if let Some(path) = has_path {
            if !save_as {
                return self.perform_save(page_idx, path);
            }
        }

        let initial_name = {
            let docs = self.documents.borrow();
            docs.get(page_idx)
                .map(|d| d.title.clone())
                .unwrap_or_else(|| "document.txt".to_string())
        };

        let dialog = FileDialog::builder()
            .title("Save As")
            .initial_name(&initial_name)
            .build();

        dialog.save(
            Some(&self.window),
            None::<&gio::Cancellable>,
            clone!(
                #[weak(rename_to = win)]
                self,
                move |res| {
                    if let Ok(file) = res {
                        if let Some(path) = file.path() {
                            win.perform_save(page_idx, path);
                        }
                    }
                }
            ),
        );
        false
    }

    fn perform_save(self: &Rc<Self>, page_idx: usize, path: PathBuf) -> bool {
        let (content, line_ending, _doc_id) = {
            let docs = self.documents.borrow();
            if let Some(doc) = docs.get(page_idx) {
                (doc.get_text(), doc.line_ending, doc.id.clone())
            } else {
                return false;
            }
        };

        match atomic_save_file(&path, &content, line_ending) {
            Ok(()) => {
                let mut docs = win_docs_mut(&self.documents);
                if let Some(doc) = docs.get_mut(page_idx) {
                    doc.mark_saved(path.clone(), line_ending);
                    if (page_idx as i32) < self.tab_view.n_pages() {
                        let page = self.tab_view.nth_page(page_idx as i32);
                        page.set_title(&doc.title);
                    }
                }
                drop(docs);

                // Reconcile recovery state: snapshot is updated with clean state
                let snapshot = {
                    let docs = self.documents.borrow();
                    docs.get(page_idx).map(|d| d.to_snapshot())
                };
                if let Some(snap) = snapshot {
                    let _ = self.recovery_manager.write_snapshot(&snap);
                }

                self.record_recent_file(path.clone());
                self.update_window_title();
                self.show_toast(&format!("Saved to {}", path.display()));
                true
            }
            Err(e) => {
                let dialog = AlertDialog::builder()
                    .heading("Save Error")
                    .body(format!("Failed to save file: {}", e))
                    .build();
                dialog.add_response("ok", "OK");
                dialog.choose(Some(&self.window), None::<&gio::Cancellable>, |_| {});
                false
            }
        }
    }

    pub fn close_current_tab(self: &Rc<Self>) {
        if let Some(page) = self.tab_view.selected_page() {
            self.tab_view.close_page(&page);
        }
    }

    fn update_window_title(self: &Rc<Self>) {
        if let Some(page) = self.tab_view.selected_page() {
            let page_idx = self.tab_view.page_position(&page) as usize;
            let docs = self.documents.borrow();
            if let Some(doc) = docs.get(page_idx) {
                let title = if doc.is_modified {
                    format!("*{} - NotepadX", doc.title)
                } else {
                    format!("{} - NotepadX", doc.title)
                };
                self.window.set_title(Some(&title));
                return;
            }
        }
        self.window.set_title(Some("NotepadX"));
    }

    fn active_document(&self) -> Option<std::cell::Ref<'_, Document>> {
        let page = self.tab_view.selected_page()?;
        let idx = self.tab_view.page_position(&page) as usize;
        let docs = self.documents.borrow();
        if idx < docs.len() {
            Some(std::cell::Ref::map(docs, |d| &d[idx]))
        } else {
            None
        }
    }

    fn active_buffer_and_view(&self) -> Option<(gtk4::TextBuffer, gtk4::TextView)> {
        let page = self.tab_view.selected_page()?;
        let idx = self.tab_view.page_position(&page) as usize;
        let docs = self.documents.borrow();
        let doc = docs.get(idx)?;
        Some((doc.buffer.clone().upcast(), doc.view.clone().upcast()))
    }

    pub fn settings(&self) -> Rc<RefCell<AppSettings>> {
        self.settings.clone()
    }

    pub fn window(&self) -> &ApplicationWindow {
        &self.window
    }

    pub fn theme_manager(&self) -> Rc<ThemeManager> {
        self.theme_manager.clone()
    }

    pub fn show_font_settings_dialog(self: &Rc<Self>) {
        let (font_face, font_size, line_height) = {
            let settings = self.settings.borrow();
            (
                settings.font_face.clone(),
                settings.font_size,
                settings.line_height,
            )
        };
        crate::ui::font_dialog::FontSettingsDialog::show(
            &self.window,
            &font_face,
            font_size,
            line_height,
            clone!(
                #[weak(rename_to = win)]
                self,
                move |face, size, lh| {
                    win.apply_font_settings(face, size, lh);
                }
            ),
        );
    }

    pub fn show_fonts_setting_dialog(self: &Rc<Self>) {
        self.show_font_settings_dialog();
    }

    pub fn apply_font_settings(self: &Rc<Self>, font_face: &str, font_size: u32, line_height: f32) {
        let mut settings = self.settings.borrow_mut();
        settings.set_font_face(font_face.to_string());
        settings.set_font_size(font_size);
        settings.set_line_height(line_height);
        let _ = settings.save();
        drop(settings);

        self.theme_manager
            .update_font_settings(font_face, font_size, line_height);

        for doc in self.documents.borrow().iter() {
            doc.view.queue_resize();
        }
    }

    fn zoom_in(self: &Rc<Self>) {
        let mut settings = self.settings.borrow_mut();
        if settings.increase_font_size() {
            let face = settings.font_face.clone();
            let size = settings.font_size;
            let lh = settings.line_height;
            let _ = settings.save();
            drop(settings);
            self.theme_manager.update_font_settings(&face, size, lh);
            for doc in self.documents.borrow().iter() {
                doc.view.queue_resize();
            }
        }
    }

    fn zoom_out(self: &Rc<Self>) {
        let mut settings = self.settings.borrow_mut();
        if settings.decrease_font_size() {
            let face = settings.font_face.clone();
            let size = settings.font_size;
            let lh = settings.line_height;
            let _ = settings.save();
            drop(settings);
            self.theme_manager.update_font_settings(&face, size, lh);
            for doc in self.documents.borrow().iter() {
                doc.view.queue_resize();
            }
        }
    }

    fn zoom_reset(self: &Rc<Self>) {
        let mut settings = self.settings.borrow_mut();
        if settings.reset_font_size() {
            let face = settings.font_face.clone();
            let size = settings.font_size;
            let lh = settings.line_height;
            let _ = settings.save();
            drop(settings);
            self.theme_manager.update_font_settings(&face, size, lh);
            for doc in self.documents.borrow().iter() {
                doc.view.queue_resize();
            }
        }
    }

    fn show_toast(self: &Rc<Self>, message: &str) {
        let toast = Toast::new(message);
        toast.set_timeout(2);
        self.toast_overlay.add_toast(toast);
    }

    fn restore_session_or_default(self: &Rc<Self>) {
        let mut restored_count = 0;

        if let Some(session) = self.recovery_manager.load_session() {
            for doc_id in &session.tab_ids {
                if let Ok(snapshot) = self.recovery_manager.recover_document(doc_id) {
                    // Check if file on disk was modified externally
                    let mut snapshot_to_restore = snapshot;
                    if let Some(ref path) = snapshot_to_restore.file_path {
                        if path.exists() {
                            if let Ok(meta) = std::fs::metadata(path) {
                                if let Ok(mtime) = meta.modified() {
                                    if let Ok(dur) = mtime.duration_since(UNIX_EPOCH) {
                                        let disk_mtime = dur.as_secs();
                                        if let Some(saved_mtime) = snapshot_to_restore.file_mtime {
                                            if disk_mtime > saved_mtime {
                                                // External modification detected! Keep unsaved draft intact.
                                                snapshot_to_restore.is_modified = true;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    let doc = Document::from_snapshot(snapshot_to_restore);
                    self.add_document_tab(doc);
                    restored_count += 1;
                }
            }

            if restored_count > 0 {
                let target_idx = session.active_tab_index.min(restored_count - 1);
                if (target_idx as i32) < self.tab_view.n_pages() {
                    let page = self.tab_view.nth_page(target_idx as i32);
                    self.tab_view.set_selected_page(&page);
                }
                self.show_toast("Restored previous session");
                return;
            }
        }

        // If no session found, check any orphan snapshots in recovery
        let orphan_ids = self.recovery_manager.list_recoverable_docs();
        for id in orphan_ids {
            if let Ok(snapshot) = self.recovery_manager.recover_document(&id) {
                let doc = Document::from_snapshot(snapshot);
                self.add_document_tab(doc);
                restored_count += 1;
            }
        }

        if restored_count == 0 {
            self.new_document();
        }
    }

    fn on_window_close(self: &Rc<Self>) -> glib::Propagation {
        // Save window dimensions into settings
        {
            let mut settings = self.settings.borrow_mut();
            let (w, h) = self.window.default_size();
            settings.window_width = w;
            settings.window_height = h;
            settings.is_maximized = self.window.is_maximized();
            let _ = settings.save();
        }

        // Persist session snapshots before exiting
        self.save_all_recovery_snapshots();

        ACTIVE_WINDOWS.with(|w| w.borrow_mut().retain(|item| !Rc::ptr_eq(item, self)));

        glib::Propagation::Proceed
    }
}

fn win_docs_mut(cell: &Rc<RefCell<Vec<Document>>>) -> std::cell::RefMut<'_, Vec<Document>> {
    cell.borrow_mut()
}

/// Helper to determine if a keyval is an ASCII Latin printable or common navigation/keypad key
pub fn is_latin_shortcut_key(key: gtk4::gdk::Key) -> bool {
    let raw = key.into_glib();
    // ASCII printable range 0x20..=0x7E: ' ' to '~' (covers a-z, A-Z, 0-9, -, =, +, _)
    // Keypad functions 0xFF80..=0xFFBD (covers KP_Enter, KP_Add, KP_Subtract, KP_0..KP_9)
    (0x20..=0x7E).contains(&raw)
        || (0xFF80..=0xFFBD).contains(&raw)
        || matches!(
            key,
            gtk4::gdk::Key::Escape
                | gtk4::gdk::Key::Return
                | gtk4::gdk::Key::KP_Enter
                | gtk4::gdk::Key::Tab
                | gtk4::gdk::Key::ISO_Left_Tab
                | gtk4::gdk::Key::BackSpace
                | gtk4::gdk::Key::Page_Up
                | gtk4::gdk::Key::Page_Down
                | gtk4::gdk::Key::Home
                | gtk4::gdk::Key::End
                | gtk4::gdk::Key::Insert
                | gtk4::gdk::Key::Delete
        )
}

/// Resolve the effective logical Key for shortcut evaluation.
/// When non-English input layouts (e.g. Burmese, Russian, Thai, Arabic) or IMEs are active,
/// GDK's keyval contains the translated non-Latin glyph, or VoidSymbol.
/// This function maps the physical hardware keycode back to its primary Latin key symbol
/// so shortcuts (Ctrl+C, Ctrl+V, Ctrl+S, Ctrl+Z, Ctrl+N, etc.) trigger reliably regardless
/// of the user's active typing language.
pub fn resolve_shortcut_key(
    display: Option<&gtk4::gdk::Display>,
    keyval: gtk4::gdk::Key,
    keycode: u32,
) -> gtk4::gdk::Key {
    let lower_keyval = keyval.to_lower();
    if is_latin_shortcut_key(lower_keyval) && lower_keyval != gtk4::gdk::Key::VoidSymbol {
        return lower_keyval;
    }

    // 1. Hardware keycode translation via GDK:
    // Try XKB groups (0 through 3). If any configured group translates this physical keycode
    // to a Latin shortcut key (e.g. secondary English layout on non-English systems,
    // or AZERTY/QWERTZ layouts where letter positions differ from QWERTY), use it.
    if let Some(disp) = display {
        for group in 0..4 {
            if let Some((translated_key, _, _, _)) =
                disp.translate_key(keycode, gtk4::gdk::ModifierType::empty(), group)
            {
                let lower = translated_key.to_lower();
                if is_latin_shortcut_key(lower) && lower != gtk4::gdk::Key::VoidSymbol {
                    return lower;
                }
            }
        }
    }

    // 2. Direct Linux standard evdev/XKB hardware keycodes mapping.
    // Standard Linux evdev keycode + 8 (XKB standard offset).
    // This is the guaranteed fallback when typing in non-Latin layouts (e.g. Burmese, Thai,
    // Russian, Arabic) where no Latin group is active or translated.
    match keycode {
        9 => gtk4::gdk::Key::Escape,
        10 => gtk4::gdk::Key::_1,
        11 => gtk4::gdk::Key::_2,
        12 => gtk4::gdk::Key::_3,
        13 => gtk4::gdk::Key::_4,
        14 => gtk4::gdk::Key::_5,
        15 => gtk4::gdk::Key::_6,
        16 => gtk4::gdk::Key::_7,
        17 => gtk4::gdk::Key::_8,
        18 => gtk4::gdk::Key::_9,
        19 => gtk4::gdk::Key::_0,
        20 => gtk4::gdk::Key::minus,
        21 => gtk4::gdk::Key::equal,
        22 => gtk4::gdk::Key::BackSpace,
        23 => gtk4::gdk::Key::Tab,
        24 => gtk4::gdk::Key::q,
        25 => gtk4::gdk::Key::w,
        26 => gtk4::gdk::Key::e,
        27 => gtk4::gdk::Key::r,
        28 => gtk4::gdk::Key::t,
        29 => gtk4::gdk::Key::y,
        30 => gtk4::gdk::Key::u,
        31 => gtk4::gdk::Key::i,
        32 => gtk4::gdk::Key::o,
        33 => gtk4::gdk::Key::p,
        36 => gtk4::gdk::Key::Return,
        38 => gtk4::gdk::Key::a,
        39 => gtk4::gdk::Key::s,
        40 => gtk4::gdk::Key::d,
        41 => gtk4::gdk::Key::f,
        42 => gtk4::gdk::Key::g,
        43 => gtk4::gdk::Key::h,
        44 => gtk4::gdk::Key::j,
        45 => gtk4::gdk::Key::k,
        46 => gtk4::gdk::Key::l,
        52 => gtk4::gdk::Key::z,
        53 => gtk4::gdk::Key::x,
        54 => gtk4::gdk::Key::c,
        55 => gtk4::gdk::Key::v,
        56 => gtk4::gdk::Key::b,
        57 => gtk4::gdk::Key::n,
        58 => gtk4::gdk::Key::m,
        82 => gtk4::gdk::Key::KP_Subtract,
        86 => gtk4::gdk::Key::KP_Add,
        90 => gtk4::gdk::Key::KP_0,
        104 => gtk4::gdk::Key::KP_Enter,
        110 => gtk4::gdk::Key::Home,
        111 => gtk4::gdk::Key::Up,
        112 => gtk4::gdk::Key::Page_Up,
        113 => gtk4::gdk::Key::Left,
        114 => gtk4::gdk::Key::Right,
        115 => gtk4::gdk::Key::End,
        116 => gtk4::gdk::Key::Down,
        117 => gtk4::gdk::Key::Page_Down,
        118 => gtk4::gdk::Key::Insert,
        119 => gtk4::gdk::Key::Delete,
        _ => lower_keyval,
    }
}

pub fn build_languages_menu() -> gio::Menu {
    let menu = gio::Menu::new();

    // Plain Text option to disable syntax highlighting
    let none_item = gio::MenuItem::new(Some("Plain Text"), Some("win.set-language"));
    none_item
        .set_action_and_target_value(Some("win.set-language"), Some(&glib::Variant::from("none")));
    menu.append_item(&none_item);

    let lm = sourceview5::LanguageManager::default();
    let lang_ids = lm.language_ids();

    let mut langs: Vec<(String, String)> = lang_ids
        .iter()
        .filter_map(|id| {
            let lang = lm.language(id)?;
            Some((lang.name().to_string(), id.to_string()))
        })
        .collect();

    // Sort alphabetically by language display name
    langs.sort_by_key(|a| a.0.to_lowercase());

    // Group by initial letter (A-Z, #)
    use std::collections::BTreeMap;
    let mut groups: BTreeMap<char, Vec<(String, String)>> = BTreeMap::new();
    for (name, id) in langs {
        let first_char = name
            .chars()
            .next()
            .map(|c| c.to_ascii_uppercase())
            .filter(|c| c.is_ascii_alphabetic())
            .unwrap_or('#');
        groups.entry(first_char).or_default().push((name, id));
    }

    // Add letter submenus (e.g. A, B, C, ... P, ... Z)
    for (letter, group_langs) in groups {
        let letter_menu = gio::Menu::new();
        for (name, id) in group_langs {
            let item = gio::MenuItem::new(Some(&name), Some("win.set-language"));
            item.set_action_and_target_value(
                Some("win.set-language"),
                Some(&glib::Variant::from(id.as_str())),
            );
            letter_menu.append_item(&item);
        }
        menu.append_submenu(Some(&letter.to_string()), &letter_menu);
    }

    menu
}

#[cfg(test)]
mod tests {
    use super::*;
    use glib::translate::FromGlib;

    #[test]
    fn test_resolve_shortcut_key_fallback() {
        let dummy = gtk4::gdk::Key::VoidSymbol;
        let cyrillic_es = gtk4::gdk::Key::Cyrillic_es;
        let arabic_sheen = gtk4::gdk::Key::Arabic_sheen;
        let burmese_ka = unsafe { gtk4::gdk::Key::from_glib(0x01001000) };

        // 1. Physical key mapping fallbacks for all shortcuts
        assert_eq!(resolve_shortcut_key(None, dummy, 39), gtk4::gdk::Key::s);
        assert_eq!(resolve_shortcut_key(None, dummy, 57), gtk4::gdk::Key::n);
        assert_eq!(resolve_shortcut_key(None, dummy, 32), gtk4::gdk::Key::o);
        assert_eq!(resolve_shortcut_key(None, dummy, 54), gtk4::gdk::Key::c);
        assert_eq!(resolve_shortcut_key(None, dummy, 55), gtk4::gdk::Key::v);
        assert_eq!(resolve_shortcut_key(None, dummy, 53), gtk4::gdk::Key::x);
        assert_eq!(resolve_shortcut_key(None, dummy, 38), gtk4::gdk::Key::a);
        assert_eq!(resolve_shortcut_key(None, dummy, 52), gtk4::gdk::Key::z);
        assert_eq!(resolve_shortcut_key(None, dummy, 41), gtk4::gdk::Key::f);
        assert_eq!(resolve_shortcut_key(None, dummy, 43), gtk4::gdk::Key::h);
        assert_eq!(resolve_shortcut_key(None, dummy, 25), gtk4::gdk::Key::w);
        assert_eq!(resolve_shortcut_key(None, dummy, 24), gtk4::gdk::Key::q);
        assert_eq!(resolve_shortcut_key(None, dummy, 29), gtk4::gdk::Key::y);
        assert_eq!(resolve_shortcut_key(None, dummy, 21), gtk4::gdk::Key::equal);
        assert_eq!(resolve_shortcut_key(None, dummy, 20), gtk4::gdk::Key::minus);
        assert_eq!(resolve_shortcut_key(None, dummy, 19), gtk4::gdk::Key::_0);
        assert_eq!(resolve_shortcut_key(None, dummy, 9), gtk4::gdk::Key::Escape);
        assert_eq!(resolve_shortcut_key(None, dummy, 23), gtk4::gdk::Key::Tab);
        assert_eq!(
            resolve_shortcut_key(None, dummy, 36),
            gtk4::gdk::Key::Return
        );
        assert_eq!(
            resolve_shortcut_key(None, dummy, 82),
            gtk4::gdk::Key::KP_Subtract
        );
        assert_eq!(
            resolve_shortcut_key(None, dummy, 86),
            gtk4::gdk::Key::KP_Add
        );
        assert_eq!(resolve_shortcut_key(None, dummy, 90), gtk4::gdk::Key::KP_0);
        assert_eq!(
            resolve_shortcut_key(None, dummy, 104),
            gtk4::gdk::Key::KP_Enter
        );
        assert_eq!(
            resolve_shortcut_key(None, dummy, 112),
            gtk4::gdk::Key::Page_Up
        );
        assert_eq!(
            resolve_shortcut_key(None, dummy, 117),
            gtk4::gdk::Key::Page_Down
        );

        // 2. Real non-Latin keyvals map to their physical key's Latin shortcut
        assert_eq!(
            resolve_shortcut_key(None, cyrillic_es, 54),
            gtk4::gdk::Key::c
        );
        assert_eq!(
            resolve_shortcut_key(None, burmese_ka, 54),
            gtk4::gdk::Key::c
        );
        assert_eq!(
            resolve_shortcut_key(None, burmese_ka, 55),
            gtk4::gdk::Key::v
        );
        assert_eq!(
            resolve_shortcut_key(None, arabic_sheen, 38),
            gtk4::gdk::Key::a
        );

        // 3. Latin keys remain unchanged
        assert_eq!(
            resolve_shortcut_key(None, gtk4::gdk::Key::c, 54),
            gtk4::gdk::Key::c
        );
        assert_eq!(
            resolve_shortcut_key(None, gtk4::gdk::Key::C, 54),
            gtk4::gdk::Key::c
        );
        assert_eq!(
            resolve_shortcut_key(None, gtk4::gdk::Key::s, 39),
            gtk4::gdk::Key::s
        );
    }
}
