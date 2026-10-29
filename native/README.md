# Rust Book app

A Tauri v2 wrapper around *The Rust Programming Language*, for Linux, macOS and
Android. `scripts/stage.sh` renders the book with `mdbook build` when the app is
bundled, and the HTML is embedded as the webview's frontend. Nothing runs at
launch: the window opens straight onto the pre-rendered book, so it works with
no network connection and starts instantly.

- **Linux** — a `.deb` and an AppImage.
- **macOS** — a universal (Apple Silicon + Intel) `.app` and a `.dmg`.
- **Android** — an APK.

## Build

```sh
cargo install tauri-cli --version "^2"
cargo install mdbook --version 0.5.1 --locked
cargo tauri build
```

`scripts/stage.sh` runs automatically (through Tauri's `beforeBuildCommand`)
and renders the book from the repository root into `native/ui/`, the webview's
frontendDist, which is embedded into the app. Set `MDBOOK_BIN` to use a
different mdbook. `cargo tauri dev` uses the same step.

The bundles land in `src-tauri/target/release/bundle/`. Building also needs the
usual Tauri prerequisites — WebKitGTK 4.1, GTK 3 and librsvg on Linux; see the
[Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

## Android

`gen/android` is generated, not committed. To build an APK you need the Android
SDK and NDK (27.2.12479018) and Java 17, then:

```sh
cd native
cargo tauri android init --ci --skip-targets-install   # writes src-tauri/gen/android
cargo tauri android build --debug --apk --target aarch64
```

CI (`.github/workflows/app.yml`) does exactly this and uploads the APK. The app
renders the book at build time like every other platform, so the APK embeds the
HTML — there is no `mdbook` process and nothing is read from disk at runtime.

## How it works

1. `scripts/stage.sh` renders the book from the repository root with the
   repository's own `book.toml`, so the note, listing and link-rewriter
   preprocessors are built by its `cargo run` commands. The output goes to
   `native/ui/`, which Tauri embeds as the webview's frontend.
2. On launch the window opens `index.html` on the webview's own origin —
   `tauri://localhost` on desktop, `http://tauri.localhost` on Android — served
   from those embedded assets. Tauri's built-in protocol resolves the book's
   relative links and serves the right content type for every file.
3. A navigation handler keeps the reader in one piece: pages on the app's own
   origin navigate normally, while an external link (crates.io, the standard
   library docs) opens in the system browser instead of replacing the book,
   which has no back button.
4. On desktop a second launch focuses the existing window rather than opening a
   rival copy; on Android the OS handles single-instance behaviour.

Because the book is embedded rather than read from disk, there is no cache to
refresh and no server to shut down. Rebuild to pick up changes to `src/`.

## Updating the book

After pulling changes to `src/` or `listings/`, rebuild — `stage.sh` re-renders
the book as part of the build.
