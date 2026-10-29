//! Where the bundled book and the helper binaries live.
//!
//! When installed they are resources (`/usr/lib/<product>/resources` on Linux,
//! `Contents/Resources/resources` in a macOS `.app`). Under `tauri dev` they may
//! still be in the source tree, so that is checked as a fallback.

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

pub fn book_source(app: &AppHandle) -> Result<PathBuf, String> {
    resource(app, "book-source")
}

pub fn bin_dir(app: &AppHandle) -> Result<PathBuf, String> {
    resource(app, "bin")
}

fn resource(app: &AppHandle, name: &str) -> Result<PathBuf, String> {
    let mut tried = Vec::new();

    if let Ok(dir) = app.path().resource_dir() {
        let candidate = dir.join("resources").join(name);
        if candidate.is_dir() {
            return Ok(candidate);
        }
        tried.push(candidate);
    }

    let candidate = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join(name);
    if candidate.is_dir() {
        return Ok(candidate);
    }
    tried.push(candidate);

    Err(format!(
        "could not find the bundled `{name}` directory (looked in {}). \
         Run native/scripts/stage.sh, then try again.",
        tried
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(" and ")
    ))
}
