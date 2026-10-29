# Rust Book desktop app

A Tauri v2 wrapper around *The Rust Programming Language*. It bundles the book
source and an `mdbook` binary, then runs `mdbook serve` on `127.0.0.1` at launch.
Nothing is rendered at build time: the HTML is built in the app's cache the
first time it starts.

Linux only for now; `cargo tauri build` produces a `.deb` and an AppImage.

## Build

```sh
cargo install tauri-cli --version "^2"
cargo install mdbook --version 0.5.1 --locked
cargo tauri build
```

`scripts/stage.sh` runs automatically and copies the book, mdbook and the two
preprocessors into `src-tauri/resources/`. Set `MDBOOK_BIN` to use a different
mdbook. `cargo tauri dev` uses the same staging step.

The bundles land in `src-tauri/target/release/bundle/`.

Building also needs WebKitGTK 4.1, GTK 3 and librsvg; see the
[Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

## How it works

1. `scripts/stage.sh` copies the source and binaries; `book.toml` is rewritten
   to call the bundled preprocessors instead of `cargo run`.
2. On launch, `src-tauri/src/server.rs` copies both into
   `~/.cache/dev.hxmbl.rustbook/book`, makes the binaries executable, picks a
   free port (preferring 47823 so the saved theme survives restarts) and starts
   `mdbook serve`.
3. The window shows `ui/index.html` until the server answers 200, then navigates
   to it. Failures appear on that page with the path to `mdbook.log`.
4. mdbook is killed when the app exits, including after a crash. A second launch
   focuses the existing window instead of starting a rival server.

The binaries are resources, not `externalBin` sidecars, so nothing is installed
into `/usr/bin` and a real mdbook install is never overwritten. External links
open in the system browser.

## Updating the book

After pulling changes to `src/` or `listings/`, stage and rebuild. The cache is
refreshed on every launch, so the new text appears immediately.
