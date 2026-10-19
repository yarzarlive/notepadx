use gtk4::gdk::Display;
use gtk4::CssProvider;
use libadwaita::{ColorScheme, StyleManager};

pub struct ThemeManager {
    css_provider: CssProvider,
}

impl ThemeManager {
    pub fn new() -> Self {
        let css_provider = CssProvider::new();
        let manager = Self { css_provider };
        manager.update_font_settings(
            crate::settings::DEFAULT_FONT_FACE,
            crate::settings::DEFAULT_FONT_SIZE,
            crate::settings::DEFAULT_LINE_HEIGHT,
        );

        if let Some(display) = Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &manager.css_provider,
                gtk4::STYLE_PROVIDER_PRIORITY_USER,
            );
        }

        let style_manager = StyleManager::default();
        // Follow system preference automatically without manual toggle
        style_manager.set_color_scheme(ColorScheme::Default);

        manager
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
            /* Browser-style Tab Bar Strip: clean background and subtle bottom edge */
            tabbar {{
                background-color: alpha(@window_fg_color, 0.04);
                border-bottom: 1px solid alpha(currentColor, 0.12);
                min-height: 38px;
                padding: 0 4px;
            }}
            /* Individual Browser Tab Heads: 220px max enforced natively via expand_tabs(false), left-aligned, browser shape */
            tabbar tab {{
                min-width: 48px;
                margin: 4px 2px 0 2px;
                border-radius: 8px 8px 0 0;
                padding: 2px 8px;
                background-color: transparent;
                color: alpha(@window_fg_color, 0.72);
                border: 1px solid transparent;
                border-bottom: none;
                transition: background-color 150ms ease, color 150ms ease;
            }}
            /* Hover on inactive tab */
            tabbar tab:hover:not(:selected) {{
                background-color: alpha(currentColor, 0.08);
                color: @window_fg_color;
            }}
            /* Selected Active Browser Tab: Elevated card look connecting to the editor */
            tabbar tab:selected {{
                background-color: @view_bg_color;
                color: @view_fg_color;
                font-weight: 600;
                border-top: 2px solid @accent_bg_color;
                border-left: 1px solid alpha(currentColor, 0.12);
                border-right: 1px solid alpha(currentColor, 0.12);
                border-bottom: 1px solid @view_bg_color;
                box-shadow: 0 -1px 3px alpha(currentColor, 0.06);
            }}
            /* Tab Title Label */
            tabbar tab label {{
                font-size: 11pt;
                padding: 0 4px;
            }}
            /* Browser-style Tab Close Button */
            tabbar tab button {{
                border-radius: 9999px;
                min-width: 18px;
                min-height: 18px;
                padding: 2px;
                margin-left: 4px;
                opacity: 0.65;
                background: transparent;
                border: none;
                box-shadow: none;
            }}
            tabbar tab button:hover {{
                opacity: 1.0;
                background-color: alpha(currentColor, 0.16);
            }}
            /* Tab Separator (divider between tabs like in Chrome/Firefox) */
            tabbar separator {{
                background-color: alpha(currentColor, 0.12);
                margin-top: 10px;
                margin-bottom: 10px;
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
