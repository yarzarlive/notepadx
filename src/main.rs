use gtk4::gio;
use libadwaita::prelude::*;
use libadwaita::Application;
use notepadx::ui::window::NotepadxWindow;
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

    if args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        println!("NotepadX - Lightweight native Linux text editor in Rust");
        println!("\nUsage:");
        println!("  notepadx [FILE...]");
        println!("\nOptions:");
        println!("  -h, --help     Print help");
        println!("  -v, --version  Print version");
        return;
    }

    let files_to_open: Vec<PathBuf> = args
        .into_iter()
        .skip(1)
        .filter(|arg| !arg.starts_with('-'))
        .map(PathBuf::from)
        .collect();

    let files_cell = Rc::new(files_to_open);

    let app = Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_OPEN)
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

    let files_for_activate = files_cell.clone();
    app.connect_activate(move |app| {
        let win = NotepadxWindow::new(app);
        for path in files_for_activate.iter() {
            if path.exists() {
                win.open_file(path);
            }
        }
    });

    app.connect_open(move |app, files, _| {
        let win = NotepadxWindow::new(app);
        for file in files {
            if let Some(path) = file.path() {
                win.open_file(&path);
            }
        }
    });

    app.run_with_args(&Vec::<String>::new());
}
