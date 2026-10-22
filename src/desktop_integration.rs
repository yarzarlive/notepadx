use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const NOTEPADX_KEYBINDING_PATH: &str =
    "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/notepadx-palette/";
pub const DEFAULT_HOTKEY_BINDING: &str = "<Primary><Alt>P";
pub const KEYBINDING_NAME: &str = "Toggle NotepadX Palette";
pub const KEYBINDING_COMMAND: &str = "notepadx --toggle";

pub const SYSTEMD_SERVICE_CONTENT: &str = r#"[Unit]
Description=NotepadX Background Daemon & Palette Service
PartOf=graphical-session.target
After=graphical-session.target

[Service]
Type=simple
ExecStart=%h/.local/bin/notepadx --daemon
Restart=on-failure
RestartSec=3s
Environment=GDK_BACKEND=wayland,x11

[Install]
WantedBy=graphical-session.target
"#;

/// Parses a GSettings string array representation such as "@as []" or "['/path1/', '/path2/']".
pub fn parse_custom_keybindings(raw: &str) -> Vec<String> {
    let mut results = Vec::new();
    let mut in_quote = false;
    let mut quote_char = '\'';
    let mut current = String::new();

    for ch in raw.chars() {
        if in_quote {
            if ch == quote_char {
                in_quote = false;
                results.push(current.clone());
                current.clear();
            } else {
                current.push(ch);
            }
        } else if ch == '\'' || ch == '"' {
            in_quote = true;
            quote_char = ch;
        }
    }

    results
}

/// Formats a slice of keybinding paths into GSettings array syntax: "['/path1/', '/path2/']".
pub fn format_custom_keybindings(items: &[String]) -> String {
    let quoted: Vec<String> = items.iter().map(|s| format!("'{}'", s)).collect();
    format!("[{}]", quoted.join(", "))
}

/// Appends a new keybinding path to the array if not already present.
/// Returns (updated_array, changed).
pub fn add_keybinding_path_if_missing(existing_raw: &str, path: &str) -> (Vec<String>, bool) {
    let mut items = parse_custom_keybindings(existing_raw);
    if items.iter().any(|item| item == path) {
        (items, false)
    } else {
        items.push(path.to_string());
        (items, true)
    }
}

/// Removes a keybinding path from the array if present.
/// Returns (updated_array, changed).
pub fn remove_keybinding_path(existing_raw: &str, path: &str) -> (Vec<String>, bool) {
    let mut items = parse_custom_keybindings(existing_raw);
    let orig_len = items.len();
    items.retain(|item| item != path);
    let changed = items.len() != orig_len;
    (items, changed)
}

/// Installs the systemd user service file into the specified user systemd directory.
pub fn install_systemd_service(user_systemd_dir: &Path) -> std::io::Result<PathBuf> {
    fs::create_dir_all(user_systemd_dir)?;
    let service_path = user_systemd_dir.join("notepadx.service");
    fs::write(&service_path, SYSTEMD_SERVICE_CONTENT)?;
    Ok(service_path)
}

/// Registers the NotepadX global toggle hotkey in GNOME via gsettings CLI.
pub fn configure_gnome_shortcut(binding: &str) -> Result<(), String> {
    // 1. Check if gsettings is available
    let which_output = Command::new("which")
        .arg("gsettings")
        .output()
        .map_err(|e| format!("Failed to check for gsettings: {}", e))?;
    if !which_output.status.success() {
        return Err("gsettings binary not found on system".to_string());
    }

    // 2. Read existing custom-keybindings list
    let get_output = Command::new("gsettings")
        .args([
            "get",
            "org.gnome.settings-daemon.plugins.media-keys",
            "custom-keybindings",
        ])
        .output()
        .map_err(|e| format!("Failed to read custom-keybindings: {}", e))?;

    let existing_raw = String::from_utf8_lossy(&get_output.stdout);
    let (updated_list, changed) =
        add_keybinding_path_if_missing(&existing_raw, NOTEPADX_KEYBINDING_PATH);

    if changed {
        let formatted = format_custom_keybindings(&updated_list);
        let set_list = Command::new("gsettings")
            .args([
                "set",
                "org.gnome.settings-daemon.plugins.media-keys",
                "custom-keybindings",
                &formatted,
            ])
            .output()
            .map_err(|e| format!("Failed to update custom-keybindings list: {}", e))?;

        if !set_list.status.success() {
            return Err(format!(
                "gsettings set custom-keybindings failed: {}",
                String::from_utf8_lossy(&set_list.stderr)
            ));
        }
    }

    // 3. Set properties for the custom keybinding entry
    let schema_path = format!(
        "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{}",
        NOTEPADX_KEYBINDING_PATH
    );

    let set_name = Command::new("gsettings")
        .args(["set", &schema_path, "name", KEYBINDING_NAME])
        .output()
        .map_err(|e| format!("Failed to set keybinding name: {}", e))?;
    if !set_name.status.success() {
        return Err(format!(
            "Failed to set shortcut name: {}",
            String::from_utf8_lossy(&set_name.stderr)
        ));
    }

    let set_command = Command::new("gsettings")
        .args(["set", &schema_path, "command", KEYBINDING_COMMAND])
        .output()
        .map_err(|e| format!("Failed to set keybinding command: {}", e))?;
    if !set_command.status.success() {
        return Err(format!(
            "Failed to set shortcut command: {}",
            String::from_utf8_lossy(&set_command.stderr)
        ));
    }

    let safe_binding = if crate::settings::is_dangerous_vt_shortcut(binding) {
        DEFAULT_HOTKEY_BINDING
    } else {
        binding
    };

    let set_binding = Command::new("gsettings")
        .args(["set", &schema_path, "binding", safe_binding])
        .output()
        .map_err(|e| format!("Failed to set keybinding combination: {}", e))?;
    if !set_binding.status.success() {
        return Err(format!(
            "Failed to set shortcut binding: {}",
            String::from_utf8_lossy(&set_binding.stderr)
        ));
    }

    Ok(())
}

/// Removes the NotepadX global toggle hotkey from GNOME via gsettings CLI.
pub fn remove_gnome_shortcut() -> Result<(), String> {
    let which_output = Command::new("which")
        .arg("gsettings")
        .output()
        .map_err(|e| format!("Failed to check for gsettings: {}", e))?;
    if !which_output.status.success() {
        return Err("gsettings binary not found on system".to_string());
    }

    let get_output = Command::new("gsettings")
        .args([
            "get",
            "org.gnome.settings-daemon.plugins.media-keys",
            "custom-keybindings",
        ])
        .output()
        .map_err(|e| format!("Failed to read custom-keybindings: {}", e))?;

    let existing_raw = String::from_utf8_lossy(&get_output.stdout);
    let (updated_list, changed) = remove_keybinding_path(&existing_raw, NOTEPADX_KEYBINDING_PATH);

    if changed {
        let formatted = format_custom_keybindings(&updated_list);
        let set_list = Command::new("gsettings")
            .args([
                "set",
                "org.gnome.settings-daemon.plugins.media-keys",
                "custom-keybindings",
                &formatted,
            ])
            .output()
            .map_err(|e| format!("Failed to update custom-keybindings list: {}", e))?;

        if !set_list.status.success() {
            return Err(format!(
                "gsettings set custom-keybindings failed: {}",
                String::from_utf8_lossy(&set_list.stderr)
            ));
        }
    }

    // Reset dconf schema for notepadx-palette
    let _ = Command::new("dconf")
        .args(["reset", "-f", NOTEPADX_KEYBINDING_PATH])
        .output();

    // Disable switch-to-session-9 (Ctrl+Alt+F9 VT switch) in mutter wayland keybindings
    let _ = Command::new("gsettings")
        .args([
            "set",
            "org.gnome.mutter.wayland.keybindings",
            "switch-to-session-9",
            "@as []",
        ])
        .output();

    Ok(())
}

/// Checks if GNOME custom keybinding is currently set to a dangerous VT shortcut, and cleans it up.
pub fn migrate_if_dangerous_hotkey() {
    let which = Command::new("which").arg("gsettings").output();
    if which.map(|o| o.status.success()).unwrap_or(false) {
        let schema_path = format!(
            "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:{}",
            NOTEPADX_KEYBINDING_PATH
        );
        if let Ok(output) = Command::new("gsettings")
            .args(["get", &schema_path, "binding"])
            .output()
        {
            let current_raw = String::from_utf8_lossy(&output.stdout);
            let trimmed = current_raw.trim().trim_matches('\'').trim_matches('"');
            if crate::settings::is_dangerous_vt_shortcut(trimmed) {
                let _ = remove_gnome_shortcut();
            }
        }
    }
}

/// Top-level integration runner: creates systemd service, enables it, and configures GNOME shortcut.
pub fn setup_desktop_integration() -> Result<String, String> {
    let mut log = Vec::new();

    // 1. Install systemd user service unit
    let user_config_dir = dirs::config_dir().unwrap_or_else(|| {
        dirs::home_dir()
            .map(|h| h.join(".config"))
            .unwrap_or_else(|| PathBuf::from("."))
    });
    let systemd_dir = user_config_dir.join("systemd").join("user");
    match install_systemd_service(&systemd_dir) {
        Ok(path) => {
            log.push(format!("Installed systemd unit to {}", path.display()));
            // Reload and enable via systemctl --user
            let reload = Command::new("systemctl")
                .args(["--user", "daemon-reload"])
                .output();
            if let Ok(res) = reload {
                if res.status.success() {
                    log.push("Executed 'systemctl --user daemon-reload'".to_string());
                }
            }
            let enable = Command::new("systemctl")
                .args(["--user", "enable", "--now", "notepadx.service"])
                .output();
            if let Ok(res) = enable {
                if res.status.success() {
                    log.push("Enabled and started notepadx.service".to_string());
                } else {
                    log.push(format!(
                        "Note: systemctl enable returned non-zero (possibly graphical session not active): {}",
                        String::from_utf8_lossy(&res.stderr).trim()
                    ));
                }
            }
        }
        Err(e) => {
            log.push(format!("Warning: Could not install systemd service: {}", e));
        }
    }

    // 2. Configure GNOME shortcut
    let hotkey = crate::settings::AppSettings::load().palette_hotkey;
    match configure_gnome_shortcut(&hotkey) {
        Ok(()) => {
            log.push(format!(
                "Configured GNOME shortcut '{}' for NotepadX Palette overlay",
                hotkey
            ));
        }
        Err(e) => {
            log.push(format!(
                "Warning: Could not configure GNOME shortcut: {}",
                e
            ));
        }
    }

    Ok(log.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_parse_empty_custom_keybindings() {
        assert!(parse_custom_keybindings("@as []").is_empty());
        assert!(parse_custom_keybindings("[]").is_empty());
        assert!(parse_custom_keybindings("").is_empty());
    }

    #[test]
    fn test_parse_and_format_keybindings() {
        let input = "['/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom0/', '/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom1/']";
        let parsed = parse_custom_keybindings(input);
        assert_eq!(parsed.len(), 2);
        assert_eq!(
            parsed[0],
            "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom0/"
        );
        assert_eq!(
            parsed[1],
            "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/custom1/"
        );

        let formatted = format_custom_keybindings(&parsed);
        assert_eq!(formatted, input);
    }

    #[test]
    fn test_add_keybinding_path_dedup() {
        let raw = "@as []";
        let (list1, changed1) = add_keybinding_path_if_missing(raw, NOTEPADX_KEYBINDING_PATH);
        assert!(changed1);
        assert_eq!(list1.len(), 1);
        assert_eq!(list1[0], NOTEPADX_KEYBINDING_PATH);

        let formatted = format_custom_keybindings(&list1);
        let (list2, changed2) =
            add_keybinding_path_if_missing(&formatted, NOTEPADX_KEYBINDING_PATH);
        assert!(!changed2);
        assert_eq!(list2.len(), 1);
    }

    #[test]
    fn test_install_systemd_service() {
        let dir = tempdir().unwrap();
        let service_file = install_systemd_service(dir.path()).unwrap();
        assert!(service_file.exists());
        let content = fs::read_to_string(&service_file).unwrap();
        assert!(content.contains("Description=NotepadX Background Daemon"));
        assert!(content.contains("ExecStart=%h/.local/bin/notepadx --daemon"));
    }

    #[test]
    fn test_remove_keybinding_path() {
        let raw = format!("['{}', '/custom/path/']", NOTEPADX_KEYBINDING_PATH);
        let (updated, changed) = remove_keybinding_path(&raw, NOTEPADX_KEYBINDING_PATH);
        assert!(changed);
        assert_eq!(updated.len(), 1);
        assert_eq!(updated[0], "/custom/path/");

        let (updated2, changed2) =
            remove_keybinding_path("['/custom/path/']", NOTEPADX_KEYBINDING_PATH);
        assert!(!changed2);
        assert_eq!(updated2.len(), 1);
    }
}
