#!/usr/bin/env bash
#
# Stage everything the desktop app needs to run mdbook at launch:
#
#   src-tauri/resources/book-source/   the book source, with book.toml pointing
#                                      at the preprocessors that ship beside it
#   src-tauri/resources/bin/           mdbook and the two preprocessors
#
# Nothing here renders the book. `mdbook build` is never run at build time; the
# app runs `mdbook serve` when it starts.
#
# The binaries are bundled as resources rather than `externalBin` sidecars on
# purpose: Tauri installs sidecars into /usr/bin next to the app binary, and a
# package called `mdbook` in /usr/bin would collide with a real mdbook install.
# As resources they land in /usr/lib/<product>/ and are copied to the app cache
# at launch, where they are made executable.
#
# Tauri runs this through beforeDevCommand / beforeBuildCommand. Run it by hand
# before `cargo build` in src-tauri if you prefer.

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
native="$(cd "$here/.." && pwd)"
root="$(cd "$native/.." && pwd)"

resources="$native/src-tauri/resources"
book_out="$resources/book-source"
bin_out="$resources/bin"

# Find mdbook. It is not part of the app's Cargo dependencies; it is a sidecar
# built or installed once and copied into the bundle.
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

echo "Staging book source from $root"
rm -rf "$book_out"
mkdir -p "$book_out"
cp -R "$root/src" "$book_out/src"
cp -R "$root/listings" "$book_out/listings"
cp -R "$root/theme" "$book_out/theme"
cp "$root/ferris.css" "$root/ferris.js" "$book_out/"

# The repository's book.toml runs the preprocessors with `cargo run`. The
# packaged app has no cargo and no packages/ directory, so point the commands at
# the binaries the app copies into <book>/bin at launch. mdbook runs
# preprocessors with the book root as cwd, so the relative paths work.
sed \
  -e 's|^command = "cargo run --manifest-path packages/mdbook-trpl/Cargo.toml --bin mdbook-trpl-note"$|command = "bin/mdbook-trpl-note"|' \
  -e 's|^command = "cargo run --manifest-path packages/mdbook-trpl/Cargo.toml --bin mdbook-trpl-listing"$|command = "bin/mdbook-trpl-listing"|' \
  -e '/^extra-watch-dirs = \["packages\/mdbook-trpl"\]$/d' \
  "$root/book.toml" >"$book_out/book.toml"

if ! grep -q '^command = "bin/mdbook-trpl-note"$' "$book_out/book.toml" ||
  ! grep -q '^command = "bin/mdbook-trpl-listing"$' "$book_out/book.toml"; then
  echo "error: could not rewrite the preprocessor commands in book.toml" >&2
  echo "The preprocessor lines changed; update native/scripts/stage.sh." >&2
  exit 1
fi

echo "Building the book preprocessors"
# Always release: the binaries are copied into every bundle, and a debug build
# of the preprocessors would need its own full compile and be far larger.
cargo build --release --manifest-path "$root/packages/mdbook-trpl/Cargo.toml" \
  --bin mdbook-trpl-note --bin mdbook-trpl-listing
pre_dir="$root/packages/mdbook-trpl/target/release"

echo "Staging binaries"
rm -rf "$bin_out"
mkdir -p "$bin_out"

copy_bin() {
  local source="$1"
  local name="$2"
  local dest="$bin_out/$name"
  if [ ! -f "$source" ]; then
    echo "error: expected $source to exist" >&2
    exit 1
  fi
  cp "$source" "$dest"
  chmod +x "$dest"
}

copy_bin "$mdbook_bin" mdbook
copy_bin "$pre_dir/mdbook-trpl-note" mdbook-trpl-note
copy_bin "$pre_dir/mdbook-trpl-listing" mdbook-trpl-listing

echo
echo "Staged:"
echo "  $book_out"
echo "  $bin_out"
echo "  mdbook: $mdbook_version ($mdbook_bin)"
