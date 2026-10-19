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

fn default_true() -> bool {
    true
}

fn default_font_face() -> String {
    DEFAULT_FONT_FACE.to_string()
}

fn default_line_height() -> f32 {
    DEFAULT_LINE_HEIGHT
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
                return AppSettings {
                    font_face,
                    font_size: clamped_font_size,
                    line_height: clamped_line_height,
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
        // Old config JSON without font_face or line_height
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
