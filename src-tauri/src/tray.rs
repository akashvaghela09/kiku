//! Staying out of the way without going away.
//!
//! Kiku is a dictation key that happens to have a window. Closing that window means
//! "I am done looking at settings", not "stop listening" - so the window hides and the
//! key keeps working. Something has to remain visible when it does, or the application
//! is running with no evidence and no way back, which is indistinguishable from a
//! process that failed to quit.
//!
//! That something is a tray icon. It carries the only two things there are to say:
//! show the window, and quit for real.

use tauri::menu::{Menu, MenuEvent, MenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WebviewWindow};

/// Menu item ids. Matched on in [`on_menu`], so they live next to it.
const SHOW: &str = "show";
const QUIT: &str = "quit";

/// Build the tray icon and its menu.
pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, SHOW, "Show Kiku", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, QUIT, "Quit Kiku", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    let Some(icon) = app.default_window_icon().cloned() else {
        // Only reachable in a bundle built without icons. The window still works; the
        // tray would just be an invisible click target, which is worse than none.
        tracing::warn!("no window icon in this bundle; the tray is unavailable");
        return Ok(());
    };

    TrayIconBuilder::with_id("kiku")
        .icon(icon)
        // A monochrome stencil, so macOS can invert it for a light or dark menu bar.
        .icon_as_template(true)
        .tooltip("Kiku")
        .menu(&menu)
        // The menu is for the right button. A left click should just show the window,
        // which is what a single visible affordance is for.
        .show_menu_on_left_click(false)
        .on_menu_event(on_menu)
        .on_tray_icon_event(on_icon)
        .build(app)?;

    Ok(())
}

fn on_menu(app: &AppHandle, event: MenuEvent) {
    match event.id().as_ref() {
        SHOW => reveal(app),
        // The only way out. Everything else leaves Kiku listening.
        QUIT => app.exit(0),
        other => tracing::debug!(id = other, "unhandled tray menu item"),
    }
}

fn on_icon(tray: &tauri::tray::TrayIcon, event: TrayIconEvent) {
    if let TrayIconEvent::Click {
        button: tauri::tray::MouseButton::Left,
        button_state: tauri::tray::MouseButtonState::Up,
        ..
    } = event
    {
        reveal(tray.app_handle());
    }
}

/// Bring the main window back, wherever it went.
///
/// Unminimising as well as showing: a window hidden while minimised comes back
/// minimised, which looks like nothing happened.
pub fn reveal(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        tracing::warn!("there is no main window to show");
        return;
    };

    restore(&window);
}

fn restore(window: &WebviewWindow) {
    if let Err(error) = window.unminimize() {
        tracing::debug!(%error, "could not unminimise the window");
    }
    if let Err(error) = window.show() {
        tracing::warn!(%error, "could not show the window");
        return;
    }
    if let Err(error) = window.set_focus() {
        tracing::debug!(%error, "could not focus the window");
    }
}
