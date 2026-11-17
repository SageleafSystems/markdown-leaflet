#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-or-later
#
# Builds "Markdown Leaflet.app" for macOS: a release binary, an .icns made from the brand icon,
# an Info.plist, and a code signature. Optionally wraps the bundle in a .dmg.
#
#   scripts/bundle-macos.sh           build and verify target/macos/Markdown Leaflet.app
#   scripts/bundle-macos.sh dmg       also create target/macos/Markdown-Leaflet-<version>.dmg
#
# Environment:
#   CODESIGN_IDENTITY   signing identity (e.g. "Developer ID Application: Name (TEAMID)").
#                       Unset or "-" signs ad hoc: fine to run locally, but Gatekeeper warns
#                       on other Macs. A real identity also enables the hardened runtime.
#   TARGET              Rust target triple to build for (e.g. aarch64-apple-darwin).
#   BUNDLE_ID           CFBundleIdentifier (default com.sageleafsystems.markdown-leaflet).
#
# Notarization is not done here; it needs an Apple Developer account. After a signed .dmg:
#   xcrun notarytool submit <dmg> --keychain-profile <profile> --wait
#   xcrun stapler staple <dmg>
set -euo pipefail

if [[ "$(uname -s)" != Darwin ]]; then
  printf 'This script builds a macOS app and must run on macOS.\n' >&2
  exit 1
fi

action=${1:-app}
case "$action" in
  app | dmg) ;;
  *)
    printf 'Usage: %s [app|dmg]\n' "$0" >&2
    exit 2
    ;;
esac

source_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
cd "$source_dir"

binary_name=markdown-leaflet
app_title="Markdown Leaflet"
bundle_id=${BUNDLE_ID:-com.sageleafsystems.markdown-leaflet}
identity=${CODESIGN_IDENTITY:--}
icon_source=assets/brand/markdown-leaflet-icon.png
icon_16=assets/brand/markdown-leaflet-icon-16.png
icon_32=assets/brand/markdown-leaflet-icon-32.png
min_macos=11.0

version=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)
if [[ -z $version ]]; then
  printf 'Could not read the version from Cargo.toml.\n' >&2
  exit 1
fi
build_number=$(git rev-list --count HEAD 2>/dev/null || printf '1')

out_dir="$source_dir/target/macos"
app="$out_dir/$app_title.app"
contents="$app/Contents"

cargo_args=(build --release --locked)
release_dir=target/release
if [[ -n ${TARGET:-} ]]; then
  cargo_args+=(--target "$TARGET")
  release_dir="target/$TARGET/release"
fi
printf 'Building %s %s (%s)\n' "$app_title" "$version" "$build_number"
cargo "${cargo_args[@]}"

rm -rf "$app"
mkdir -p "$contents/MacOS" "$contents/Resources"
cp "$release_dir/$binary_name" "$contents/MacOS/$binary_name"

work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT

# --- Icon ---------------------------------------------------------------------------------------
# The brand PNG is a 1024 px canvas with its own margin and shadow, as macOS app icons are drawn,
# so it is used as the master unchanged.
iconset="$work/AppIcon.iconset"
mkdir "$iconset"
master="$icon_source"
for size in 16 32 128 256 512; do
  sips -z "$size" "$size" "$master" --out "$iconset/icon_${size}x${size}.png" >/dev/null
  double=$((size * 2))
  sips -z "$double" "$double" "$master" --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
done
# The 16 and 32 px entries (and 16@2x) use the simplified icon, which stays legible at that size.
cp "$icon_16" "$iconset/icon_16x16.png"
cp "$icon_32" "$iconset/icon_16x16@2x.png"
cp "$icon_32" "$iconset/icon_32x32.png"
iconutil -c icns "$iconset" -o "$contents/Resources/AppIcon.icns"

# --- Info.plist ---------------------------------------------------------------------------------
year=$(date +%Y)
cat >"$contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key><string>en</string>
  <key>CFBundleExecutable</key><string>$binary_name</string>
  <key>CFBundleIconFile</key><string>AppIcon</string>
  <key>CFBundleIdentifier</key><string>$bundle_id</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>CFBundleName</key><string>$app_title</string>
  <key>CFBundleDisplayName</key><string>$app_title</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundleVersion</key><string>$build_number</string>
  <key>LSMinimumSystemVersion</key><string>$min_macos</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.productivity</string>
  <key>CFBundleDocumentTypes</key>
  <array>
    <dict>
      <key>CFBundleTypeName</key><string>Markdown document</string>
      <key>CFBundleTypeRole</key><string>Viewer</string>
      <key>LSHandlerRank</key><string>Default</string>
      <key>LSItemContentTypes</key>
      <array><string>public.markdown</string><string>net.daringfireball.markdown</string></array>
      <key>CFBundleTypeExtensions</key>
      <array><string>md</string><string>markdown</string><string>mdown</string><string>mdx</string></array>
    </dict>
    <dict>
      <key>CFBundleTypeName</key><string>Folder</string>
      <key>CFBundleTypeRole</key><string>Viewer</string>
      <key>LSHandlerRank</key><string>Alternate</string>
      <key>LSItemContentTypes</key>
      <array><string>public.folder</string></array>
    </dict>
  </array>
  <key>UTImportedTypeDeclarations</key>
  <array>
    <dict>
      <key>UTTypeIdentifier</key><string>net.daringfireball.markdown</string>
      <key>UTTypeDescription</key><string>Markdown document</string>
      <key>UTTypeConformsTo</key>
      <array><string>public.plain-text</string></array>
      <key>UTTypeTagSpecification</key>
      <dict>
        <key>public.filename-extension</key>
        <array><string>md</string><string>markdown</string><string>mdown</string><string>mdx</string></array>
        <key>public.mime-type</key>
        <array><string>text/markdown</string></array>
      </dict>
    </dict>
  </array>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSSupportsAutomaticGraphicsSwitching</key><true/>
  <key>NSHumanReadableCopyright</key><string>Copyright © $year Sageleaf Systems. GPL-3.0-or-later.</string>
</dict>
</plist>
PLIST
plutil -lint "$contents/Info.plist" >/dev/null

# --- Signature ----------------------------------------------------------------------------------
sign_args=(--force --sign "$identity")
if [[ $identity != - ]]; then
  sign_args+=(--options runtime --timestamp)
fi
codesign "${sign_args[@]}" "$app"

# --- Checks -------------------------------------------------------------------------------------
codesign --verify --strict "$app"
if [[ ! -x $contents/MacOS/$binary_name ]]; then
  printf 'Bundle is missing its executable.\n' >&2
  exit 1
fi
icns_size=$(stat -f %z "$contents/Resources/AppIcon.icns")
if ((icns_size < 10000)); then
  printf 'AppIcon.icns looks too small (%s bytes).\n' "$icns_size" >&2
  exit 1
fi
printf 'App bundle: %s\n' "$app"

# --- Disk image ---------------------------------------------------------------------------------
if [[ $action == dmg ]]; then
  dmg="$out_dir/Markdown-Leaflet-$version.dmg"
  stage="$work/dmg"
  mkdir "$stage"
  cp -R "$app" "$stage/"
  ln -s /Applications "$stage/Applications"
  rm -f "$dmg"
  hdiutil create -quiet -volname "$app_title" -srcfolder "$stage" -ov -format UDZO "$dmg"
  if [[ $identity != - ]]; then
    codesign --force --sign "$identity" --timestamp "$dmg"
  fi
  hdiutil verify -quiet "$dmg"
  printf 'Disk image: %s\n' "$dmg"
fi
