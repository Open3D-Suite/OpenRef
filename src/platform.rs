// SPDX-License-Identifier: GPL-3.0-or-later
//! The little OpenRef needs from Windows itself: the colour of a pixel anywhere on the screen
//! (for the colour picker) and a system-wide shortcut (to leave click-through mode, when the
//! window no longer gets the mouse or the keyboard).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Colour of a screen pixel (physical pixels, desktop coordinates) as [r, g, b].
#[cfg(windows)]
pub fn screen_pixel(x: i32, y: i32) -> Option<[u8; 3]> {
    use windows_sys::Win32::Graphics::Gdi::{CLR_INVALID, GetDC, GetPixel, ReleaseDC};
    // SAFETY: the desktop DC is taken and given back here; GetPixel only reads.
    unsafe {
        let dc = GetDC(std::ptr::null_mut());
        if dc.is_null() {
            return None;
        }
        let color = GetPixel(dc, x, y);
        ReleaseDC(std::ptr::null_mut(), dc);
        if color == CLR_INVALID {
            return None;
        }
        Some([(color & 0xff) as u8, ((color >> 8) & 0xff) as u8, ((color >> 16) & 0xff) as u8])
    }
}

#[cfg(not(windows))]
pub fn screen_pixel(_x: i32, _y: i32) -> Option<[u8; 3]> {
    None
}

/// Whether a process with this id is still running.
#[cfg(windows)]
pub fn process_alive(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
    use windows_sys::Win32::System::Threading::{GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    // SAFETY: the handle is checked and closed here.
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return false;
        }
        let mut code = 0u32;
        let ok = GetExitCodeProcess(handle, &mut code) != 0;
        CloseHandle(handle);
        ok && code == STILL_ACTIVE as u32
    }
}

#[cfg(not(windows))]
pub fn process_alive(_pid: u32) -> bool {
    true
}

/// A system-wide Ctrl+Alt+<key> that works while it is alive. Its own thread owns the
/// registration (Windows wants the same thread to register, wait and unregister).
pub struct GlobalShortcut {
    pressed: Arc<AtomicBool>,
    #[cfg(windows)]
    thread_id: u32,
}

impl GlobalShortcut {
    /// Registers Ctrl+Alt+`key` (an uppercase letter). Fails when another program has it.
    #[cfg(windows)]
    pub fn ctrl_alt(key: char, ctx: egui::Context) -> Result<Self, String> {
        use windows_sys::Win32::System::Threading::GetCurrentThreadId;
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, RegisterHotKey, UnregisterHotKey};
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, WM_HOTKEY};

        let pressed = Arc::new(AtomicBool::new(false));
        let flag = pressed.clone();
        let (answer, registered) = std::sync::mpsc::sync_channel::<Result<u32, String>>(1);
        let vk = key.to_ascii_uppercase() as u32;
        std::thread::spawn(move || {
            // SAFETY: plain Win32 calls on this thread's own message queue.
            unsafe {
                let id = 1;
                if RegisterHotKey(std::ptr::null_mut(), id, MOD_CONTROL | MOD_ALT | MOD_NOREPEAT, vk) == 0 {
                    let _ = answer.send(Err(format!("Ctrl+Alt+{key} is taken by another program")));
                    return;
                }
                let _ = answer.send(Ok(GetCurrentThreadId()));
                let mut msg: MSG = std::mem::zeroed();
                // GetMessageW returns 0 on WM_QUIT (sent by Drop) and -1 on error.
                while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                    if msg.message == WM_HOTKEY {
                        flag.store(true, Ordering::SeqCst);
                        ctx.request_repaint();
                    }
                }
                UnregisterHotKey(std::ptr::null_mut(), id);
            }
        });
        let thread_id = registered.recv().map_err(|e| e.to_string())??;
        Ok(Self { pressed, thread_id })
    }

    #[cfg(not(windows))]
    pub fn ctrl_alt(_key: char, _ctx: egui::Context) -> Result<Self, String> {
        Err("global shortcuts are only made on Windows".into())
    }

    /// True once after each press.
    pub fn take_pressed(&self) -> bool {
        self.pressed.swap(false, Ordering::SeqCst)
    }
}

impl Drop for GlobalShortcut {
    fn drop(&mut self) {
        #[cfg(windows)]
        // SAFETY: asks the shortcut's thread to stop; it unregisters the shortcut itself.
        unsafe {
            use windows_sys::Win32::UI::WindowsAndMessaging::{PostThreadMessageW, WM_QUIT};
            PostThreadMessageW(self.thread_id, WM_QUIT, 0, 0);
        }
    }
}

/// A colour written the ways artists paste it.
pub struct ColorText {
    pub hex: String,
    pub rgb: String,
    pub hsv: String,
}

pub fn describe(rgb: [u8; 3]) -> ColorText {
    let [r, g, b] = rgb;
    let (h, s, v) = to_hsv(rgb);
    ColorText {
        hex: format!("#{r:02X}{g:02X}{b:02X}"),
        rgb: format!("{r}, {g}, {b}"),
        hsv: format!("{:.0}°, {:.0}%, {:.0}%", h, s * 100.0, v * 100.0),
    }
}

/// Hue in degrees, saturation and value in 0..1.
pub fn to_hsv([r, g, b]: [u8; 3]) -> (f32, f32, f32) {
    let (r, g, b) = (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let hue = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / delta).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    let saturation = if max == 0.0 { 0.0 } else { delta / max };
    (hue, saturation, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_are_described_as_hex_rgb_and_hsv() {
        let text = describe([255, 128, 0]);
        assert_eq!(text.hex, "#FF8000");
        assert_eq!(text.rgb, "255, 128, 0");
        assert_eq!(text.hsv, "30°, 100%, 100%");
    }

    #[test]
    fn hsv_of_greys_and_primaries() {
        assert_eq!(to_hsv([0, 0, 0]), (0.0, 0.0, 0.0));
        let (h, s, v) = to_hsv([0, 0, 255]);
        assert_eq!((h.round(), s, v), (240.0, 1.0, 1.0));
        let (h, _, _) = to_hsv([255, 0, 255]);
        assert_eq!(h.round(), 300.0);
    }
}
