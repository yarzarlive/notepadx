use gtk4::gdk::Display;
use gtk4::CssProvider;
use libadwaita::{ColorScheme, StyleManager};

pub struct ThemeManager {
    css_provider: CssProvider,
}

impl ThemeManager {
    pub fn new() -> Self {
        let css_provider = CssProvider::new();
        css_provider.load_from_string(
            "dialog.font-settings-dialog floating-sheet > dimming,
            dialog.font-settings-dialog bottom-sheet > dimming,
            dialog.preferences floating-sheet > dimming,
            dialog.preferences bottom-sheet > dimming,
            floating-sheet > dimming,
            bottom-sheet > dimming,
            dialog-host > dimming,
            dimming {
                background: none;
                background-color: transparent;
            }",
        );

        if let Some(display) = Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &css_provider,
                gtk4::STYLE_PROVIDER_PRIORITY_USER,
            );
        }

        let style_manager = StyleManager::default();
        // Follow system preference automatically without manual toggle
        style_manager.set_color_scheme(ColorScheme::Default);

        Self { css_provider }
    }

    pub fn ensure_display_provider(&self, display: &Display) {
        gtk4::style_context_add_provider_for_display(
            display,
            &self.css_provider,
            gtk4::STYLE_PROVIDER_PRIORITY_USER,
        );
    }

    pub fn update_font_settings(&self, font_face: &str, font_size_pt: u32, line_height: f32) {
        let clean_face = font_face.trim().trim_matches('\'').trim_matches('"').trim();
        let fallback_scripts = "'JetBrains Mono', 'Fira Code', 'Noto Sans Mono', 'Noto Sans Myanmar', 'Noto Sans Thai', 'Noto Sans Khmer', monospace";

        let font_family_stack = if clean_face.is_empty()
            || clean_face.eq_ignore_ascii_case("monospace")
            || clean_face.eq_ignore_ascii_case("default")
        {
            fallback_scripts.to_string()
        } else if clean_face.contains(',') {
            let user_parts: Vec<String> = clean_face
                .split(',')
                .map(|s| s.trim().trim_matches('\'').trim_matches('"').trim())
                .filter(|s| !s.is_empty())
                .map(|s| format!("'{}'", s))
                .collect();
            format!("{}, {}", user_parts.join(", "), fallback_scripts)
        } else {
            format!("'{}', {}", clean_face, fallback_scripts)
        };

        // In GTK4 CSS, font shorthand on textview sets Pango layout metrics properly,
        // while explicit font-family, font-size, and line-height ensure fine-grained overrides.
        let css = format!(
            "textview, textview text, textview.monospace, .view, .view.monospace {{
                font: {font_size_pt}pt {font_family_stack};
                font-family: {font_family_stack};
                font-size: {font_size_pt}pt;
                line-height: {line_height:.2};
            }}
            .find-replace-bar {{
                background-color: alpha(currentColor, 0.04);
                border-bottom: 1px solid alpha(currentColor, 0.12);
                padding: 6px 12px;
            }}
            /* Transparent overlay background of Setting box so live text changes in tab remain visible */
            dialog.font-settings-dialog floating-sheet > dimming,
            dialog.font-settings-dialog bottom-sheet > dimming,
            dialog.preferences floating-sheet > dimming,
            dialog.preferences bottom-sheet > dimming,
            floating-sheet > dimming,
            bottom-sheet > dimming,
            dialog-host > dimming,
            dimming {{
                background: none;
                background-color: transparent;
            }}",
            font_size_pt = font_size_pt,
            font_family_stack = font_family_stack,
            line_height = line_height,
        );
        self.css_provider.load_from_string(&css);
    }

    pub fn update_font_size(&self, font_size_pt: u32) {
        self.update_font_settings(
            crate::settings::DEFAULT_FONT_FACE,
            font_size_pt,
            crate::settings::DEFAULT_LINE_HEIGHT,
        );
    }
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}
