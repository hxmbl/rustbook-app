//! The desktop shell for *The Rust Programming Language*.
//!
//! There is no frontend to build. The app ships the book's Markdown, its theme
//! and an `mdbook` binary, and runs `mdbook serve` on a loopback port at launch.
//! The window shows a small loading page until the server answers, then
//! navigates to it.

mod paths;
mod server;

use std::sync::Mutex;

use tauri::{Manager, RunEvent, Url};
use tauri_plugin_opener::OpenerExt;

/// Holds the running mdbook process so it can be killed when the app exits.
struct ServerState(Mutex<Option<server::BookServer>>);

pub fn run() {
    tauri::Builder::default()
        // A second launch should surface the window that is already open: two
        // instances would fight over the same cache directory, and the second
        // one would delete the first one's book while it is being served. Tauri
        // requires this plugin to be registered before every other plugin.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .on_window_event(|window, event| {
            // A single-window reader should quit when its window closes, the way
            // it does on Linux and Windows, rather than leaving an app running
            // with no window and no way to reopen one.
            #[cfg(target_os = "macos")]
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                window.app_handle().exit(0);
            }
            #[cfg(not(target_os = "macos"))]
            let _ = (window, event);
        })
        .setup(|app| {
            app.manage(ServerState(Mutex::new(None)));

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
            let window = tauri::WebviewWindowBuilder::from_config(app.handle(), &config)?
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

            // Building the book takes a second or two, so do it off the main
            // thread and leave the loading page visible meanwhile.
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                match server::start(&handle) {
                    Ok(book) => {
                        let url = match book.url().parse::<Url>() {
                            Ok(url) => url,
                            Err(err) => {
                                server::show_error(
                                    &window,
                                    &format!("could not parse the book's address: {err}"),
                                    book.log_path(),
                                );
                                return;
                            }
                        };
                        if let Err(err) = window.navigate(url) {
                            server::show_error(
                                &window,
                                &format!("could not open the book: {err}"),
                                book.log_path(),
                            );
                            return;
                        }
                        *handle.state::<ServerState>().0.lock().unwrap() = Some(book);
                    }
                    Err(err) => {
                        eprintln!("{err}");
                        server::show_error(&window, &err, &server::log_path(&handle));
                    }
                }

                // On Linux mdbook is told to die with the thread that spawned
                // it, not with the process (PR_SET_PDEATHSIG is per-thread), so
                // this thread has to stay alive for as long as the book runs.
                // The process exiting is what ends it.
                loop {
                    std::thread::park();
                }
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to build The Rust Programming Language")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                // Also stops a server that is still starting up, which would
                // otherwise outlive the app.
                server::shutdown();
                if let Some(mut book) = app.state::<ServerState>().0.lock().unwrap().take() {
                    book.stop();
                }
            }
        });
}

/// Pages the app itself owns: the loading page and the local mdbook server.
fn is_app_url(url: &Url) -> bool {
    match url.scheme() {
        "tauri" => true,
        "http" | "https" => matches!(
            url.host_str(),
            Some("127.0.0.1" | "localhost" | "tauri.localhost")
        ),
        _ => false,
    }
}
