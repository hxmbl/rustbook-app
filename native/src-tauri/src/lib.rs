//! The native shell for *The Rust Programming Language*, on Linux, macOS and
//! Android.
//!
//! There is no frontend to build and nothing runs at launch. `scripts/stage.sh`
//! renders the book with `mdbook build` when the app is bundled, and the HTML is
//! embedded as the webview's frontend. The window opens straight onto it: on
//! desktop through Tauri's `tauri://` scheme, on Android through
//! `http://tauri.localhost`, both served from the same embedded assets.
//!
//! The only thing this crate does at runtime is keep external links out of the
//! reader: a link to crates.io opens in the system browser rather than by
//! replacing the book, which has no back button.

use tauri::{Manager, Url};
use tauri_plugin_opener::OpenerExt;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // `single_instance` has to be registered before every other plugin so a
    // second launch is detected and forwarded early. It is desktop-only: Android
    // hands single-instance behaviour to the OS, and the plugin has no mobile
    // backend, so registering it unconditionally would not compile there.
    let builder = tauri::Builder::default();

    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
        // A second launch should surface the window that is already open rather
        // than open a second copy of the book.
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_focus();
        }
    }));

    builder
        .plugin(tauri_plugin_opener::init())
        .on_window_event(|window, event| {
            // A single-window reader should quit when its window closes, the way
            // it does on Linux and Android, rather than leaving an app running
            // with no window and no way to reopen one. macOS keeps running in the
            // dock until the user quits, the platform convention.
            #[cfg(target_os = "macos")]
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                window.app_handle().exit(0);
            }
            #[cfg(not(target_os = "macos"))]
            let _ = (window, event);
        })
        .setup(|app| {
            // The window is created here rather than by the config so the
            // navigation handler can be attached to it. Without the handler a
            // link to crates.io would replace the book, and the window has no
            // back button.
            let config = app
                .config()
                .app
                .windows
                .first()
                .cloned()
                .ok_or("tauri.conf.json does not declare a window")?;

            let opener = app.handle().clone();
            tauri::WebviewWindowBuilder::from_config(app.handle(), &config)?
                .on_navigation(move |url| {
                    if is_app_url(url) {
                        return true;
                    }
                    if let Err(err) = opener.opener().open_url(url.as_str(), None::<&str>) {
                        eprintln!("could not open {url} in the browser: {err}");
                    }
                    false
                })
                .build()?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run The Rust Programming Language");
}

/// Pages the app itself owns: the whole embedded book, which lives on the
/// webview's own origin — `tauri://localhost` on desktop, `http://tauri.localhost`
/// on Android — plus Tauri's own `tauri` scheme. Anything else is an external
/// link and belongs in the system browser.
fn is_app_url(url: &Url) -> bool {
    match url.scheme() {
        "tauri" => true,
        "http" | "https" => matches!(url.host_str(), Some("tauri.localhost")),
        _ => false,
    }
}
