use gtk4::gdk::Display;
use gtk4::CssProvider;
use libadwaita::{ColorScheme, StyleManager};
use std::cell::RefCell;

pub struct ThemeManager {
    css_provider: CssProvider,
    font_face: RefCell<String>,
    font_size: RefCell<u32>,
    line_height: RefCell<f32>,
}

impl ThemeManager {
    pub fn new() -> Self {
        let css_provider = CssProvider::new();
        let manager = Self {
            css_provider,
            font_face: RefCell::new(crate::settings::DEFAULT_FONT_FACE.to_string()),
            font_size: RefCell::new(crate::settings::DEFAULT_FONT_SIZE),
            line_height: RefCell::new(crate::settings::DEFAULT_LINE_HEIGHT),
        };
        manager.refresh();

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

    pub fn refresh(&self) {
        let face = self.font_face.borrow().clone();
        let size = *self.font_size.borrow();
        let lh = *self.line_height.borrow();
        self.apply_css(&face, size, lh);
    }

    pub fn update_font_settings(&self, font_face: &str, font_size_pt: u32, line_height: f32) {
        *self.font_face.borrow_mut() = font_face.to_string();
        *self.font_size.borrow_mut() = font_size_pt;
        *self.line_height.borrow_mut() = line_height;
        self.apply_css(font_face, font_size_pt, line_height);
    }

    fn apply_css(&self, font_face: &str, font_size_pt: u32, line_height: f32) {
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

        let is_dark = StyleManager::default().is_dark();
        let (
            note_bg,
            tabbar_bg,
            tab_fg,
            tab_fg_dim,
            tab_hover_bg,
            tab_btn_hover_bg,
            tab_btn_active_bg,
            tab_sep_color,
        ) = if is_dark {
            (
                "#262626",
                "#1b1b1b",
                "#ffffff",
                "rgba(255, 255, 255, 0.65)",
                "rgba(255, 255, 255, 0.06)",
                "rgba(255, 255, 255, 0.12)",
                "rgba(255, 255, 255, 0.20)",
                "rgba(255, 255, 255, 0.08)",
            )
        } else {
            (
                "#ffffff",
                "#ebebeb",
                "#1f1f1f",
                "rgba(0, 0, 0, 0.65)",
                "rgba(0, 0, 0, 0.05)",
                "rgba(0, 0, 0, 0.08)",
                "rgba(0, 0, 0, 0.15)",
                "rgba(0, 0, 0, 0.10)",
            )
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

            /* Note Panel & Editor Area: unified background matching active tab */
            tabview,
            tabview > scrolledwindow,
            tabview > scrolledwindow > textview,
            tabview > scrolledwindow textview text {{
                background-color: {note_bg};
                border: none;
                box-shadow: none;
            }}

            /* Find & Replace Bar: seamless integration */
            .find-replace-bar {{
                background-color: {note_bg};
                border-bottom: 1px solid {tab_sep_color};
                padding: 6px 12px;
            }}

            /* Google Material Tab Bar Strip */
            tabbar {{
                background-color: {tabbar_bg};
                border: none;
                border-bottom: none;
                box-shadow: none;
                min-height: 40px;
                padding: 0;
                margin: 0;
            }}

            /* Override Adwaita default inset shadows and box padding */
            tabbar .box {{
                background-color: transparent;
                box-shadow: none;
                padding: 0;
                margin: 0;
            }}

            /* TabBox container: tabs flush with the bottom edge */
            tabbar tabbox {{
                padding-top: 5px;
                padding-bottom: 0px;
                margin-bottom: 0px;
                min-height: 35px;
            }}

            tabbar tabbox > tabboxchild {{
                border-radius: 8px 8px 0 0;
                margin-bottom: 0px;
                padding-bottom: 0px;
                outline: none;
            }}

            /* Inactive Individual Tabs: Material surface style */
            tabbar tab {{
                min-width: 60px;
                min-height: 35px;
                margin: 0 1px 0 1px;
                margin-bottom: 0px;
                border-radius: 8px 8px 0 0;
                border: none;
                border-bottom: none;
                padding: 0 8px 0 12px;
                background-color: transparent;
                color: {tab_fg_dim};
                box-shadow: none;
                outline: none;
                transition: background-color 150ms cubic-bezier(0.4, 0, 0.2, 1), color 150ms cubic-bezier(0.4, 0, 0.2, 1);
            }}

            /* Inactive Tab Hover: subtle Material state layer */
            tabbar tab:hover:not(:selected) {{
                background-color: {tab_hover_bg};
                color: {tab_fg};
                border-radius: 8px 8px 0 0;
            }}

            /* Selected Active Tab: Continuous layer/sheet with the note panel */
            tabbar tab:selected {{
                background-color: {note_bg};
                color: {tab_fg};
                font-weight: 500;
                border: none;
                border-bottom: none;
                border-radius: 8px 8px 0 0;
                margin-bottom: 0px;
                box-shadow: none;
                outline: none;
            }}

            /* Tab Title Typography */
            tabbar tab label {{
                font-size: 10.5pt;
                font-weight: 400;
                padding: 0 4px;
            }}

            tabbar tab:selected label {{
                font-weight: 500;
                color: {tab_fg};
            }}

            /* Document Icon */
            tabbar tab image.tab-icon {{
                opacity: 0.70;
                margin-right: 4px;
            }}

            tabbar tab:selected image.tab-icon {{
                opacity: 0.95;
            }}

            /* Material Circular Close Button */
            tabbar tab button {{
                border-radius: 9999px;
                min-width: 20px;
                min-height: 20px;
                padding: 2px;
                margin-left: 6px;
                margin-right: 0px;
                opacity: 0.65;
                background: transparent;
                border: none;
                box-shadow: none;
                color: inherit;
                transition: opacity 150ms ease, background-color 150ms ease;
            }}

            tabbar tab button:hover {{
                opacity: 1.0;
                background-color: {tab_btn_hover_bg};
            }}

            tabbar tab button:active {{
                background-color: {tab_btn_active_bg};
            }}

            /* Tab Separator (Divider between inactive tabs) */
            tabbar separator {{
                background-color: {tab_sep_color};
                margin-top: 10px;
                margin-bottom: 6px;
                min-width: 1px;
            }}

            /* End Action Widget: New Tab (+) button */
            tabbar .end-action,
            tabbar .start-action {{
                padding: 4px 6px 0 6px;
                margin-bottom: 0px;
            }}

            tabbar .end-action button,
            tabbar .start-action button {{
                border-radius: 9999px;
                min-width: 28px;
                min-height: 28px;
                padding: 4px;
                margin: 0 4px;
                background: transparent;
                border: none;
                box-shadow: none;
                opacity: 0.70;
                color: inherit;
                transition: background-color 150ms ease, opacity 150ms ease;
            }}

            tabbar .end-action button:hover,
            tabbar .start-action button:hover {{
                opacity: 1.0;
                background-color: {tab_btn_hover_bg};
            }}

            tabbar .end-action button:active,
            tabbar .start-action button:active {{
                background-color: {tab_btn_active_bg};
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
            }}

            /* Palette Container styling */
            window.notepadx-palette {{
                background-color: @window_bg_color;
                border-left: 1px solid alpha(@borders, 0.4);
                box-shadow: -12px 0 32px rgba(0, 0, 0, 0.35);
            }}

            /* Slimmer palette header bar */
            window.notepadx-palette headerbar {{
                min-height: 38px;
                padding-left: 6px;
                padding-right: 6px;
                border-bottom: 1px solid alpha(@borders, 0.25);
                background: alpha(@headerbar_bg_color, 0.95);
            }}

            /* Pin button active state */
            button.palette-pin.active,
            togglebutton.palette-pin:checked,
            togglebutton.palette-pin.active {{
                color: @accent_color;
                background: alpha(@accent_color, 0.15);
            }}",
            font_size_pt = font_size_pt,
            font_family_stack = font_family_stack,
            line_height = line_height,
            note_bg = note_bg,
            tabbar_bg = tabbar_bg,
            tab_fg = tab_fg,
            tab_fg_dim = tab_fg_dim,
            tab_hover_bg = tab_hover_bg,
            tab_btn_hover_bg = tab_btn_hover_bg,
            tab_btn_active_bg = tab_btn_active_bg,
            tab_sep_color = tab_sep_color,
        );
        self.css_provider.load_from_string(&css);
    }

    pub fn update_font_size(&self, font_size_pt: u32) {
        let face = self.font_face.borrow().clone();
        let lh = *self.line_height.borrow();
        self.update_font_settings(&face, font_size_pt, lh);
    }
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}
