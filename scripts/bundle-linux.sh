#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Builds a portable Linux AppImage: a release binary, an AppDir with the freedesktop.org
# desktop entry and hicolor icon set, and the AppImage itself.
#
#   scripts/bundle-linux.sh    build and verify target/linux/markdown-leaflet-<version>-linux-x86_64.AppImage
#
# Environment:
#   TARGET           Rust target triple to build for (e.g. x86_64-unknown-linux-gnu).
#   APPIMAGETOOL     Path to an existing appimagetool binary. If unset, a pinned release is
#                     downloaded into target/linux/tools/ and cached there for later runs.
set -euo pipefail

if [[ "$(uname -s)" != Linux ]]; then
  printf 'This script builds a Linux AppImage and must run on Linux.\n' >&2
  exit 1
fi

source_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
cd "$source_dir"

binary_name=markdown-leaflet
app_name="Markdown Leaflet"
icon_master=assets/brand/markdown-leaflet-icon.png
icon_small_16=assets/brand/markdown-leaflet-icon-16.png
icon_small_32=assets/brand/markdown-leaflet-icon-32.png
icon_svg=assets/brand/markdown-leaflet-icon.svg
desktop_src=assets/linux/$binary_name.desktop

# Pinned to a known-good release; bump deliberately and re-verify the AppImage still builds.
appimagetool_version=13
appimagetool_url="https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage"

version=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)
if [[ -z $version ]]; then
  printf 'Could not read the version from Cargo.toml.\n' >&2
  exit 1
fi

out_dir="$source_dir/target/linux"
tools_dir="$out_dir/tools"
appdir="$out_dir/AppDir"
artifact="$out_dir/$binary_name-$version-linux-x86_64.AppImage"

cargo_args=(build --release --locked)
release_dir=target/release
if [[ -n ${TARGET:-} ]]; then
  cargo_args+=(--target "$TARGET")
  release_dir="target/$TARGET/release"
fi
printf 'Building %s %s\n' "$app_name" "$version"
cargo "${cargo_args[@]}"

if [[ ! -x "$release_dir/$binary_name" ]]; then
  printf 'Release binary not found at %s/%s\n' "$release_dir" "$binary_name" >&2
  exit 1
fi

# --- AppDir ---------------------------------------------------------------------------------
rm -rf "$appdir"
mkdir -p "$appdir/usr/bin" "$appdir/usr/share/applications"
cp "$release_dir/$binary_name" "$appdir/usr/bin/$binary_name"

# Full-size icons use the brand master; the 16/32 px entries use the simplified icon, which
# stays legible at that size (same split scripts/bundle-macos.sh uses for its .iconset).
declare -A icon_sources=(
  [16]=$icon_small_16
  [32]=$icon_small_32
  [48]=$icon_master
  [64]=$icon_master
  [128]=$icon_master
  [256]=$icon_master
)
for size in "${!icon_sources[@]}"; do
  icon_dir="$appdir/usr/share/icons/hicolor/${size}x${size}/apps"
  mkdir -p "$icon_dir"
  magick "${icon_sources[$size]}" -resize "${size}x${size}" "$icon_dir/$binary_name.png"
done
icon_scalable_dir="$appdir/usr/share/icons/hicolor/scalable/apps"
mkdir -p "$icon_scalable_dir"
cp "$icon_svg" "$icon_scalable_dir/$binary_name.svg"

# Installed desktop entry: Exec relies on the AppDir's usr/bin being on PATH when launched
# through the AppImage runtime, which prepends it there.
cp "$desktop_src" "$appdir/usr/share/applications/$binary_name.desktop"
desktop-file-validate "$appdir/usr/share/applications/$binary_name.desktop"

# AppDir root: appimagetool and desktop file managers expect AppRun, a root .desktop, and a
# root icon with the same basename as the Icon= key. File managers that show a thumbnail for
# the .AppImage file itself (e.g. Nautilus) read .DirIcon instead, which must be a raster image
# at that same path, not a symlink or an SVG, or they fall back to a generic icon.
ln -sf "usr/bin/$binary_name" "$appdir/AppRun"
cp "$desktop_src" "$appdir/$binary_name.desktop"
cp "$appdir/usr/share/icons/hicolor/256x256/apps/$binary_name.png" "$appdir/$binary_name.png"
cp "$appdir/$binary_name.png" "$appdir/.DirIcon"

# --- appimagetool -----------------------------------------------------------------------------
appimagetool=${APPIMAGETOOL:-}
if [[ -z $appimagetool ]]; then
  mkdir -p "$tools_dir"
  appimagetool="$tools_dir/appimagetool-v$appimagetool_version"
  if [[ ! -x $appimagetool ]]; then
    printf 'Downloading appimagetool (continuous release)\n'
    curl -fsSL -o "$appimagetool" "$appimagetool_url"
    chmod +x "$appimagetool"
  fi
fi

mkdir -p "$out_dir"
rm -f "$artifact"
ARCH=x86_64 "$appimagetool" "$appdir" "$artifact"

# --- Checks -------------------------------------------------------------------------------------
if [[ ! -f "$artifact" ]]; then
  printf 'AppImage was not produced at %s\n' "$artifact" >&2
  exit 1
fi
chmod +x "$artifact"
artifact_size=$(stat -c %s "$artifact")
if ((artifact_size < 1000000)); then
  printf 'AppImage looks too small (%s bytes).\n' "$artifact_size" >&2
  exit 1
fi
sha256sum "$artifact" >"$artifact.sha256"
printf 'AppImage: %s\n' "$artifact"
printf 'Checksum: %s.sha256\n' "$artifact"
