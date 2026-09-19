//! Placing the listening overlay on screen.
//!
//! Two decisions here are load-bearing:
//!
//! * **Anchor to the monitor's work area, not its bounds.** The work area excludes the
//!   macOS Dock and the Windows taskbar. Using the full bounds is the most common
//!   shipped bug in this category — the capsule ends up underneath the dock.
//! * **Recompute on every show, never cache.** Monitors get plugged in, resolutions
//!   change, and a cached position puts the overlay on a screen that is no longer
//!   there.
//!
//! The OS window itself is a fixed 320×96 and is never resized. Resizing a window per
//! state costs a compositor round-trip and flickers on X11; only the CSS capsule inside
//! it animates.

use std::sync::Mutex;

use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, PhysicalPosition, WebviewWindow};

/// Requested size of the overlay window, in logical pixels.
///
/// Larger than the capsule on purpose: the widest state is 260 px, its shadow bleeds
/// about 28 px, and the entry animation translates 8 px.
///
/// This is a *request*. WebKitGTK gives its webview a natural minimum height that a
/// GTK window will not shrink below, so on Linux the real window can be taller than
/// this. That is harmless — the extra area is transparent and click-through — but it
/// means the position must be computed from the window's actual size, which
/// [`place`] does.
pub const WINDOW_SIZE: LogicalSize<f64> = LogicalSize::new(320.0, 96.0);

/// Gap between the bottom of the capsule and the bottom of the work area.
///
/// The capsule is anchored to the *bottom* of the overlay window by the same amount in
/// CSS, and the window's bottom edge is placed on the work area's bottom edge. That
/// makes the window's exact height irrelevant — which matters, because WebKitGTK
/// forces it larger than requested and some window managers clamp it again.
pub const BOTTOM_MARGIN: f64 = 40.0;

pub const LABEL: &str = "overlay";

/// The overlay window's real size, once it has been measured.
///
/// A hidden window has not been realised and reports 0×0, so the first placement has
/// to assume [`WINDOW_SIZE`]. After the first show the actual size — which WebKitGTK
/// may have forced larger — is recorded here and every later placement is exact.
static MEASURED_SIZE: Mutex<Option<(f64, f64)>> = Mutex::new(None);

pub fn window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(LABEL)
}

/// Move the overlay to the bottom-centre of the work area it belongs on, then show it.
///
/// Bottom-centre rather than a corner: corners are where OS notifications live, and it
/// is the least content-destructive band in editors, chat apps and browsers, where the
/// caret usually sits higher up the screen.
pub fn show(app: &AppHandle) {
    let Some(overlay) = window(app) else {
        return;
    };

    if let Err(error) = place(app, &overlay) {
        // A positioning failure should still leave the user with visible feedback,
        // even if it lands in the wrong place.
        tracing::warn!(%error, "could not position the overlay");
    }

    if let Err(error) = overlay.show() {
        tracing::warn!(%error, "could not show the overlay");
        return;
    }

    make_click_through(&overlay);

    // Now that the window is realised it has a real size. Record it, and correct the
    // placement if the platform gave us something other than what we asked for.
    if remember_size(&overlay) {
        if let Err(error) = place(app, &overlay) {
            tracing::debug!(%error, "could not correct the overlay placement");
        }
    }
}

/// Record the window's realised size. Returns whether it differed from what was
/// assumed, meaning the placement needs redoing.
fn remember_size(overlay: &WebviewWindow) -> bool {
    let Ok(size) = overlay.outer_size() else {
        return false;
    };
    if size.width == 0 || size.height == 0 {
        return false;
    }

    let scale = overlay.scale_factor().unwrap_or(1.0);
    let measured = (
        f64::from(size.width) / scale,
        f64::from(size.height) / scale,
    );

    let Ok(mut cached) = MEASURED_SIZE.lock() else {
        return false;
    };
    let changed = *cached != Some(measured);
    *cached = Some(measured);
    changed
}

/// Make the overlay transparent to the mouse.
///
/// Applied on every show rather than once at startup, because on Linux the call
/// reaches into the GDK window behind the GTK widget, and that does not exist until
/// the window has been realised — which only happens the first time it is shown.
/// Calling it during setup panics inside the windowing layer.
fn make_click_through(overlay: &WebviewWindow) {
    if let Err(error) = overlay.set_ignore_cursor_events(true) {
        tracing::warn!(%error, "the overlay may intercept clicks");
    }
}

pub fn hide(app: &AppHandle) {
    if let Some(overlay) = window(app) {
        let _ = overlay.hide();
    }
}

fn place(app: &AppHandle, overlay: &WebviewWindow) -> tauri::Result<()> {
    // Size is reasserted here rather than only at startup: a size set on a window GTK
    // has not realised does not stick, and the overlay is hidden — therefore
    // unrealised — until the first time it is shown.
    //
    // The minimum is cleared first because WebKitGTK gives a webview a natural
    // minimum height larger than our capsule, and a GTK window will not shrink below
    // its child's minimum however often `set_size` is called.
    overlay.set_min_size(Some(WINDOW_SIZE))?;
    overlay.set_size(WINDOW_SIZE)?;

    // The monitor under the pointer is the best cross-platform proxy for "the screen
    // the user is working on". Querying the focused window's monitor needs per-platform
    // APIs that are unavailable or unreliable on X11.
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|PhysicalPosition { x, y }| overlay.monitor_from_point(x, y).ok().flatten())
        .or_else(|| overlay.primary_monitor().ok().flatten());

    let Some(monitor) = monitor else {
        tracing::debug!("no monitor available; leaving the overlay where it is");
        return Ok(());
    };

    let scale = monitor.scale_factor();
    let area = monitor.work_area();

    // Work in logical pixels so the arithmetic matches the CSS, then let Tauri convert.
    let area_x = f64::from(area.position.x) / scale;
    let area_y = f64::from(area.position.y) / scale;
    let area_width = f64::from(area.size.width) / scale;
    let area_height = f64::from(area.size.height) / scale;

    // Use the measured size where we have one. A window that is still hidden reports
    // 0×0, which would place the capsule against the wrong edge entirely.
    let (window_width, window_height) = MEASURED_SIZE
        .lock()
        .ok()
        .and_then(|cached| *cached)
        .unwrap_or((WINDOW_SIZE.width, WINDOW_SIZE.height));

    let (x, y) = bottom_centre(
        (area_x, area_y, area_width, area_height),
        (window_width, window_height),
    );

    tracing::debug!(
        area_x,
        area_y,
        area_width,
        area_height,
        window_width,
        window_height,
        scale,
        x,
        y,
        "overlay placement"
    );

    overlay.set_position(LogicalPosition::new(x, y))?;
    Ok(())
}

/// Where the overlay window goes: horizontally centred, with its bottom edge on the
/// bottom edge of the work area.
///
/// The capsule's distance from the taskbar is set in CSS, by the padding beneath it,
/// so this only has to get the window's bottom edge right. A window manager that
/// clamps an oversized window into the work area arrives at exactly the same place.
///
/// Extracted so the arithmetic is testable without a display server.
fn bottom_centre(area: (f64, f64, f64, f64), window: (f64, f64)) -> (f64, f64) {
    let (area_x, area_y, area_width, area_height) = area;
    let (window_width, window_height) = window;

    (
        area_x + (area_width - window_width) / 2.0,
        area_y + area_height - window_height,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The window size the tests position against.
    const WINDOW: (f64, f64) = (WINDOW_SIZE.width, WINDOW_SIZE.height);

    #[test]
    fn a_single_display_centres_the_overlay_horizontally() {
        let (x, _) = bottom_centre((0.0, 0.0, 1920.0, 1080.0), WINDOW);
        assert_eq!(x, (1920.0 - 320.0) / 2.0);
    }

    #[test]
    fn the_overlay_sits_above_the_bottom_of_the_work_area() {
        let (_, y) = bottom_centre((0.0, 0.0, 1920.0, 1080.0), WINDOW);
        assert_eq!(
            y + WINDOW_SIZE.height,
            1080.0,
            "window bottom on work area bottom"
        );
    }

    #[test]
    fn a_taskbar_shrinking_the_work_area_lifts_the_overlay_above_it() {
        // A 48px taskbar: the work area is shorter than the display.
        let with_taskbar = bottom_centre((0.0, 0.0, 1920.0, 1032.0), WINDOW);
        let without = bottom_centre((0.0, 0.0, 1920.0, 1080.0), WINDOW);
        assert!(
            with_taskbar.1 < without.1,
            "the overlay must rise to clear the taskbar"
        );
    }

    #[test]
    fn a_macos_menu_bar_offset_is_respected() {
        // macOS work areas start below the menu bar, so the origin is non-zero.
        let (_, y) = bottom_centre((0.0, 25.0, 1440.0, 875.0), WINDOW);
        assert_eq!(y + WINDOW_SIZE.height, 25.0 + 875.0);
    }

    #[test]
    fn a_secondary_monitor_to_the_right_gets_its_own_coordinates() {
        let (x, _) = bottom_centre((1920.0, 0.0, 2560.0, 1440.0), WINDOW);
        assert_eq!(x, 1920.0 + (2560.0 - 320.0) / 2.0);
        assert!(x > 1920.0, "the overlay must land on the second display");
    }

    #[test]
    fn a_monitor_left_of_the_primary_gets_negative_coordinates() {
        // Displays to the left of the primary have negative origins, which is exactly
        // where naive `width / 2` arithmetic goes wrong.
        let (x, _) = bottom_centre((-1920.0, 0.0, 1920.0, 1080.0), WINDOW);
        assert!(x < 0.0, "expected a negative origin, got {x}");
        assert_eq!(x, -1920.0 + (1920.0 - 320.0) / 2.0);
    }

    /// Whether a window of this size contains the capsule in its widest state, the
    /// shadow around it, and the 8px it translates on entry.
    ///
    /// The shadow is `0 8px 28px`, so it bleeds asymmetrically: roughly
    /// blur/2 + offset below the capsule and blur/2 - offset above it.
    fn contains_every_capsule_state(window: LogicalSize<f64>) -> bool {
        const CAPSULE_MAX_WIDTH: f64 = 260.0;
        const CAPSULE_HEIGHT: f64 = 44.0;
        const BLUR: f64 = 28.0;
        const OFFSET_Y: f64 = 8.0;
        const ENTRY_TRANSLATE: f64 = 8.0;

        let needed_height =
            CAPSULE_HEIGHT + (BLUR / 2.0 - OFFSET_Y) + (BLUR / 2.0 + OFFSET_Y) + ENTRY_TRANSLATE;

        window.width >= CAPSULE_MAX_WIDTH + BLUR && window.height >= needed_height
    }

    #[test]
    fn the_window_is_large_enough_for_the_widest_capsule_and_its_shadow() {
        assert!(contains_every_capsule_state(WINDOW_SIZE));
    }

    #[test]
    fn a_taller_window_than_requested_still_ends_on_the_work_area_edge() {
        // WebKitGTK can force a taller window than we ask for. Whatever height it
        // picks, the bottom edge must land in the same place, because that is what
        // fixes the capsule's distance from the taskbar.
        let area = (0.0, 0.0, 1920.0, 1034.0);
        for height in [96.0, 150.0, 200.0, 400.0] {
            let (_, y) = bottom_centre(area, (320.0, height));
            assert_eq!(y + height, 1034.0, "height {height} put the edge elsewhere");
        }
    }

    #[test]
    fn a_window_sized_to_the_capsule_alone_would_clip_the_shadow() {
        // Guards the guard: the check above must be able to fail.
        assert!(!contains_every_capsule_state(LogicalSize::new(260.0, 44.0)));
    }
}
