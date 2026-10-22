use crate::settings::{AppSettings, MAX_PALETTE_WIDTH_FRACTION, MIN_PALETTE_WIDTH_FRACTION};
use crate::ui::window::NotepadxWindow;
use gtk4::prelude::*;
use libadwaita::{Application, ApplicationWindow};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// Calculates palette geometry (x, y, width, height) for given screen boundaries and fraction.
pub fn calculate_geometry_for_screen(
    screen_x: i32,
    screen_y: i32,
    screen_w: i32,
    screen_h: i32,
    fraction: f32,
) -> (i32, i32, i32, i32) {
    let clamped_fraction = if fraction.is_nan() {
        crate::settings::DEFAULT_PALETTE_WIDTH_FRACTION
    } else {
        fraction.clamp(MIN_PALETTE_WIDTH_FRACTION, MAX_PALETTE_WIDTH_FRACTION)
    };

    let palette_w = (screen_w as f32 * clamped_fraction).round() as i32;
    let palette_h = screen_h;
    let target_x = screen_x + (screen_w - palette_w);
    let target_y = screen_y;

    (target_x, target_y, palette_w, palette_h)
}

/// Queries active display monitor geometry and calculates palette dimensions.
pub fn calculate_palette_geometry(
    display: &gtk4::gdk::Display,
    fraction: f32,
) -> (i32, i32, i32, i32) {
    let monitors = display.monitors();
    let (screen_x, screen_y, screen_w, screen_h) = if monitors.n_items() > 0 {
        if let Some(monitor) = monitors.item(0).and_downcast::<gtk4::gdk::Monitor>() {
            let geom = monitor.geometry();
            (geom.x(), geom.y(), geom.width(), geom.height())
        } else {
            (0, 0, 1920, 1080)
        }
    } else {
        (0, 0, 1920, 1080)
    };

    calculate_geometry_for_screen(screen_x, screen_y, screen_w, screen_h, fraction)
}

pub struct PaletteWindow {
    window: ApplicationWindow,
    editor_window: Rc<NotepadxWindow>,
    is_pinned: Rc<RefCell<bool>>,
    last_toggle_time: Rc<RefCell<Instant>>,
}

impl PaletteWindow {
    pub fn new(app: &Application) -> Rc<Self> {
        let is_pinned = Rc::new(RefCell::new(false));
        let editor_window = NotepadxWindow::new_palette(app, Some(is_pinned.clone()));
        let window = editor_window.window().clone();
        *is_pinned.borrow_mut() = editor_window.settings().borrow().palette_pinned;

        Rc::new(Self {
            window,
            editor_window,
            is_pinned,
            last_toggle_time: Rc::new(RefCell::new(
                Instant::now()
                    .checked_sub(Duration::from_secs(1))
                    .unwrap_or_else(Instant::now),
            )),
        })
    }

    pub fn window(&self) -> &ApplicationWindow {
        &self.window
    }

    pub fn editor_window(&self) -> &Rc<NotepadxWindow> {
        &self.editor_window
    }

    pub fn is_visible(&self) -> bool {
        self.window.is_visible()
    }

    pub fn is_pinned(&self) -> bool {
        *self.is_pinned.borrow()
    }

    pub fn set_pinned(&self, pinned: bool) {
        *self.is_pinned.borrow_mut() = pinned;
        self.editor_window.set_pinned(pinned);
    }

    pub fn settings(&self) -> Rc<RefCell<AppSettings>> {
        self.editor_window.settings()
    }

    pub fn update_geometry(&self) {
        let fraction = self.settings().borrow().palette_width_fraction;
        if let Some(display) = gtk4::gdk::Display::default() {
            let (_x, _y, w, h) = calculate_palette_geometry(&display, fraction);
            self.window.set_default_size(w, h);
        }
    }

    pub fn focus_active_editor(&self) {
        self.editor_window.focus_active_view();
    }

    pub fn show(&self) {
        self.update_geometry();
        self.window.present();
        self.focus_active_editor();
    }

    pub fn hide(&self) {
        self.window.set_visible(false);
    }

    pub fn toggle(&self) {
        let mut last = self.last_toggle_time.borrow_mut();
        if last.elapsed() < Duration::from_millis(200) {
            return;
        }
        *last = Instant::now();

        if self.window.is_visible() {
            self.hide();
        } else {
            self.show();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_geometry_right_column() {
        // Standard 1080p single monitor
        let (x, y, w, h) = calculate_geometry_for_screen(0, 0, 1920, 1080, 0.3333);
        assert_eq!(w, 640);
        assert_eq!(h, 1080);
        assert_eq!(x, 1920 - 640);
        assert_eq!(y, 0);

        // Standard 1440p monitor
        let (x, y, w, h) = calculate_geometry_for_screen(0, 0, 2560, 1440, 0.3333);
        assert_eq!(w, 853);
        assert_eq!(h, 1440);
        assert_eq!(x, 2560 - 853);
        assert_eq!(y, 0);
    }

    #[test]
    fn test_calculate_geometry_multi_monitor_offset() {
        // Second monitor positioned to the right (x = 1920)
        let (x, y, w, h) = calculate_geometry_for_screen(1920, 0, 1920, 1080, 0.3333);
        assert_eq!(w, 640);
        assert_eq!(h, 1080);
        assert_eq!(x, 1920 + (1920 - 640)); // 3200
        assert_eq!(y, 0);
    }

    #[test]
    fn test_calculate_geometry_clamping() {
        // Below min clamp (0.10 -> 0.20)
        let (_x, _y, w_min, _h) = calculate_geometry_for_screen(0, 0, 1000, 1000, 0.10);
        assert_eq!(w_min, 200);

        // Above max clamp (0.90 -> 0.60)
        let (_x, _y, w_max, _h) = calculate_geometry_for_screen(0, 0, 1000, 1000, 0.90);
        assert_eq!(w_max, 600);

        // NaN fallback (0.3333)
        let (_x, _y, w_nan, _h) = calculate_geometry_for_screen(0, 0, 1000, 1000, f32::NAN);
        assert_eq!(w_nan, 333);
    }
}
