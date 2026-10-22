use gdk4_x11::X11Surface;
use gtk4::prelude::*;
use std::process::Command;

/// Attempts to get the X11 Window ID (XID) for the given GTK window surface.
pub fn get_x11_xid(window: &libadwaita::ApplicationWindow) -> Option<u64> {
    let surface = window.surface()?;
    let x11_surface = surface.downcast_ref::<X11Surface>()?;
    Some(x11_surface.xid())
}

/// Sets or unsets the _NET_WM_STATE_ABOVE property on an X11 window.
pub fn set_x11_always_on_top(xid: u64, enable: bool) {
    // 1. Send EWMH ClientMessage via libX11.so.6
    send_ewmh_client_message(xid, enable);

    // 2. Also update property via xprop for compatibility
    if enable {
        let _ = Command::new("xprop")
            .args([
                "-id",
                &xid.to_string(),
                "-f",
                "_NET_WM_STATE",
                "32a",
                "-set",
                "_NET_WM_STATE",
                "_NET_WM_STATE_ABOVE",
            ])
            .output();
    } else {
        let _ = Command::new("xprop")
            .args(["-id", &xid.to_string(), "-remove", "_NET_WM_STATE"])
            .output();
    }
}

/// Moves and resizes an X11 window using xdotool.
pub fn move_and_resize_x11(xid: u64, x: i32, y: i32, width: i32, height: i32) {
    let _ = Command::new("xdotool")
        .args([
            "windowsize",
            &xid.to_string(),
            &width.to_string(),
            &height.to_string(),
        ])
        .output();
    let _ = Command::new("xdotool")
        .args([
            "windowmove",
            &xid.to_string(),
            &x.to_string(),
            &y.to_string(),
        ])
        .output();
    let _ = Command::new("xdotool")
        .args(["windowraise", &xid.to_string()])
        .output();
}

/// Queries current window geometry (x, y, width, height) using xdotool.
pub fn query_geometry_x11(xid: u64) -> Option<(i32, i32, i32, i32)> {
    let output = Command::new("xdotool")
        .args(["getwindowgeometry", &xid.to_string()])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut pos_x = None;
    let mut pos_y = None;
    let mut width = None;
    let mut height = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Position:") {
            // e.g. "Position: 1280,36 (screen: 0)"
            let after = trimmed.strip_prefix("Position:")?.trim();
            let coords = after.split_whitespace().next()?;
            let mut parts = coords.split(',');
            pos_x = parts.next()?.parse::<i32>().ok();
            pos_y = parts.next()?.parse::<i32>().ok();
        } else if trimmed.starts_with("Geometry:") {
            // e.g. "Geometry: 640x1044"
            let after = trimmed.strip_prefix("Geometry:")?.trim();
            let mut parts = after.split('x');
            width = parts.next()?.parse::<i32>().ok();
            height = parts.next()?.parse::<i32>().ok();
        }
    }

    match (pos_x, pos_y, width, height) {
        (Some(x), Some(y), Some(w), Some(h)) => Some((x, y, w, h)),
        _ => None,
    }
}

type XOpenDisplayFn = unsafe extern "C" fn(*const std::ffi::c_char) -> *mut std::ffi::c_void;
type XDefaultRootWindowFn = unsafe extern "C" fn(*mut std::ffi::c_void) -> std::ffi::c_ulong;
type XInternAtomFn = unsafe extern "C" fn(
    *mut std::ffi::c_void,
    *const std::ffi::c_char,
    std::ffi::c_int,
) -> std::ffi::c_ulong;
type XSendEventFn = unsafe extern "C" fn(
    *mut std::ffi::c_void,
    std::ffi::c_ulong,
    std::ffi::c_int,
    std::ffi::c_long,
    *mut std::ffi::c_void,
) -> std::ffi::c_int;
type XFlushFn = unsafe extern "C" fn(*mut std::ffi::c_void) -> std::ffi::c_int;
type XCloseDisplayFn = unsafe extern "C" fn(*mut std::ffi::c_void) -> std::ffi::c_int;

#[repr(C)]
struct XClientMessageEvent {
    type_: std::ffi::c_int,
    serial: std::ffi::c_ulong,
    send_event: std::ffi::c_int,
    display: *mut std::ffi::c_void,
    window: std::ffi::c_ulong,
    message_type: std::ffi::c_ulong,
    format: std::ffi::c_int,
    data: [std::ffi::c_long; 5],
}

#[repr(C)]
union XEvent {
    client: std::mem::ManuallyDrop<XClientMessageEvent>,
    _pad: [std::ffi::c_long; 24],
}

fn send_ewmh_client_message(xid: u64, enable: bool) {
    unsafe {
        let handle = libc::dlopen(c"libX11.so.6".as_ptr(), libc::RTLD_LAZY);
        if handle.is_null() {
            return;
        }

        let open_display_ptr = libc::dlsym(handle, c"XOpenDisplay".as_ptr());
        let default_root_ptr = libc::dlsym(handle, c"XDefaultRootWindow".as_ptr());
        let intern_atom_ptr = libc::dlsym(handle, c"XInternAtom".as_ptr());
        let send_event_ptr = libc::dlsym(handle, c"XSendEvent".as_ptr());
        let flush_ptr = libc::dlsym(handle, c"XFlush".as_ptr());
        let close_display_ptr = libc::dlsym(handle, c"XCloseDisplay".as_ptr());

        if !open_display_ptr.is_null()
            && !default_root_ptr.is_null()
            && !intern_atom_ptr.is_null()
            && !send_event_ptr.is_null()
            && !flush_ptr.is_null()
            && !close_display_ptr.is_null()
        {
            let open_display: XOpenDisplayFn = std::mem::transmute(open_display_ptr);
            let default_root_window: XDefaultRootWindowFn = std::mem::transmute(default_root_ptr);
            let intern_atom: XInternAtomFn = std::mem::transmute(intern_atom_ptr);
            let send_event: XSendEventFn = std::mem::transmute(send_event_ptr);
            let flush: XFlushFn = std::mem::transmute(flush_ptr);
            let close_display: XCloseDisplayFn = std::mem::transmute(close_display_ptr);

            let display = open_display(std::ptr::null());
            if !display.is_null() {
                let root = default_root_window(display);
                let net_wm_state = intern_atom(display, c"_NET_WM_STATE".as_ptr(), 0);
                let net_wm_state_above = intern_atom(display, c"_NET_WM_STATE_ABOVE".as_ptr(), 0);

                let client = XClientMessageEvent {
                    type_: 33, // ClientMessage
                    serial: 0,
                    send_event: 1,
                    display,
                    window: xid as std::ffi::c_ulong,
                    message_type: net_wm_state,
                    format: 32,
                    data: [
                        if enable { 1 } else { 0 }, // 1 = _NET_WM_STATE_ADD, 0 = _NET_WM_STATE_REMOVE
                        net_wm_state_above as std::ffi::c_long,
                        0,
                        1, // source: normal application
                        0,
                    ],
                };

                let mut event = XEvent {
                    client: std::mem::ManuallyDrop::new(client),
                };

                let mask = (1 << 20) | (1 << 19); // SubstructureRedirectMask | SubstructureNotifyMask
                send_event(
                    display,
                    root,
                    0,
                    mask,
                    &mut event as *mut _ as *mut std::ffi::c_void,
                );
                flush(display);
                close_display(display);
            }
        }

        libc::dlclose(handle);
    }
}
