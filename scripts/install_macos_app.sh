#!/bin/bash
set -euo pipefail
ACTION="${1:-}"
APP="$HOME/Applications/Rust Radio.app"
BUNDLE_ID='com.skeffington.rustradio'
SOURCE="${2:-/private/tmp/rust-radio-target/release/Rust Radio.app}"
[[ "$(uname -s)" == Darwin ]] || { echo 'macOS required' >&2; exit 1; }
read_id() { /usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$1/Contents/Info.plist" 2>/dev/null || true; }
check_bundle() {
  [[ -d "$1" ]] || { echo "App bundle missing: $1" >&2; exit 1; }
  [[ "$(read_id "$1")" == "$BUNDLE_ID" ]] || { echo "Refusing bundle with unexpected identifier: $1" >&2; exit 1; }
}
dock_edit() {
  local mode="$1" tmp="$HOME/Library/Caches/rust-radio-dock-$$.plist"
  mkdir -p "$HOME/Library/Caches"
  defaults export com.apple.dock "$tmp"
  local status
  status="$(python3 - "$tmp" "$APP" "$mode" <<'PY'
import pathlib,plistlib,sys,urllib.parse
p,path,mode=sys.argv[1:]
def normalize(s):
    u=urllib.parse.urlsplit(s)
    return urllib.parse.unquote(u.path).rstrip('/') if u.scheme=='file' else urllib.parse.unquote(s).rstrip('/')
with open(p,'rb') as f: d=plistlib.load(f)
apps=d.setdefault('persistent-apps',[])
def url(x):
    try: return x['tile-data']['file-data']['_CFURLString']
    except (KeyError,TypeError): return ''
matches=[i for i,x in enumerate(apps) if normalize(url(x))==str(pathlib.Path(path).resolve()).rstrip('/')]
if mode=='add':
    if len(matches)>1:
        for i in reversed(matches): apps.pop(i)
        apps.append({'tile-data':{'file-data':{'_CFURLString':pathlib.Path(path).as_uri(),'_CFURLStringType':15},'file-label':'Rust Radio'},'tile-type':'file-tile'})
    if not matches:
        apps.append({'tile-data':{'file-data':{'_CFURLString':pathlib.Path(path).as_uri(),'_CFURLStringType':15},'file-label':'Rust Radio'},'tile-type':'file-tile'})
elif mode=='remove':
    for i in reversed(matches): apps.pop(i)
else: raise SystemExit('bad mode')
with open(p,'wb') as f: plistlib.dump(d,f)
print('changed' if (mode=='add' and not matches) or (mode=='remove' and matches) or (mode=='add' and len(matches)>1) else 'unchanged')
PY
)" || { rm -f "$tmp"; return 1; }
  if [[ "$status" == changed ]]; then
    defaults import com.apple.dock "$tmp"
    killall Dock 2>/dev/null || true
  fi
  rm -f "$tmp"
}
STAGE=''
BACKUP=''
rollback_install() {
  local rc=$?
  if [[ -n "$STAGE" ]]; then rm -rf "$STAGE"; fi
  if [[ $rc -ne 0 && -n "$BACKUP" && -e "$BACKUP" ]]; then
    rm -rf "$APP"
    mv "$BACKUP" "$APP"
  fi
  return "$rc"
}
case "$ACTION" in
 install)
  check_bundle "$SOURCE"
  if [[ -e "$APP" ]]; then check_bundle "$APP"; fi
  running=0
  pgrep -x rust-radio-gui >/dev/null && running=1 || true
  mkdir -p "$HOME/Applications"
  STAGE="$HOME/Applications/.Rust Radio.app.$$"
  BACKUP="$HOME/Applications/.Rust Radio.app.backup.$$"
  trap rollback_install EXIT
  rm -rf "$STAGE" "$BACKUP"
  ditto "$SOURCE" "$STAGE"
  check_bundle "$STAGE"
  if [[ -e "$APP" ]]; then mv "$APP" "$BACKUP"; fi
  if ! mv "$STAGE" "$APP"; then
    if [[ -e "$BACKUP" ]]; then mv "$BACKUP" "$APP"; fi
    exit 1
  fi
  dock_edit add
  if [[ "$running" == 1 ]]; then
    echo 'The previous Rust Radio process is still running from its mapped executable; quit and relaunch to use the updated bundle.' >&2
  fi
  if [[ -e "$BACKUP" ]] && ! rm -rf "$BACKUP"; then
    echo "Installed successfully; old bundle backup remains at $BACKUP" >&2
  fi
  trap - EXIT
  echo "Installed $APP"
  ;;
 uninstall)
  if [[ -e "$APP" ]]; then check_bundle "$APP"; rm -rf "$APP"; fi
  dock_edit remove
  echo 'Rust Radio per-user install removed.'
  ;;
 *) echo "usage: $0 {install|uninstall} [built-app-path]" >&2; exit 2 ;;
esac
