use glib::clone;
use gtk4::prelude::*;
use libadwaita::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct DialogState {
    font_face: String,
    font_size: u32,
    line_height: f32,
}

pub struct FontSettingsDialog;

impl FontSettingsDialog {
    pub fn show<F>(
        parent: &impl IsA<gtk4::Widget>,
        current_font_face: &str,
        current_font_size: u32,
        current_line_height: f32,
        on_change: F,
    ) where
        F: Fn(&str, u32, f32) + 'static,
    {
        let state = Rc::new(RefCell::new(DialogState {
            font_face: current_font_face.to_string(),
            font_size: current_font_size,
            line_height: current_line_height,
        }));
        let updating_ui = Rc::new(Cell::new(false));
        let on_change = Rc::new(on_change);

        let dialog = libadwaita::PreferencesDialog::new();
        dialog.set_title("Font Settings");
        dialog.add_css_class("font-settings-dialog");
        dialog.set_content_width(500);
        dialog.set_content_height(490);

        let page = libadwaita::PreferencesPage::new();

        // 1. Typography configuration group
        let typography_group = libadwaita::PreferencesGroup::new();
        typography_group.set_title("Typography");
        typography_group
            .set_description(Some("Configure editor font face, size, and line height."));

        // Font Family picker row
        let font_row = libadwaita::ActionRow::builder()
            .title("Font Family")
            .subtitle(current_font_face)
            .build();

        let font_dialog = gtk4::FontDialog::new();
        font_dialog.set_modal(true);
        font_dialog.set_title("Select Font");

        let font_button = gtk4::FontDialogButton::new(Some(font_dialog));
        font_button.set_valign(gtk4::Align::Center);

        let mut initial_desc = gtk4::pango::FontDescription::new();
        initial_desc.set_family(current_font_face);
        initial_desc.set_size((current_font_size as i32) * gtk4::pango::SCALE);
        font_button.set_font_desc(&initial_desc);

        font_row.add_suffix(&font_button);
        font_row.set_activatable_widget(Some(&font_button));
        typography_group.add(&font_row);

        // Custom Font Face Entry row
        let entry_row = libadwaita::EntryRow::builder()
            .title("Font Face Name")
            .text(current_font_face)
            .build();
        typography_group.add(&entry_row);

        // Font Size SpinRow
        let size_adjustment = gtk4::Adjustment::new(
            current_font_size as f64,
            crate::settings::MIN_FONT_SIZE as f64,
            crate::settings::MAX_FONT_SIZE as f64,
            1.0,
            4.0,
            0.0,
        );
        let size_row = libadwaita::SpinRow::builder()
            .title("Font Size")
            .subtitle("Size in points (pt)")
            .adjustment(&size_adjustment)
            .climb_rate(1.0)
            .digits(0)
            .build();
        typography_group.add(&size_row);

        // Line Height SpinRow
        let line_height_adjustment = gtk4::Adjustment::new(
            current_line_height as f64,
            crate::settings::MIN_LINE_HEIGHT as f64,
            crate::settings::MAX_LINE_HEIGHT as f64,
            0.05,
            0.2,
            0.0,
        );
        let line_height_row = libadwaita::SpinRow::builder()
            .title("Line Height")
            .subtitle("Relative line spacing multiplier (e.g. 1.45)")
            .adjustment(&line_height_adjustment)
            .climb_rate(0.05)
            .digits(2)
            .build();
        typography_group.add(&line_height_row);

        // Reset to Defaults row
        let reset_row = libadwaita::ActionRow::builder()
            .title("Reset to Defaults")
            .subtitle("Restore default font (Monospace 12pt, 1.45 line height)")
            .build();
        let reset_btn = gtk4::Button::builder()
            .label("Reset")
            .valign(gtk4::Align::Center)
            .css_classes(["flat"])
            .build();
        reset_row.add_suffix(&reset_btn);
        reset_row.set_activatable_widget(Some(&reset_btn));
        typography_group.add(&reset_row);

        page.add(&typography_group);

        dialog.add(&page);

        // Signal handlers for immediate global updates
        font_button.connect_font_desc_notify(clone!(
            #[weak]
            entry_row,
            #[weak]
            font_row,
            #[weak]
            size_adjustment,
            #[strong]
            updating_ui,
            #[strong]
            state,
            #[strong]
            on_change,
            move |btn| {
                if updating_ui.get() {
                    return;
                }
                updating_ui.set(true);

                let maybe_info = if let Some(desc) = btn.font_desc() {
                    let mut st = state.borrow_mut();
                    if let Some(fam) = desc.family() {
                        let fam_str = fam.to_string();
                        entry_row.set_text(&fam_str);
                        font_row.set_subtitle(&fam_str);
                        st.font_face = fam_str;
                    }
                    let size = desc.size() / gtk4::pango::SCALE;
                    if size > 0 {
                        let clamped = (size as u32).clamp(
                            crate::settings::MIN_FONT_SIZE,
                            crate::settings::MAX_FONT_SIZE,
                        );
                        size_adjustment.set_value(clamped as f64);
                        st.font_size = clamped;
                    }
                    Some((st.font_face.clone(), st.font_size, st.line_height))
                } else {
                    None
                };

                updating_ui.set(false);

                if let Some((face, size, lh)) = maybe_info {
                    on_change(&face, size, lh);
                }
            }
        ));

        entry_row.connect_changed(clone!(
            #[weak]
            font_row,
            #[weak]
            font_button,
            #[strong]
            updating_ui,
            #[strong]
            state,
            #[strong]
            on_change,
            move |row| {
                if updating_ui.get() {
                    return;
                }
                updating_ui.set(true);

                let text = row.text().to_string();
                font_row.set_subtitle(&text);

                let (face, size, lh) = {
                    let mut st = state.borrow_mut();
                    st.font_face = text.clone();
                    (st.font_face.clone(), st.font_size, st.line_height)
                };

                let mut desc = gtk4::pango::FontDescription::new();
                desc.set_family(&face);
                desc.set_size((size as i32) * gtk4::pango::SCALE);
                font_button.set_font_desc(&desc);

                updating_ui.set(false);

                on_change(&face, size, lh);
            }
        ));

        size_adjustment.connect_value_changed(clone!(
            #[weak]
            font_button,
            #[strong]
            updating_ui,
            #[strong]
            state,
            #[strong]
            on_change,
            move |adj| {
                if updating_ui.get() {
                    return;
                }
                updating_ui.set(true);

                let size = (adj.value().round() as u32).clamp(
                    crate::settings::MIN_FONT_SIZE,
                    crate::settings::MAX_FONT_SIZE,
                );

                let (face, size, lh) = {
                    let mut st = state.borrow_mut();
                    st.font_size = size;
                    (st.font_face.clone(), st.font_size, st.line_height)
                };

                let mut desc = gtk4::pango::FontDescription::new();
                desc.set_family(&face);
                desc.set_size((size as i32) * gtk4::pango::SCALE);
                font_button.set_font_desc(&desc);

                updating_ui.set(false);

                on_change(&face, size, lh);
            }
        ));

        line_height_adjustment.connect_value_changed(clone!(
            #[strong]
            updating_ui,
            #[strong]
            state,
            #[strong]
            on_change,
            move |adj| {
                if updating_ui.get() {
                    return;
                }
                let lh = (adj.value() as f32).clamp(
                    crate::settings::MIN_LINE_HEIGHT,
                    crate::settings::MAX_LINE_HEIGHT,
                );

                let (face, size, lh) = {
                    let mut st = state.borrow_mut();
                    st.line_height = lh;
                    (st.font_face.clone(), st.font_size, st.line_height)
                };

                on_change(&face, size, lh);
            }
        ));

        reset_btn.connect_clicked(clone!(
            #[weak]
            entry_row,
            #[weak]
            font_row,
            #[weak]
            size_adjustment,
            #[weak]
            line_height_adjustment,
            #[weak]
            font_button,
            #[strong]
            updating_ui,
            #[strong]
            state,
            #[strong]
            on_change,
            move |_| {
                if updating_ui.get() {
                    return;
                }
                updating_ui.set(true);

                let (face, size, lh) = {
                    let mut st = state.borrow_mut();
                    st.font_face = crate::settings::DEFAULT_FONT_FACE.to_string();
                    st.font_size = crate::settings::DEFAULT_FONT_SIZE;
                    st.line_height = crate::settings::DEFAULT_LINE_HEIGHT;
                    (st.font_face.clone(), st.font_size, st.line_height)
                };

                entry_row.set_text(&face);
                font_row.set_subtitle(&face);
                size_adjustment.set_value(size as f64);
                line_height_adjustment.set_value(lh as f64);

                let mut desc = gtk4::pango::FontDescription::new();
                desc.set_family(&face);
                desc.set_size((size as i32) * gtk4::pango::SCALE);
                font_button.set_font_desc(&desc);

                updating_ui.set(false);

                on_change(&face, size, lh);
            }
        ));

        dialog.present(Some(parent));
    }
}
