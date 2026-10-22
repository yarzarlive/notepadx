use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

pub const DEFAULT_FONT_FACE: &str = "Monospace";
pub const DEFAULT_FONT_SIZE: u32 = 12;
pub const MIN_FONT_SIZE: u32 = 8;
pub const MAX_FONT_SIZE: u32 = 48;
pub const DEFAULT_LINE_HEIGHT: f32 = 1.45;
pub const MIN_LINE_HEIGHT: f32 = 1.0;
pub const MAX_LINE_HEIGHT: f32 = 3.0;
pub const MAX_RECENT_FILES: usize = 20;
pub const DEFAULT_PALETTE_WIDTH_FRACTION: f32 = 0.3333;
pub const MIN_PALETTE_WIDTH_FRACTION: f32 = 0.20;
pub const MAX_PALETTE_WIDTH_FRACTION: f32 = 0.60;
pub const DEFAULT_PALETTE_HOTKEY: &str = "<Primary><Alt>P";

/// Returns true if a keybinding string uses Ctrl+Alt+F1..F12, which triggers Linux kernel VT console switching.
pub fn is_dangerous_vt_shortcut(binding: &str) -> bool {
    let lower = binding.to_ascii_lowercase();
    let has_ctrl = lower.contains("primary") || lower.contains("control") || lower.contains("ctrl");
    let has_alt = lower.contains("alt");
    if has_ctrl && has_alt {
        for n in 1..=12 {
            let fn_str = format!("f{}", n);
            if let Some(idx) = lower.find(&fn_str) {
                let after = &lower[idx + fn_str.len()..];
                if after.starts_with(char::is_numeric) {
                    continue;
                }
                return true;
            }
        }
    }
    false
}

fn default_true() -> bool {
    true
}

fn default_font_face() -> String {
    DEFAULT_FONT_FACE.to_string()
}

fn default_line_height() -> f32 {
    DEFAULT_LINE_HEIGHT
}

fn default_palette_width_fraction() -> f32 {
    DEFAULT_PALETTE_WIDTH_FRACTION
}

fn default_palette_hotkey() -> String {
    DEFAULT_PALETTE_HOTKEY.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default = "default_font_face")]
    pub font_face: String,
    pub font_size: u32,
    #[serde(default = "default_line_height")]
    pub line_height: f32,
    pub window_width: i32,
    pub window_height: i32,
    pub is_maximized: bool,
    #[serde(default)]
    pub recent_files: Vec<PathBuf>,
    #[serde(default = "default_true")]
    pub show_line_numbers: bool,
    #[serde(default = "default_true")]
    pub highlight_current_line: bool,
    #[serde(default = "default_true")]
    pub syntax_highlighting: bool,
    #[serde(default = "default_true")]
    pub word_wrap: bool,
    #[serde(default = "default_palette_width_fraction")]
    pub palette_width_fraction: f32,
    #[serde(default)]
    pub palette_pinned: bool,
    #[serde(default = "default_true")]
    pub palette_close_on_escape: bool,
    #[serde(default = "default_palette_hotkey")]
    pub palette_hotkey: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            font_face: DEFAULT_FONT_FACE.to_string(),
            font_size: DEFAULT_FONT_SIZE,
            line_height: DEFAULT_LINE_HEIGHT,
            window_width: 850,
            window_height: 600,
            is_maximized: false,
            recent_files: Vec::new(),
            show_line_numbers: true,
            highlight_current_line: true,
            syntax_highlighting: true,
            word_wrap: true,
            palette_width_fraction: DEFAULT_PALETTE_WIDTH_FRACTION,
            palette_pinned: false,
            palette_close_on_escape: true,
            palette_hotkey: DEFAULT_PALETTE_HOTKEY.to_string(),
        }
    }
}

impl AppSettings {
    pub fn config_path() -> PathBuf {
        let base_dir = dirs::config_dir().unwrap_or_else(|| {
            dirs::home_dir()
                .map(|h| h.join(".config"))
                .unwrap_or_else(|| PathBuf::from("."))
        });
        base_dir.join("notepadx").join("config.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if let Ok(bytes) = fs::read(&path) {
            if let Ok(settings) = serde_json::from_slice::<AppSettings>(&bytes) {
                let clamped_font_size = settings.font_size.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE);
                let clamped_line_height = if settings.line_height.is_nan() {
                    DEFAULT_LINE_HEIGHT
                } else {
                    settings.line_height.clamp(MIN_LINE_HEIGHT, MAX_LINE_HEIGHT)
                };
                let font_face = if settings.font_face.trim().is_empty() {
                    DEFAULT_FONT_FACE.to_string()
                } else {
                    settings.font_face
                };
                let clamped_palette_width = if settings.palette_width_fraction.is_nan() {
                    DEFAULT_PALETTE_WIDTH_FRACTION
                } else {
                    settings
                        .palette_width_fraction
                        .clamp(MIN_PALETTE_WIDTH_FRACTION, MAX_PALETTE_WIDTH_FRACTION)
                };
                let palette_hotkey = if settings.palette_hotkey.trim().is_empty()
                    || is_dangerous_vt_shortcut(&settings.palette_hotkey)
                {
                    DEFAULT_PALETTE_HOTKEY.to_string()
                } else {
                    settings.palette_hotkey
                };
                return AppSettings {
                    font_face,
                    font_size: clamped_font_size,
                    line_height: clamped_line_height,
                    palette_width_fraction: clamped_palette_width,
                    palette_hotkey,
                    ..settings
                };
            }
        }
        AppSettings::default()
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let temp_path = path.with_extension(format!("tmp-{}", uuid::Uuid::new_v4()));
        let json = serde_json::to_vec_pretty(self)?;
        {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp_path)?;
            file.write_all(&json)?;
            file.flush()?;
            let _ = file.sync_all();
        }

        fs::rename(&temp_path, &path)?;
        if let Some(parent) = path.parent() {
            if let Ok(dir_file) = File::open(parent) {
                let _ = dir_file.sync_all();
            }
        }
        Ok(())
    }

    pub fn increase_font_size(&mut self) -> bool {
        if self.font_size < MAX_FONT_SIZE {
            self.font_size += 1;
            true
        } else {
            false
        }
    }

    pub fn decrease_font_size(&mut self) -> bool {
        if self.font_size > MIN_FONT_SIZE {
            self.font_size -= 1;
            true
        } else {
            false
        }
    }

    pub fn reset_font_size(&mut self) -> bool {
        if self.font_size != DEFAULT_FONT_SIZE {
            self.font_size = DEFAULT_FONT_SIZE;
            true
        } else {
            false
        }
    }

    pub fn set_font_face(&mut self, face: String) -> bool {
        let trimmed = face.trim();
        let new_face = if trimmed.is_empty() {
            DEFAULT_FONT_FACE.to_string()
        } else {
            trimmed.to_string()
        };
        if self.font_face != new_face {
            self.font_face = new_face;
            true
        } else {
            false
        }
    }

    pub fn set_font_size(&mut self, size: u32) -> bool {
        let clamped = size.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE);
        if self.font_size != clamped {
            self.font_size = clamped;
            true
        } else {
            false
        }
    }

    pub fn set_line_height(&mut self, height: f32) -> bool {
        let clamped = if height.is_nan() {
            DEFAULT_LINE_HEIGHT
        } else {
            height.clamp(MIN_LINE_HEIGHT, MAX_LINE_HEIGHT)
        };
        if (self.line_height - clamped).abs() > 0.001 {
            self.line_height = clamped;
            true
        } else {
            false
        }
    }

    pub fn reset_font_settings(&mut self) -> bool {
        let mut changed = false;
        if self.font_face != DEFAULT_FONT_FACE {
            self.font_face = DEFAULT_FONT_FACE.to_string();
            changed = true;
        }
        if self.font_size != DEFAULT_FONT_SIZE {
            self.font_size = DEFAULT_FONT_SIZE;
            changed = true;
        }
        if (self.line_height - DEFAULT_LINE_HEIGHT).abs() > 0.001 {
            self.line_height = DEFAULT_LINE_HEIGHT;
            changed = true;
        }
        changed
    }

    pub fn add_recent_file(&mut self, path: PathBuf) -> bool {
        self.recent_files.retain(|p| p != &path);
        self.recent_files.insert(0, path);
        if self.recent_files.len() > MAX_RECENT_FILES {
            self.recent_files.truncate(MAX_RECENT_FILES);
        }
        true
    }

    pub fn clear_recent_files(&mut self) -> bool {
        if !self.recent_files.is_empty() {
            self.recent_files.clear();
            true
        } else {
            false
        }
    }

    pub fn set_palette_width_fraction(&mut self, fraction: f32) -> bool {
        let clamped = if fraction.is_nan() {
            DEFAULT_PALETTE_WIDTH_FRACTION
        } else {
            fraction.clamp(MIN_PALETTE_WIDTH_FRACTION, MAX_PALETTE_WIDTH_FRACTION)
        };
        if (self.palette_width_fraction - clamped).abs() > 0.001 {
            self.palette_width_fraction = clamped;
            true
        } else {
            false
        }
    }

    pub fn set_palette_pinned(&mut self, pinned: bool) -> bool {
        if self.palette_pinned != pinned {
            self.palette_pinned = pinned;
            true
        } else {
            false
        }
    }

    pub fn set_palette_close_on_escape(&mut self, close: bool) -> bool {
        if self.palette_close_on_escape != close {
            self.palette_close_on_escape = close;
            true
        } else {
            false
        }
    }

    pub fn set_palette_hotkey(&mut self, hotkey: String) -> bool {
        let trimmed = hotkey.trim();
        let new_hotkey = if trimmed.is_empty() || is_dangerous_vt_shortcut(trimmed) {
            DEFAULT_PALETTE_HOTKEY.to_string()
        } else {
            trimmed.to_string()
        };
        if self.palette_hotkey != new_hotkey {
            self.palette_hotkey = new_hotkey;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_size_bounds() {
        let mut settings = AppSettings::default();
        assert_eq!(settings.font_size, DEFAULT_FONT_SIZE);

        for _ in 0..100 {
            settings.increase_font_size();
        }
        assert_eq!(settings.font_size, MAX_FONT_SIZE);

        for _ in 0..100 {
            settings.decrease_font_size();
        }
        assert_eq!(settings.font_size, MIN_FONT_SIZE);

        settings.reset_font_size();
        assert_eq!(settings.font_size, DEFAULT_FONT_SIZE);
    }

    #[test]
    fn test_font_settings_and_reset() {
        let mut settings = AppSettings::default();
        assert_eq!(settings.font_face, DEFAULT_FONT_FACE);
        assert_eq!(settings.font_size, DEFAULT_FONT_SIZE);
        assert!((settings.line_height - DEFAULT_LINE_HEIGHT).abs() < 0.001);

        // Custom font settings
        assert!(settings.set_font_face("JetBrains Mono".to_string()));
        assert_eq!(settings.font_face, "JetBrains Mono");

        assert!(settings.set_font_size(20));
        assert_eq!(settings.font_size, 20);

        assert!(settings.set_line_height(1.8));
        assert!((settings.line_height - 1.8).abs() < 0.001);

        // Clamping line height
        settings.set_line_height(10.0);
        assert_eq!(settings.line_height, MAX_LINE_HEIGHT);

        settings.set_line_height(0.1);
        assert_eq!(settings.line_height, MIN_LINE_HEIGHT);

        // Empty font face falls back to default
        settings.set_font_face("   ".to_string());
        assert_eq!(settings.font_face, DEFAULT_FONT_FACE);

        // Reset
        settings.set_font_face("Fira Code".to_string());
        settings.set_font_size(18);
        settings.set_line_height(2.0);
        assert!(settings.reset_font_settings());
        assert_eq!(settings.font_face, DEFAULT_FONT_FACE);
        assert_eq!(settings.font_size, DEFAULT_FONT_SIZE);
        assert!((settings.line_height - DEFAULT_LINE_HEIGHT).abs() < 0.001);
    }

    #[test]
    fn test_json_deserialization_backwards_compatibility() {
        // Old config JSON without font_face, line_height, or palette settings
        let old_json = r#"{
            "font_size": 16,
            "window_width": 900,
            "window_height": 700,
            "is_maximized": false
        }"#;

        let parsed: AppSettings = serde_json::from_str(old_json).unwrap();
        assert_eq!(parsed.font_size, 16);
        assert_eq!(parsed.font_face, DEFAULT_FONT_FACE);
        assert!((parsed.line_height - DEFAULT_LINE_HEIGHT).abs() < 0.001);
        assert!(parsed.word_wrap);
        assert!((parsed.palette_width_fraction - DEFAULT_PALETTE_WIDTH_FRACTION).abs() < 0.001);
        assert!(!parsed.palette_pinned);
        assert!(parsed.palette_close_on_escape);
        assert_eq!(parsed.palette_hotkey, DEFAULT_PALETTE_HOTKEY);
    }

    #[test]
    fn test_dangerous_vt_shortcut_detection() {
        assert!(is_dangerous_vt_shortcut("<Primary><Alt>F9"));
        assert!(is_dangerous_vt_shortcut("<Primary><Alt>f9"));
        assert!(is_dangerous_vt_shortcut("<Ctrl><Alt>F1"));
        assert!(is_dangerous_vt_shortcut("<Control><Alt>F12"));
        assert!(!is_dangerous_vt_shortcut("<Primary><Alt>P"));
        assert!(!is_dangerous_vt_shortcut("<Primary>F9"));
        assert!(!is_dangerous_vt_shortcut("<Alt>F9"));
        assert!(!is_dangerous_vt_shortcut("<Primary><Shift>P"));
    }

    #[test]
    fn test_palette_settings_defaults_and_clamping() {
        let mut settings = AppSettings::default();
        assert!((settings.palette_width_fraction - 0.3333).abs() < 0.001);
        assert!(!settings.palette_pinned);
        assert!(settings.palette_close_on_escape);
        assert_eq!(settings.palette_hotkey, DEFAULT_PALETTE_HOTKEY);

        // Clamping upper
        assert!(settings.set_palette_width_fraction(0.99));
        assert_eq!(settings.palette_width_fraction, MAX_PALETTE_WIDTH_FRACTION);

        // Clamping lower
        assert!(settings.set_palette_width_fraction(0.05));
        assert_eq!(settings.palette_width_fraction, MIN_PALETTE_WIDTH_FRACTION);

        // Clamping NaN
        assert!(settings.set_palette_width_fraction(f32::NAN));
        assert_eq!(
            settings.palette_width_fraction,
            DEFAULT_PALETTE_WIDTH_FRACTION
        );

        // Pinned
        assert!(settings.set_palette_pinned(true));
        assert!(settings.palette_pinned);

        // Escape
        assert!(settings.set_palette_close_on_escape(false));
        assert!(!settings.palette_close_on_escape);

        // Hotkey
        assert!(settings.set_palette_hotkey("<Primary><Shift>P".to_string()));
        assert_eq!(settings.palette_hotkey, "<Primary><Shift>P");

        // Dangerous VT hotkey fallback to safe default
        assert!(settings.set_palette_hotkey("<Primary><Alt>F9".to_string()));
        assert_eq!(settings.palette_hotkey, DEFAULT_PALETTE_HOTKEY);

        // Change to another hotkey first
        assert!(settings.set_palette_hotkey("<Primary><Alt>N".to_string()));
        assert_eq!(settings.palette_hotkey, "<Primary><Alt>N");

        // Empty hotkey fallback
        assert!(settings.set_palette_hotkey("   ".to_string()));
        assert_eq!(settings.palette_hotkey, DEFAULT_PALETTE_HOTKEY);
    }

    #[test]
    fn test_recent_files_history_limit_and_dedup() {
        let mut settings = AppSettings::default();
        assert!(settings.recent_files.is_empty());

        for i in 0..25 {
            settings.add_recent_file(PathBuf::from(format!("/tmp/file_{}.txt", i)));
        }

        // Limit to max 20
        assert_eq!(settings.recent_files.len(), 20);
        // Most recent at index 0
        assert_eq!(settings.recent_files[0], PathBuf::from("/tmp/file_24.txt"));

        // Re-adding existing file moves it to front without duplicate
        settings.add_recent_file(PathBuf::from("/tmp/file_10.txt"));
        assert_eq!(settings.recent_files.len(), 20);
        assert_eq!(settings.recent_files[0], PathBuf::from("/tmp/file_10.txt"));

        // Clear history
        assert!(settings.clear_recent_files());
        assert!(settings.recent_files.is_empty());
        assert!(!settings.clear_recent_files());
    }
}
