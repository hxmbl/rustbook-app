#!/usr/bin/env bash
#
# Render the book for the app:
#
#   native/ui/   the HTML that `mdbook build` writes, which is the webview's
#                frontendDist and is embedded into the app (or the APK) as its
#                assets. Served at `tauri://localhost` on desktop and
#                `http://tauri.localhost` on Android.
#
# This is the only place mdbook runs. It renders the book from the repository
# root with the repository's own book.toml, exactly as CI does, so the
# preprocessors (note, listing and link-rewriter) are built by its `cargo run`
# commands and nothing has to be rewritten. Neither mdbook nor the preprocessors
# end up in the bundle: the app is a static reader.
#
# Tauri runs this through beforeDevCommand / beforeBuildCommand. Run it by hand
# before `cargo build` in src-tauri if you prefer, or after editing `src/` to
# see the change in a running `cargo tauri dev` (reload the window).

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
native="$(cd "$here/.." && pwd)"
root="$(cd "$native/.." && pwd)"

book_out="$native/ui"

# Find mdbook. It is a build tool here, not part of the app.
mdbook_bin="${MDBOOK_BIN:-}"
if [ -z "$mdbook_bin" ]; then
  for candidate in "$(command -v mdbook 2>/dev/null || true)" "$HOME/.cargo/bin/mdbook"; do
    if [ -n "$candidate" ] && [ -x "$candidate" ]; then
      mdbook_bin="$candidate"
      break
    fi
  done
fi
if [ -z "$mdbook_bin" ]; then
  echo "error: mdbook not found. Install it with:" >&2
  echo "  cargo install mdbook --version 0.5.1 --locked" >&2
  exit 1
fi

mdbook_version="$("$mdbook_bin" --version)"
case "$mdbook_version" in
  "mdbook v0.5."*) ;;
  *)
    echo "error: expected mdbook v0.5.x, found: $mdbook_version" >&2
    echo "The book's book.toml and preprocessors are pinned to the version CI uses (0.5.1)." >&2
    exit 1
    ;;
esac

echo "Rendering the book from $root"
rm -rf "$book_out"
mkdir -p "$(dirname "$book_out")"
# mdbook runs the preprocessor commands from the book root, so `cargo run
# --manifest-path packages/mdbook-trpl/...` in book.toml resolves there. The
# destination is absolute, so it is written straight to native/ui regardless of
# the book root.
"$mdbook_bin" build "$root" --dest-dir "$book_out"

# The app opens on index.html; the others are what the book links to and what a
# static host (or the webview) would look for. Fail loudly if any is missing so
# a broken render never ships as a blank app.
for required in index.html print.html 404.html; do
  if [ ! -f "$book_out/$required" ]; then
    echo "error: mdbook did not write $book_out/$required" >&2
    exit 1
  fi
done
if ! ls "$book_out"/searchindex*.js >/dev/null 2>&1; then
  echo "error: mdbook did not write a search index into $book_out" >&2
  exit 1
fi

echo
echo "Rendered: $book_out ($(du -sh "$book_out" | cut -f1))"
echo "  mdbook: $mdbook_version ($mdbook_bin)"
