use gtk4::gio;
use gtk4::glib;
use libadwaita::prelude::*;
use libadwaita::Application;
use notepadx::desktop_integration;
use notepadx::ui::palette::PaletteWindow;
use notepadx::ui::window::NotepadxWindow;
use std::cell::RefCell;
use std::env;
use std::path::PathBuf;
use std::rc::Rc;

const APP_ID: &str = "com.notepadx.NotepadX";

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.contains(&"--version".to_string()) || args.contains(&"-v".to_string()) {
        println!("NotepadX version {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    if args.contains(&"--setup-hotkey".to_string()) {
        match desktop_integration::setup_desktop_integration() {
            Ok(msg) => {
                println!("{}", msg);
                return;
            }
            Err(e) => {
                eprintln!("Error setting up hotkey and service: {}", e);
                std::process::exit(1);
            }
        }
    }

    if args.contains(&"--remove-hotkey".to_string()) {
        match desktop_integration::remove_gnome_shortcut() {
            Ok(()) => {
                println!("Successfully removed system-wide hotkey and GNOME desktop integration.");
                return;
            }
            Err(e) => {
                eprintln!("Error removing hotkey: {}", e);
                std::process::exit(1);
            }
        }
    }

    if args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        println!("NotepadX - Lightweight native Linux text editor in Rust");
        println!("\nUsage:");
        println!("  notepadx [OPTIONS] [FILE...]");
        println!("\nOptions:");
        println!("  -d, --daemon       Run as background daemon for quick palette overlay");
        println!("      --toggle       Toggle visibility of the palette overlay");
        println!("      --palette      Open directly in palette overlay mode");
        println!("      --setup-hotkey Install systemd daemon and GNOME global shortcut");
        println!("      --remove-hotkey Remove GNOME global shortcut and reset OS keybinding");
        println!("  -h, --help         Print help");
        println!("  -v, --version      Print version");
        return;
    }

    // Prefer X11/Xwayland when DISPLAY is present to support edge-docking and stay-on-top window management
    if env::var("GDK_BACKEND").is_err() && env::var("DISPLAY").is_ok() {
        env::set_var("GDK_BACKEND", "x11,wayland");
    }

    desktop_integration::migrate_if_dangerous_hotkey();

    let app = Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE | gio::ApplicationFlags::HANDLES_OPEN)
        .build();

    app.set_accels_for_action(
        "win.zoom-in",
        &["<Primary>plus", "<Primary>equal", "<Primary>KP_Add"],
    );
    app.set_accels_for_action(
        "win.zoom-out",
        &[
            "<Primary>minus",
            "<Primary>underscore",
            "<Primary>KP_Subtract",
        ],
    );
    app.set_accels_for_action("win.zoom-reset", &["<Primary>0", "<Primary>KP_0"]);
    app.set_accels_for_action("win.new-document", &["<Primary>n"]);
    app.set_accels_for_action("win.open-file", &["<Primary>o"]);
    app.set_accels_for_action("win.save-file", &["<Primary>s"]);
    app.set_accels_for_action("win.save-as", &["<Primary><Shift>s"]);
    app.set_accels_for_action("win.close-tab", &["<Primary>w"]);
    app.set_accels_for_action("win.exit", &["<Primary>q"]);
    app.set_accels_for_action("win.undo", &["<Primary>z"]);
    app.set_accels_for_action("win.redo", &["<Primary>y", "<Primary><Shift>z"]);
    app.set_accels_for_action("win.cut", &["<Primary>x"]);
    app.set_accels_for_action("win.copy", &["<Primary>c"]);
    app.set_accels_for_action("win.paste", &["<Primary>v"]);
    app.set_accels_for_action("win.select-all", &["<Primary>a"]);
    app.set_accels_for_action("win.find", &["<Primary>f"]);
    app.set_accels_for_action("win.replace", &["<Primary>h"]);
    app.set_accels_for_action("win.toggle-dock", &["<Primary><Alt>d", "<Primary><Alt>p"]);

    let palette_window_cell: Rc<RefCell<Option<Rc<PaletteWindow>>>> = Rc::new(RefCell::new(None));
    let get_or_create_palette = {
        let palette_window_cell = palette_window_cell.clone();
        let app = app.clone();
        Rc::new(move || -> Rc<PaletteWindow> {
            let mut pal = palette_window_cell.borrow_mut();
            if let Some(ref p) = *pal {
                p.clone()
            } else {
                let p = PaletteWindow::new(&app);
                *pal = Some(p.clone());
                p
            }
        })
    };

    let toggle_action = gio::SimpleAction::new("toggle-palette", None);
    let get_palette_for_action = get_or_create_palette.clone();
    toggle_action.connect_activate(move |_, _| {
        let palette = get_palette_for_action();
        palette.toggle();
    });
    app.add_action(&toggle_action);

    let daemon_guard: Rc<RefCell<Option<gio::ApplicationHoldGuard>>> = Rc::new(RefCell::new(None));
    let hold_daemon = {
        let app = app.clone();
        let daemon_guard = daemon_guard.clone();
        Rc::new(move || {
            let mut guard = daemon_guard.borrow_mut();
            if guard.is_none() {
                *guard = Some(app.hold());
            }
        })
    };

    let get_palette_cmd = get_or_create_palette.clone();
    let hold_daemon_cmd = hold_daemon.clone();
    app.connect_command_line(move |app, cmdline| {
        let raw_args: Vec<String> = cmdline
            .arguments()
            .into_iter()
            .map(|s| s.to_string_lossy().to_string())
            .collect();

        let has_toggle = raw_args.iter().any(|a| a == "--toggle");
        let has_daemon = raw_args.iter().any(|a| a == "--daemon" || a == "-d");
        let has_palette = raw_args.iter().any(|a| a == "--palette");

        if has_toggle {
            let palette = get_palette_cmd();
            palette.toggle();
            return glib::ExitCode::SUCCESS;
        }

        if has_palette {
            let palette = get_palette_cmd();
            palette.show();
            return glib::ExitCode::SUCCESS;
        }

        if has_daemon {
            hold_daemon_cmd();
            let _ = get_palette_cmd();
            return glib::ExitCode::SUCCESS;
        }

        let files: Vec<PathBuf> = raw_args
            .into_iter()
            .skip(1)
            .filter(|arg| !arg.starts_with('-'))
            .map(PathBuf::from)
            .collect();

        let win = NotepadxWindow::new(app);
        for path in files {
            if path.exists() {
                win.open_file(&path);
            }
        }

        glib::ExitCode::SUCCESS
    });

    app.connect_activate(move |app| {
        let _win = NotepadxWindow::new(app);
    });

    app.connect_open(move |app, files, _| {
        let win = NotepadxWindow::new(app);
        for file in files {
            if let Some(path) = file.path() {
                win.open_file(&path);
            }
        }
    });

    app.run();
}
