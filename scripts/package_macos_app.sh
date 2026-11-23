#!/bin/bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET_DIR="${CARGO_TARGET_DIR:-/private/tmp/rust-radio-target}"
APP="$TARGET_DIR/release/Rust Radio.app"
STAGE="$TARGET_DIR/release/.Rust Radio.app.stage.$$"
BACKUP="$TARGET_DIR/release/.Rust Radio.app.backup.$$"
ICONSET="$TARGET_DIR/macos-icon.iconset"
mkdir -p "$TARGET_DIR"
make_icon() {
  command -v rsvg-convert >/dev/null || { echo 'rsvg-convert is required' >&2; exit 1; }
  command -v sips >/dev/null || { echo 'sips is required' >&2; exit 1; }
  command -v iconutil >/dev/null || { echo 'iconutil is required' >&2; exit 1; }
  local png="$TARGET_DIR/RustRadio-1024.png"
  rsvg-convert -w 1024 -h 1024 "$ROOT/assets/macos/RustRadio.svg" -o "$png"
  rm -rf "$ICONSET"; mkdir -p "$ICONSET"
  for spec in '16 16' '32 16@2x' '32 32' '64 32@2x' '128 128' '256 128@2x' '256 256' '512 256@2x' '512 512' '1024 512@2x'; do
    set -- $spec; sips -z "$1" "$1" "$png" --out "$ICONSET/icon_$2.png" >/dev/null
  done
  iconutil -c icns "$ICONSET" -o "$ROOT/assets/macos/RustRadio.icns"
}
case "${1:-app}" in
 icon) make_icon; exit 0 ;;
 app) ;;
 *) echo "usage: $0 [app|icon]" >&2; exit 2 ;;
esac
[[ -f "$ROOT/assets/macos/RustRadio.icns" ]] || { echo "Missing committed icon; run $0 icon first." >&2; exit 1; }
CARGO_TARGET_DIR="$TARGET_DIR" cargo build --release -p rust-radio-gui --bin rust-radio-gui
if [[ -e "$APP" ]]; then
  current_id=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$APP/Contents/Info.plist" 2>/dev/null || true)
  [[ "$current_id" == com.skeffington.rustradio ]] || { echo "Refusing to replace app with unexpected or missing bundle ID: $APP" >&2; exit 1; }
fi
cleanup() {
  rm -rf "$STAGE"
  if [[ -e "$BACKUP" && ! -e "$APP" ]]; then mv "$BACKUP" "$APP"; fi
}
trap cleanup EXIT
rm -rf "$STAGE" "$BACKUP"
mkdir -p "$STAGE/Contents/MacOS" "$STAGE/Contents/Resources"
install -m 755 "$TARGET_DIR/release/rust-radio-gui" "$STAGE/Contents/MacOS/rust-radio-gui"
install -m 644 "$ROOT/assets/macos/RustRadio.icns" "$STAGE/Contents/Resources/RustRadio.icns"
cp "$ROOT/assets/macos/Info.plist.in" "$STAGE/Contents/Info.plist"
/usr/libexec/PlistBuddy -c 'Set :CFBundleShortVersionString 0.1.0' "$STAGE/Contents/Info.plist"
plutil -lint "$STAGE/Contents/Info.plist"
[[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$STAGE/Contents/Info.plist")" == com.skeffington.rustradio ]]
[[ -x "$STAGE/Contents/MacOS/rust-radio-gui" && -s "$STAGE/Contents/Resources/RustRadio.icns" ]]
if [[ -e "$APP" ]]; then mv "$APP" "$BACKUP"; fi
mv "$STAGE" "$APP"
if [[ -e "$BACKUP" ]] && ! rm -rf "$BACKUP"; then
  echo "Packaged successfully; prior bundle backup remains at $BACKUP" >&2
fi
trap - EXIT
echo "Packaged: $APP"
echo 'Unsigned and unnotarized local build; distribution requires the separate signing/notarization release gate.'
