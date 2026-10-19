use glib::clone;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, Entry, EventControllerKey, Label, Orientation, SearchEntry,
    TextBuffer, TextSearchFlags, TextView,
};
use std::cell::RefCell;
use std::rc::Rc;

pub type BufferViewProvider = Rc<dyn Fn() -> Option<(TextBuffer, TextView)>>;
pub type ToastFn = Rc<dyn Fn(&str)>;

pub struct FindReplaceBar {
    container: GtkBox,
    search_entry: SearchEntry,
    replace_row: GtkBox,
    replace_entry: Entry,
    status_label: Label,
    match_case_check: CheckButton,
    get_target: BufferViewProvider,
    show_toast: ToastFn,
    replace_visible: RefCell<bool>,
}

impl FindReplaceBar {
    pub fn new(get_target: BufferViewProvider, show_toast: ToastFn) -> Rc<Self> {
        let container = GtkBox::new(Orientation::Vertical, 4);
        container.add_css_class("find-replace-bar");
        container.set_visible(false);

        // --- Row 1: Find Controls ---
        let find_row = GtkBox::new(Orientation::Horizontal, 6);

        let search_entry = SearchEntry::builder()
            .placeholder_text("Find...")
            .hexpand(true)
            .build();

        let status_label = Label::builder()
            .label("")
            .css_classes(["dim-label"])
            .xalign(0.0)
            .build();
        status_label.set_width_chars(12);

        let nav_box = GtkBox::new(Orientation::Horizontal, 0);
        nav_box.add_css_class("linked");

        let btn_prev = Button::builder()
            .icon_name("go-up-symbolic")
            .tooltip_text("Previous Match (Shift+Enter)")
            .build();

        let btn_next = Button::builder()
            .icon_name("go-down-symbolic")
            .tooltip_text("Next Match (Enter)")
            .build();

        nav_box.append(&btn_prev);
        nav_box.append(&btn_next);

        let match_case_check = CheckButton::builder()
            .label("Match Case")
            .active(false)
            .build();

        let btn_toggle_replace = Button::builder()
            .icon_name("edit-find-replace-symbolic")
            .tooltip_text("Toggle Replace")
            .build();

        let btn_close = Button::builder()
            .icon_name("window-close-symbolic")
            .tooltip_text("Close (Esc)")
            .css_classes(["flat"])
            .build();

        find_row.append(&search_entry);
        find_row.append(&status_label);
        find_row.append(&nav_box);
        find_row.append(&match_case_check);
        find_row.append(&btn_toggle_replace);
        find_row.append(&btn_close);

        // --- Row 2: Replace Controls ---
        let replace_row = GtkBox::new(Orientation::Horizontal, 6);
        replace_row.set_visible(false);

        let replace_entry = Entry::builder()
            .placeholder_text("Replace with...")
            .hexpand(true)
            .build();

        let rep_btn_box = GtkBox::new(Orientation::Horizontal, 0);
        rep_btn_box.add_css_class("linked");

        let btn_replace = Button::builder()
            .label("Replace")
            .tooltip_text("Replace current match")
            .build();

        let btn_replace_all = Button::builder()
            .label("Replace All")
            .tooltip_text("Replace all occurrences")
            .build();

        rep_btn_box.append(&btn_replace);
        rep_btn_box.append(&btn_replace_all);

        replace_row.append(&replace_entry);
        replace_row.append(&rep_btn_box);

        container.append(&find_row);
        container.append(&replace_row);

        let bar = Rc::new(Self {
            container,
            search_entry,
            replace_row,
            replace_entry,
            status_label,
            match_case_check,
            get_target,
            show_toast,
            replace_visible: RefCell::new(false),
        });

        // Event hooks
        btn_next.connect_clicked(clone!(
            #[weak]
            bar,
            move |_| {
                bar.find_next(true);
            }
        ));

        btn_prev.connect_clicked(clone!(
            #[weak]
            bar,
            move |_| {
                bar.find_prev();
            }
        ));

        btn_replace.connect_clicked(clone!(
            #[weak]
            bar,
            move |_| {
                bar.replace_current();
            }
        ));

        btn_replace_all.connect_clicked(clone!(
            #[weak]
            bar,
            move |_| {
                bar.replace_all();
            }
        ));

        btn_toggle_replace.connect_clicked(clone!(
            #[weak]
            bar,
            move |_| {
                bar.toggle_replace();
            }
        ));

        btn_close.connect_clicked(clone!(
            #[weak]
            bar,
            move |_| {
                bar.hide();
            }
        ));

        // When search text changes, update count & highlight without necessarily advancing cursor
        bar.search_entry.connect_search_changed(clone!(
            #[weak]
            bar,
            move |_| {
                bar.find_next(false);
            }
        ));

        // SearchEntry signals
        bar.search_entry.connect_activate(clone!(
            #[weak]
            bar,
            move |_| {
                bar.find_next(true);
            }
        ));

        bar.search_entry.connect_next_match(clone!(
            #[weak]
            bar,
            move |_| {
                bar.find_next(true);
            }
        ));

        bar.search_entry.connect_previous_match(clone!(
            #[weak]
            bar,
            move |_| {
                bar.find_prev();
            }
        ));

        bar.search_entry.connect_stop_search(clone!(
            #[weak]
            bar,
            move |_| {
                bar.hide();
            }
        ));

        // When Match Case is toggled, re-run search
        bar.match_case_check.connect_toggled(clone!(
            #[weak]
            bar,
            move |_| {
                bar.find_next(false);
            }
        ));

        // Replace entry activate -> Replace current match
        bar.replace_entry.connect_activate(clone!(
            #[weak]
            bar,
            move |_| {
                bar.replace_current();
            }
        ));

        let search_key_controller = EventControllerKey::new();
        search_key_controller.connect_key_pressed(clone!(
            #[weak]
            bar,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_, keyval, keycode, state| {
                if keyval == gtk4::gdk::Key::Escape || keycode == 9 {
                    bar.hide();
                    return glib::Propagation::Stop;
                }
                if keyval == gtk4::gdk::Key::Return
                    || keyval == gtk4::gdk::Key::KP_Enter
                    || keycode == 36
                    || keycode == 104
                {
                    let is_shift = state.contains(gtk4::gdk::ModifierType::SHIFT_MASK);
                    if is_shift {
                        bar.find_prev();
                    } else {
                        bar.find_next(true);
                    }
                    return glib::Propagation::Stop;
                }
                glib::Propagation::Proceed
            }
        ));
        bar.search_entry.add_controller(search_key_controller);

        let replace_key_controller = EventControllerKey::new();
        replace_key_controller.connect_key_pressed(clone!(
            #[weak]
            bar,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_, keyval, keycode, state| {
                if keyval == gtk4::gdk::Key::Escape || keycode == 9 {
                    bar.hide();
                    return glib::Propagation::Stop;
                }
                if keyval == gtk4::gdk::Key::Return
                    || keyval == gtk4::gdk::Key::KP_Enter
                    || keycode == 36
                    || keycode == 104
                {
                    let is_shift = state.contains(gtk4::gdk::ModifierType::SHIFT_MASK);
                    if is_shift {
                        bar.find_prev();
                    } else {
                        bar.find_next(true);
                    }
                    return glib::Propagation::Stop;
                }
                glib::Propagation::Proceed
            }
        ));
        bar.replace_entry.add_controller(replace_key_controller);

        bar
    }

    pub fn container(&self) -> &GtkBox {
        &self.container
    }

    pub fn is_visible(&self) -> bool {
        self.container.is_visible()
    }

    pub fn show_find(&self, selected_text: Option<String>) {
        self.container.set_visible(true);
        if let Some(text) = selected_text {
            let trimmed = text.trim();
            if !trimmed.is_empty() && !trimmed.contains('\n') {
                self.search_entry.set_text(trimmed);
            }
        }
        self.search_entry.grab_focus();
        self.search_entry.select_region(0, -1);
        self.find_next(false);
    }

    pub fn show_replace(&self, selected_text: Option<String>) {
        self.container.set_visible(true);
        *self.replace_visible.borrow_mut() = true;
        self.replace_row.set_visible(true);

        if let Some(text) = selected_text {
            let trimmed = text.trim();
            if !trimmed.is_empty() && !trimmed.contains('\n') {
                self.search_entry.set_text(trimmed);
            }
        }

        if self.search_entry.text().is_empty() {
            self.search_entry.grab_focus();
        } else {
            self.replace_entry.grab_focus();
            self.replace_entry.select_region(0, -1);
        }
        self.find_next(false);
    }

    pub fn toggle_replace(&self) {
        let mut vis = self.replace_visible.borrow_mut();
        *vis = !*vis;
        self.replace_row.set_visible(*vis);
        if *vis {
            self.replace_entry.grab_focus();
        } else {
            self.search_entry.grab_focus();
        }
    }

    pub fn hide(&self) {
        self.container.set_visible(false);
        // Refocus active text view
        if let Some((_, view)) = (self.get_target)() {
            view.grab_focus();
        }
    }

    fn search_flags(&self) -> TextSearchFlags {
        let mut flags = TextSearchFlags::VISIBLE_ONLY;
        if !self.match_case_check.is_active() {
            flags |= TextSearchFlags::CASE_INSENSITIVE;
        }
        flags
    }

    pub fn find_next(&self, advance: bool) {
        let query = self.search_entry.text().to_string();
        if query.is_empty() {
            self.status_label.set_label("");
            return;
        }

        let Some((buffer, view)) = (self.get_target)() else {
            self.status_label.set_label("");
            return;
        };

        let flags = self.search_flags();

        // Determine where to start searching from
        let search_start = if advance {
            if let Some((_, end)) = buffer.selection_bounds() {
                end
            } else {
                buffer.iter_at_mark(&buffer.get_insert())
            }
        } else if let Some((start, _)) = buffer.selection_bounds() {
            start
        } else {
            buffer.iter_at_mark(&buffer.get_insert())
        };

        // Forward search from search_start
        let match_found = search_start
            .forward_search(&query, flags, None)
            .or_else(|| {
                // Wrap around from beginning of document
                let start_iter = buffer.start_iter();
                start_iter.forward_search(&query, flags, None)
            });

        if let Some((m_start, m_end)) = match_found {
            buffer.select_range(&m_start, &m_end);
            view.scroll_mark_onscreen(&buffer.get_insert());
            self.update_match_count(&buffer, &query, flags, m_start.offset());
        } else {
            self.status_label.set_label("No results");
        }
    }

    pub fn find_prev(&self) {
        let query = self.search_entry.text().to_string();
        if query.is_empty() {
            self.status_label.set_label("");
            return;
        }

        let Some((buffer, view)) = (self.get_target)() else {
            self.status_label.set_label("");
            return;
        };

        let flags = self.search_flags();

        // Search backward from selection start or cursor
        let search_start = if let Some((start, _)) = buffer.selection_bounds() {
            start
        } else {
            buffer.iter_at_mark(&buffer.get_insert())
        };

        let match_found = search_start
            .backward_search(&query, flags, None)
            .or_else(|| {
                // Wrap around from end of document
                let end_iter = buffer.end_iter();
                end_iter.backward_search(&query, flags, None)
            });

        if let Some((m_start, m_end)) = match_found {
            buffer.select_range(&m_start, &m_end);
            view.scroll_mark_onscreen(&buffer.get_insert());
            self.update_match_count(&buffer, &query, flags, m_start.offset());
        } else {
            self.status_label.set_label("No results");
        }
    }

    fn update_match_count(
        &self,
        buffer: &TextBuffer,
        query: &str,
        flags: TextSearchFlags,
        current_offset: i32,
    ) {
        let mut total = 0;
        let mut current_idx = 0;
        let mut iter = buffer.start_iter();

        while let Some((m_start, m_end)) = iter.forward_search(query, flags, None) {
            total += 1;
            if m_start.offset() == current_offset {
                current_idx = total;
            }
            iter = m_end;
        }

        if total == 0 {
            self.status_label.set_label("No results");
        } else if current_idx > 0 {
            self.status_label
                .set_label(&format!("{}/{}", current_idx, total));
        } else {
            self.status_label.set_label(&format!("{} matches", total));
        }
    }

    pub fn replace_current(&self) {
        let query = self.search_entry.text().to_string();
        if query.is_empty() {
            return;
        }

        let Some((buffer, _view)) = (self.get_target)() else {
            return;
        };

        let flags = self.search_flags();
        let replace_text = self.replace_entry.text().to_string();

        let has_matching_selection = if let Some((s_start, s_end)) = buffer.selection_bounds() {
            let selected_text = buffer.text(&s_start, &s_end, false);
            if flags.contains(TextSearchFlags::CASE_INSENSITIVE) {
                selected_text.to_lowercase() == query.to_lowercase()
            } else {
                selected_text == query
            }
        } else {
            false
        };

        if has_matching_selection {
            buffer.begin_user_action();
            if let Some((mut s_start, mut s_end)) = buffer.selection_bounds() {
                buffer.delete(&mut s_start, &mut s_end);
                buffer.insert(&mut s_start, &replace_text);
            }
            buffer.end_user_action();
            self.find_next(true);
        } else {
            // Find next match and if found, replace it
            self.find_next(true);
            if let Some((mut s_start, mut s_end)) = buffer.selection_bounds() {
                let selected_text = buffer.text(&s_start, &s_end, false);
                let matched = if flags.contains(TextSearchFlags::CASE_INSENSITIVE) {
                    selected_text.to_lowercase() == query.to_lowercase()
                } else {
                    selected_text == query
                };
                if matched {
                    buffer.begin_user_action();
                    buffer.delete(&mut s_start, &mut s_end);
                    buffer.insert(&mut s_start, &replace_text);
                    buffer.end_user_action();
                    self.find_next(true);
                }
            }
        }
    }

    pub fn replace_all(&self) {
        let query = self.search_entry.text().to_string();
        if query.is_empty() {
            return;
        }

        let Some((buffer, _view)) = (self.get_target)() else {
            return;
        };

        let flags = self.search_flags();
        let replace_text = self.replace_entry.text().to_string();

        buffer.begin_user_action();

        let mut replaced_count = 0;
        let mut iter = buffer.start_iter();

        while let Some((mut m_start, mut m_end)) = iter.forward_search(&query, flags, None) {
            let start_offset = m_start.offset();
            buffer.delete(&mut m_start, &mut m_end);
            buffer.insert(&mut m_start, &replace_text);
            replaced_count += 1;

            // Continue searching from after the replaced text to prevent infinite loops
            iter = m_start;
            if iter.offset() <= start_offset && replace_text.is_empty() && !iter.forward_char() {
                break;
            }
        }

        buffer.end_user_action();

        (self.show_toast)(&format!("Replaced {} occurrences", replaced_count));
        self.find_next(false);
    }
}
