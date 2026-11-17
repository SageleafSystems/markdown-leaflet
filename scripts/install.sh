#!/usr/bin/env bash
set -euo pipefail

app_name=markdown-leaflet
source_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)
user_prefix=${PREFIX:-"${HOME:?HOME must be set}/.local"}
system_prefix=${PREFIX:-/usr/local}

# Set by set_prefix for the action in hand: a user-local install needs no
# privilege, a system-wide one (global/global-remove) needs sudo for every
# write under $prefix — but never for the `cargo build` below, which always
# runs as the invoking user.
prefix=
bin_dir=
installed_binary=
icon_svg=
icon_png=
icon_16=
icon_32=
desktop_file=
sudo_cmd=()

set_prefix() {
  if [[ "$1" == global* ]]; then
    prefix=$system_prefix
    sudo_cmd=(sudo)
  else
    prefix=$user_prefix
    sudo_cmd=()
  fi
  bin_dir="$prefix/bin"
  installed_binary="$bin_dir/$app_name"
  icon_svg="$prefix/share/icons/hicolor/scalable/apps/$app_name.svg"
  icon_png="$prefix/share/icons/hicolor/1024x1024/apps/$app_name.png"
  icon_16="$prefix/share/icons/hicolor/16x16/apps/$app_name.png"
  icon_32="$prefix/share/icons/hicolor/32x32/apps/$app_name.png"
  desktop_file="$prefix/share/applications/$app_name.desktop"
}

# Linux desktop environments (Wayland compositors especially — winit has no
# window-icon API there) resolve the taskbar icon from an installed
# `.desktop` file, not from the running window. macOS instead needs a
# proper `.app` bundle, which this script does not build yet.
refresh_desktop_caches() {
  if command -v update-desktop-database >/dev/null 2>&1; then
    "${sudo_cmd[@]}" update-desktop-database "$prefix/share/applications" 2>/dev/null || true
  fi
  if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    "${sudo_cmd[@]}" gtk-update-icon-cache -f -t "$prefix/share/icons/hicolor" 2>/dev/null || true
  fi
}

cleanup_temp_binary() {
  if [[ -n "${temporary_binary:-}" ]]; then
    "${sudo_cmd[@]}" rm -f -- "$temporary_binary"
  fi
}

do_install() {
  if [[ -e "$installed_binary" && ! -f "$installed_binary" ]]; then
    printf 'Install target exists and is not a regular file: %s\n' "$installed_binary" >&2
    exit 1
  fi

  cargo build --release --locked --manifest-path "$source_dir/Cargo.toml"
  "${sudo_cmd[@]}" mkdir -p -- "$bin_dir"

  temporary_binary=$("${sudo_cmd[@]}" mktemp "$bin_dir/.$app_name.XXXXXXXX")
  trap cleanup_temp_binary EXIT
  "${sudo_cmd[@]}" install -m 755 "$source_dir/target/release/$app_name" "$temporary_binary"
  "${sudo_cmd[@]}" mv -f -- "$temporary_binary" "$installed_binary"
  trap - EXIT

  printf 'Installed %s\n' "$installed_binary"
  case ":$PATH:" in
    *":$bin_dir:"*) ;;
    *) printf 'Add %s to PATH to run it by name.\n' "$bin_dir" ;;
  esac

  if [[ "$(uname -s)" == Linux ]]; then
    "${sudo_cmd[@]}" mkdir -p -- "$(dirname -- "$icon_svg")" "$(dirname -- "$icon_png")" "$(dirname -- "$icon_16")" "$(dirname -- "$icon_32")" "$(dirname -- "$desktop_file")"
    "${sudo_cmd[@]}" install -m 644 "$source_dir/assets/brand/markdown-leaflet-icon.svg" "$icon_svg"
    "${sudo_cmd[@]}" install -m 644 "$source_dir/assets/brand/markdown-leaflet-icon.png" "$icon_png"
    # Small sizes use the simplified icon, which stays legible where the full artwork would blur.
    "${sudo_cmd[@]}" install -m 644 "$source_dir/assets/brand/markdown-leaflet-icon-16.png" "$icon_16"
    "${sudo_cmd[@]}" install -m 644 "$source_dir/assets/brand/markdown-leaflet-icon-32.png" "$icon_32"
    "${sudo_cmd[@]}" install -m 644 "$source_dir/assets/linux/$app_name.desktop" "$desktop_file"
    refresh_desktop_caches
    printf 'Installed desktop icon and launcher entry\n'
  fi
}

do_remove() {
  if [[ -e "$installed_binary" || -L "$installed_binary" ]]; then
    "${sudo_cmd[@]}" rm -f -- "$installed_binary"
    printf 'Removed %s\n' "$installed_binary"
  else
    printf 'No installed binary at %s\n' "$installed_binary"
  fi

  if [[ "$(uname -s)" == Linux ]]; then
    "${sudo_cmd[@]}" rm -f -- "$icon_svg" "$icon_png" "$icon_16" "$icon_32" "$desktop_file"
    refresh_desktop_caches
  fi
}

usage() {
  cat <<EOF
Usage: $0 [install|remove|uninstall|global|global-remove|global-uninstall]

install (default)  Install for the current user only, into $user_prefix.
                    Set PREFIX to choose another prefix; use the same
                    PREFIX again when removing.
remove, uninstall   Remove a user install.
global              Install system-wide, for every user on this
                    machine, into /usr/local by default (set PREFIX
                    to choose another prefix). Runs the build as you,
                    then uses sudo to write under the prefix — it
                    will prompt for your password.
global-remove,
global-uninstall    Remove a system-wide install.

Either way, ensure the chosen bin/ directory is on PATH to launch
$app_name by name.
EOF
}

action=${1:-install}
if [[ $# -gt 1 ]]; then
  usage >&2
  exit 2
fi

case "$action" in
  install|global)
    set_prefix "$action"
    do_install
    ;;
  remove|uninstall|global-remove|global-uninstall)
    set_prefix "$action"
    do_remove
    ;;
  -h|--help|help)
    usage
    ;;
  *)
    usage >&2
    exit 2
    ;;
esac
