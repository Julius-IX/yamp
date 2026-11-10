#!/usr/bin/env bash
# Release build for yamp. Builds the three distributed artifacts, an AppImage, an NSIS
# setup exe and a universal APK, and copies them to one folder under their release names:
#
#   yamp_v[version]_[target]_[descriptor]_[extra].[ext]
#   yamp_v1.0.1_Linux-x86-64_alpha.AppImage
#   yamp_v1.0.1_Windows_x86-64_alpha_setup.exe
#   yamp_v1.0.1_Android_universal_alpha.apk
#
# The folder is ../builds/v[version]-[descriptor]/ next to the repo, for example ../builds/v1.0.1-alpha/.
# The version comes from src-tauri/tauri.conf.json.
#
# One command for a full release:   scripts/build.sh --release -d alpha -y
#
# With the updater on, the desktop updater reads latest.json from the latest GitHub release, so the
# script also writes that file next to the artifacts. Upload everything it lists at the end to one
# release. The Android app only looks for the first .apk asset of the latest release.
#
# Before building, everything that is missing is listed in one place: the Discord id, the cloud
# client ids of every platform being built, the updater signing key when the updater is on, and
# any target the host cannot build. The ids are looked up the way the build.rs of cloud-auth and
# yamp-discord do it: the environment, then the config.toml of the crate, then
# src-tauri/.cargo/config.toml.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 1

CONF="src-tauri/tauri.conf.json"
ANDROID_DIR="src-tauri/gen/android"
CARGO_CONF="src-tauri/.cargo/config.toml"
DISCORD_CONF="src-tauri/crates/yamp_discord/config.toml"
CLOUD_CONF="src-tauri/crates/cloud_auth/config.toml"
BUILDS_ROOT="${YAMP_BUILDS_DIR:-$(cd "$ROOT/.." && pwd)/builds}"
OUT="${YAMP_OUT_DIR:-}"
SIGN_KEY_FILE="${YAMP_SIGNING_KEY:-$HOME/.tauri/yamp.key}"
ANDROID_TARGETS=(aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android)

TARGETS=""
DESCRIPTOR=""
DESC_GIVEN=0
ASSUME_YES=0
DRY=0
NO_UPDATER=0
TAG=""
NOTES=""
ALL_REQUESTED=0
UNSIGNED=0
SIGN_MISSING=0
BUILT=()
FAILED=()
WARNINGS=()

if [[ -t 1 ]]; then
  B=$'\033[1m' G=$'\033[32m' Y=$'\033[33m' R=$'\033[31m' D=$'\033[2m' N=$'\033[0m'
else
  B="" G="" Y="" R="" D="" N=""
fi

say() { printf '%s\n' "$*"; }
step() { printf '\n%s==> %s%s\n' "$B" "$*" "$N"; }
warn() { printf '%swarning:%s %s\n' "$Y" "$N" "$*" >&2; }
die() { printf '%serror:%s %s\n' "$R" "$N" "$*" >&2; exit 1; }

usage() {
  cat <<USAGE
Usage: scripts/build.sh [options]

  -r, --release             build every target for a release, same as --targets all
  -t, --targets <list>      linux, windows, android or all, comma separated
  -d, --descriptor <text>   release tag in the folder and file names, such as alpha. May be empty
  -o, --out <dir>           where the artifacts go, default ../builds/v<version>[-<descriptor>]
  -y, --yes                 do not ask for confirmation
      --tag <tag>           GitHub release the files are uploaded to, default v<version>[-<descriptor>]
      --notes <text>        release notes shown by the updater, goes into latest.json
      --no-updater          with the updater on, build the desktop targets without update bundles
  -n, --dry-run             show the commands and the file names without building
  -h, --help                this text

Anything not given as a flag is asked for. The environment is read as well:
  YAMP_*_CLIENT_ID, YAMP_*_CLIENT_SECRET, YAMP_DISCORD_APP_ID   win over the config files
  TAURI_SIGNING_PRIVATE_KEY[_PASSWORD]  signs the update bundles, only used when the updater is on
  YAMP_SIGNING_KEY                      path of the key file, default ~/.tauri/yamp.key
  YAMP_BUILDS_DIR                       parent of the version folders, default ../builds
  YAMP_OUT_DIR                          exact output folder, overrides the two above
  SQLX_OFFLINE                          defaults to true, the committed .sqlx data is used
USAGE
}

needval() { [[ $2 -ge 2 ]] || die "$1 needs a value"; }

while [[ $# -gt 0 ]]; do
  case "$1" in
    -r|--release) TARGETS=all; shift ;;
    -t|--targets) needval "$1" $#; TARGETS="$2"; shift 2 ;;
    -d|--descriptor) needval "$1" $#; DESCRIPTOR="$2"; DESC_GIVEN=1; shift 2 ;;
    -o|--out) needval "$1" $#; OUT="$2"; shift 2 ;;
    -y|--yes) ASSUME_YES=1; shift ;;
    --tag) needval "$1" $#; TAG="$2"; shift 2 ;;
    --notes) needval "$1" $#; NOTES="$2"; shift 2 ;;
    --no-updater) NO_UPDATER=1; shift ;;
    -n|--dry-run) DRY=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) usage >&2; die "unknown option $1" ;;
  esac
done

# SECTION: prompts

INTERACTIVE=0
[[ -t 0 ]] && INTERACTIVE=1

# Prints the answer. Without a terminal the default is the answer.
ask() {
  local reply
  if [[ $INTERACTIVE -eq 0 ]]; then echo "$2"; return; fi
  read -r -p "$1" reply </dev/tty
  echo "${reply:-$2}"
}

confirm() {
  [[ $ASSUME_YES -eq 1 ]] && return 0
  [[ $INTERACTIVE -eq 0 ]] && return 1
  local reply
  read -r -p "$1 [y/N] " reply </dev/tty
  [[ "$reply" =~ ^[Yy] ]]
}

# SECTION: host and project

case "$(uname -s)" in
  Linux*) HOST=linux ;;
  Darwin*) HOST=macos ;;
  MINGW*|MSYS*|CYGWIN*) HOST=windows ;;
  *) HOST=other ;;
esac

command -v node >/dev/null || die "node is not installed"
command -v npm >/dev/null || die "npm is not installed"
command -v cargo >/dev/null || die "cargo is not installed"

# Reads a value of tauri.conf.json, a dotted path, and prints it. Nothing when it is not there.
conf_get() {
  node -p "
    let v = JSON.parse(require('fs').readFileSync('$CONF', 'utf8'));
    for (const k of '$1'.split('.')) v = v == null ? undefined : v[k];
    v === undefined || v === null || v === false ? '' : (typeof v === 'object' ? 'object' : String(v));
  " 2>/dev/null
}

VERSION="$(conf_get version)"
[[ -n "$VERSION" ]] || die "cannot read the version from $CONF"
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+([.-][0-9A-Za-z.-]+)?$ ]] || die "version \"$VERSION\" in $CONF is not a dotted version"

# The three only have to agree with each other.
PKG_VERSION="$(node -p "require('./package.json').version" 2>/dev/null)"
CARGO_VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' src-tauri/Cargo.toml | head -n1)"

# The Tauri updater is only part of a build when the config asks for update bundles. The yt-dlp
# updater of the yt-api crate is something else and needs nothing here.
UPDATER_ON=0
[[ -n "$(conf_get bundle.createUpdaterArtifacts)" ]] && UPDATER_ON=1

# The updater reads latest.json from the endpoint. It is a release asset when the endpoint is
# https://github.com/<owner>/<repo>/releases/latest/download/latest.json, and the repo is taken from it.
ENDPOINT="$(node -p "(((JSON.parse(require('fs').readFileSync('$CONF','utf8')).plugins||{}).updater||{}).endpoints||[])[0]||''" 2>/dev/null)"
RELEASE_REPO=""
if [[ "$ENDPOINT" =~ ^https://github\.com/([^/]+/[^/]+)/releases/latest/download/latest\.json$ ]]; then
  RELEASE_REPO="${BASH_REMATCH[1]}"
fi

# SECTION: reading the ids

# The value of a key in a config file, read the way the build.rs files of the crates do it: only
# the KEY = "value" lines of the [env] table count, a value has to be quoted and an empty one is
# not a value. With flat set the keys above any table header count too.
cfg_file_value() { # file name flat
  [[ -f "$1" ]] || return 0
  awk -v want="$2" -v flat="$3" '
    BEGIN { in_env = (flat == "1") }
    { gsub(/^[ \t]+|[ \t\r]+$/, "") }
    $0 == "" || substr($0, 1, 1) == "#" { next }
    substr($0, 1, 1) == "[" { in_env = ($0 == "[env]"); next }
    {
      i = index($0, "="); if (!i) next
      key = substr($0, 1, i - 1); val = substr($0, i + 1)
      gsub(/^[ \t]+|[ \t]+$/, "", key); gsub(/^[ \t]+|[ \t]+$/, "", val)
      q = substr(val, 1, 1)
      if (q != "\"" && q != "\047") next
      rest = substr(val, 2); j = index(rest, q); if (!j) next
      val = substr(rest, 1, j - 1)
      if (in_env && key == want && val != "") { print val; exit }
    }
  ' "$1"
}

# Prints the value of a name and succeeds when there is one. The first source with a value wins:
# the environment, the config.toml of the crate, the [env] table of src-tauri/.cargo/config.toml.
cfg_get() { # name crate-config
  local name="$1" v="${!1:-}"
  [[ -n "$v" ]] || v="$(cfg_file_value "$2" "$name" 1)"
  [[ -n "$v" ]] || v="$(cfg_file_value "$CARGO_CONF" "$name" 0)"
  [[ -n "$v" ]] && echo "$v"
}

# SECTION: what this host can build

# Each function prints the reason a target cannot be built here, and nothing when it can.

why_linux() {
  [[ "$HOST" == linux && "$(uname -m)" == x86_64 ]] || echo "an AppImage needs an x86-64 Linux host"
}

why_windows() {
  [[ "$HOST" == windows ]] && return
  local missing=() c
  for c in cargo-xwin makensis clang; do command -v "$c" >/dev/null || missing+=("$c"); done
  command -v lld >/dev/null || command -v ld.lld >/dev/null || missing+=(lld)
  [[ ${#missing[@]} -eq 0 ]] || echo "cross building from $HOST needs ${missing[*]}"
}

android_sdk() { echo "${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"; }

why_android() {
  local sdk; sdk="$(android_sdk)"
  [[ -n "$sdk" && -d "$sdk" ]] || { echo "ANDROID_HOME is not set"; return; }
  # the newest NDK of the SDK is used when NDK_HOME is not set
  if [[ -z "${NDK_HOME:-}" && ! -d "$sdk/ndk" ]]; then echo "NDK_HOME is not set and the SDK has no ndk folder"; return; fi
  [[ -n "${JAVA_HOME:-}" ]] || command -v java >/dev/null || { echo "JAVA_HOME is not set"; return; }
  [[ -d "$ANDROID_DIR" ]] || { echo "$ANDROID_DIR is missing, run tauri android init"; return; }
  # app/build.gradle.kts reads keyAlias, password and storeFile from this file for the release signing
  local props="$ANDROID_DIR/keystore.properties"
  [[ -f "$props" ]] || { echo "$props is missing, the release APK cannot be signed"; return; }
  local k
  for k in keyAlias password storeFile; do
    grep -q "^$k=" "$props" || { echo "$props has no $k"; return; }
  done
  local store; store="$(sed -n 's/^storeFile=//p' "$props" | head -n1)"
  # gradle resolves the path against the app module folder
  [[ -f "$store" || -f "$ANDROID_DIR/app/$store" ]] || echo "the storeFile of keystore.properties does not exist"
}

label() {
  case "$1" in
    linux) echo "Linux    AppImage        x86-64" ;;
    windows) echo "Windows  NSIS setup exe  x86-64" ;;
    android) echo "Android  universal APK" ;;
  esac
}

# SECTION: choosing

step "yamp v$VERSION on $HOST"

AVAILABLE=()
UNAVAILABLE=()   # entries are "target|reason"
n=0
for t in linux windows android; do
  n=$((n + 1))
  reason="$(why_"$t")"
  if [[ -z "$reason" ]]; then
    AVAILABLE+=("$t")
    [[ -n "$TARGETS" ]] || say "  $n) $(label "$t")"
  else
    UNAVAILABLE+=("$t|$reason")
    [[ -n "$TARGETS" ]] || say "  $n) ${D}$(label "$t")  unavailable, $reason${N}"
  fi
done
[[ ${#AVAILABLE[@]} -gt 0 ]] || die "nothing can be built on this host"

if [[ -z "$TARGETS" ]]; then
  say ""
  picked="$(ask "Targets, numbers separated by spaces or a for all [a]: " a)"
  if [[ "$picked" == a ]]; then
    TARGETS=all
  else
    for i in $picked; do
      case "$i" in 1) TARGETS+="linux," ;; 2) TARGETS+="windows," ;; 3) TARGETS+="android," ;; *) die "no target $i" ;; esac
    done
  fi
fi

SELECTED=()
for t in ${TARGETS//,/ }; do
  case "$t" in
    all) SELECTED=("${AVAILABLE[@]}"); ALL_REQUESTED=1; break ;;
    linux|windows|android)
      reason="$(why_"$t")"
      [[ -z "$reason" ]] || die "$t cannot be built here, $reason"
      SELECTED+=("$t") ;;
    *) die "unknown target $t" ;;
  esac
done
[[ ${#SELECTED[@]} -gt 0 ]] || die "no target selected"

if [[ $DESC_GIVEN -eq 0 ]]; then
  while true; do
    DESCRIPTOR="$(ask "Descriptor, such as alpha, empty for none []: " "")"
    # an underscore separates the parts of the name, so it cannot be part of one
    [[ -z "$DESCRIPTOR" || "$DESCRIPTOR" =~ ^[A-Za-z0-9.-]+$ ]] && break
    warn "use letters, digits, dots and dashes only"
    [[ $INTERACTIVE -eq 1 ]] || exit 1
  done
fi
[[ -z "$DESCRIPTOR" || "$DESCRIPTOR" =~ ^[A-Za-z0-9.-]+$ ]] || die "descriptor may only hold letters, digits, dots and dashes"

# ../builds/v1.0.1-alpha, or ../builds/v1.0.1 without a descriptor
[[ -n "$OUT" ]] || OUT="$BUILDS_ROOT/v${VERSION}${DESCRIPTOR:+-$DESCRIPTOR}"
[[ -n "$TAG" ]] || TAG="v${VERSION}${DESCRIPTOR:+-$DESCRIPTOR}"

# Name of a release file. The target is one piece, the descriptor and the extra are left out when empty.
artifact() { # target descriptor extra extension
  local name="yamp_v${VERSION}_$1"
  [[ -n "$2" ]] && name+="_$2"
  [[ -n "$3" ]] && name+="_$3"
  echo "$name.$4"
}

# SECTION: checks before building

add_warn() { WARNINGS+=("$*"); }

# The levels a build of a target looks at, most specific first, as cloud-auth's app.rs does.
levels_for() {
  case "$1" in
    linux) echo "LINUX DESKTOP" ;;
    windows) echo "WINDOWS DESKTOP" ;;
    android) echo "ANDROID MOBILE" ;;
  esac
}

# Google Drive and OneDrive for one target. A service without an id for the platform is left out of
# the build, which the app does silently, so it is reported here.
check_cloud() { # target
  local target="$1" svc lv found levels first second
  read -r first second <<<"$(levels_for "$target")"
  for svc in GOOGLE MICROSOFT; do
    found=""
    for lv in $first $second; do
      if cfg_get "YAMP_${svc}_${lv}_CLIENT_ID" "$CLOUD_CONF" >/dev/null; then found="$lv"; break; fi
    done
    if [[ -z "$found" ]]; then
      add_warn "$target: no $svc client id, set YAMP_${svc}_${first}_CLIENT_ID or YAMP_${svc}_${second}_CLIENT_ID, that cloud service is left out of the build"
    elif [[ "$svc" == GOOGLE && "$target" != android ]]; then
      # the desktop clients of Google come with a secret, the Android one has none
      cfg_get "YAMP_${svc}_${found}_CLIENT_SECRET" "$CLOUD_CONF" >/dev/null ||
        add_warn "$target: YAMP_GOOGLE_${found}_CLIENT_ID is set but YAMP_GOOGLE_${found}_CLIENT_SECRET is not"
    fi
  done
}

# Looks at bundle.icon of tauri.conf.json. Prints one problem per line, nothing when it is fine.
check_icons() {
  node -e '
    const fs = require("fs"), path = require("path");
    const conf = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
    const list = (conf.bundle || {}).icon || [];
    const dir = path.dirname(process.argv[1]);
    const png = (f) => { // width and height from the IHDR chunk
      const b = fs.readFileSync(f);
      return b.length > 24 && b.toString("latin1", 1, 4) === "PNG" ? [b.readUInt32BE(16), b.readUInt32BE(20)] : null;
    };
    if (!list.length) { console.log("bundle.icon is empty, the AppImage gets no icon"); process.exit(); }
    let largest = 0;
    for (const f of list) {
      const p = path.join(dir, f);
      if (!fs.existsSync(p)) { console.log(`bundle.icon lists ${f}, which does not exist`); continue; }
      if (f.endsWith(".png")) {
        const d = png(p);
        if (!d) console.log(`${f} is not a PNG`);
        else if (d[0] !== d[1]) console.log(`${f} is ${d[0]}x${d[1]}, an icon has to be square`);
        else largest = Math.max(largest, d[0]);
      }
    }
    if (!largest) console.log("bundle.icon has no PNG, the AppImage gets no icon");
    else if (largest < 512) {
      const big = path.join(dir, "icons/icon.png");
      const d = fs.existsSync(big) ? png(big) : null;
      console.log(`the largest PNG in bundle.icon is ${largest}x${largest}` +
        (d && d[0] >= 512 ? `, add icons/icon.png (${d[0]}x${d[1]}) to bundle.icon for a sharp icon in launchers` : ""));
    }
  ' "$CONF" 2>/dev/null
}

preflight() {
  local t desktop=0
  for t in "${SELECTED[@]}"; do [[ "$t" == android ]] || desktop=1; done

  # a full release that cannot be built in full
  if [[ $ALL_REQUESTED -eq 1 && ${#UNAVAILABLE[@]} -gt 0 ]]; then
    local e
    for e in "${UNAVAILABLE[@]}"; do add_warn "the release is incomplete, ${e%%|*} is skipped: ${e#*|}"; done
  fi

  [[ "$PKG_VERSION" == "$VERSION" ]] || add_warn "package.json says $PKG_VERSION, $CONF says $VERSION"
  [[ "$CARGO_VERSION" == "$VERSION" ]] || add_warn "src-tauri/Cargo.toml says $CARGO_VERSION, $CONF says $VERSION"

  for t in "${SELECTED[@]}"; do check_cloud "$t"; done

  # the Discord presence is compiled out of the mobile builds, so only the desktop ones need the id
  if [[ $desktop -eq 1 ]]; then
    cfg_get YAMP_DISCORD_APP_ID "$DISCORD_CONF" >/dev/null ||
      add_warn "no YAMP_DISCORD_APP_ID in the environment, $DISCORD_CONF or $CARGO_CONF, the desktop builds have no Discord presence"
  fi

  # the AppImage bundles GStreamer when this is on, and the build host has to have it
  if [[ " ${SELECTED[*]} " == *" linux "* && -n "$(conf_get bundle.linux.appimage.bundleMediaFramework)" ]]; then
    command -v pkg-config >/dev/null && pkg-config --exists gstreamer-1.0 ||
      add_warn "linux: bundle.linux.appimage.bundleMediaFramework is on and gstreamer-1.0 is not found by pkg-config, the AppImage build may fail"
  fi

  # the icons the bundlers get: every listed file has to exist, and a Linux build wants a big PNG
  if [[ " ${SELECTED[*]} " == *" linux "* ]]; then
    local line
    while IFS= read -r line; do add_warn "linux: $line"; done < <(check_icons)
  fi

  # the updater parts are only checked when the config asks for update bundles
  if [[ $UPDATER_ON -eq 1 && $desktop -eq 1 ]]; then
    [[ -n "$RELEASE_REPO" ]] ||
      add_warn "the updater endpoint \"$ENDPOINT\" is not a GitHub latest.json, so latest.json is not generated"
    [[ -n "$(conf_get plugins.updater.pubkey)" ]] || add_warn "plugins.updater.pubkey in $CONF is empty, the updater cannot verify an update"
    if [[ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" && ! -f "$SIGN_KEY_FILE" ]]; then
      SIGN_MISSING=1
      add_warn "no updater signing key, $SIGN_KEY_FILE does not exist and TAURI_SIGNING_PRIVATE_KEY is not set"
    fi
  fi
}

# Loads the signing key for the desktop builds. Without the key a build with update bundles fails, so
# a missing key is settled before the build starts: --no-updater or a yes to the question builds without them.
settle_signing() {
  [[ $UPDATER_ON -eq 1 ]] || return 0
  if [[ $SIGN_MISSING -eq 1 ]]; then
    if [[ $NO_UPDATER -eq 1 ]] || { [[ $ASSUME_YES -eq 0 ]] && confirm "Build the desktop targets without update bundles?"; }; then
      # TAURI_CONFIG is merged over tauri.conf.json, which avoids quoting a --config argument
      export TAURI_CONFIG='{"bundle":{"createUpdaterArtifacts":false}}'
      UNSIGNED=1
    elif [[ $DRY -eq 0 ]]; then
      die "set TAURI_SIGNING_PRIVATE_KEY or YAMP_SIGNING_KEY, or pass --no-updater"
    fi
    return
  fi
  [[ $DRY -eq 0 ]] || return 0
  local t desktop=0
  for t in "${SELECTED[@]}"; do [[ "$t" == android ]] || desktop=1; done
  [[ $desktop -eq 1 ]] || return 0
  if [[ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" ]]; then
    TAURI_SIGNING_PRIVATE_KEY="$(<"$SIGN_KEY_FILE")"
    export TAURI_SIGNING_PRIVATE_KEY
    if [[ -z "${TAURI_SIGNING_PRIVATE_KEY_PASSWORD+x}" ]]; then
      local pw=""
      if [[ $INTERACTIVE -eq 1 ]]; then read -r -s -p "Password of $SIGN_KEY_FILE, enter for none: " pw </dev/tty; echo; fi
      export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$pw"
    fi
    say "Signing update bundles with $SIGN_KEY_FILE"
  fi
}

# The committed .sqlx data is used, since a fresh checkout has no development database.
export SQLX_OFFLINE="${SQLX_OFFLINE:-true}"

# SECTION: summary

preflight
step "Checks"
if [[ ${#WARNINGS[@]} -eq 0 ]]; then
  say "${G}nothing is missing${N}"
else
  for w in "${WARNINGS[@]}"; do warn "$w"; done
fi
settle_signing

step "Plan"
say "  version      $VERSION"
say "  descriptor   ${DESCRIPTOR:-none}"
say "  out          $OUT"
say "  release tag  $TAG"
if [[ $UPDATER_ON -eq 0 ]]; then
  say "  updater      ${D}not configured in $CONF, no .sig files${N}"
elif [[ $UNSIGNED -eq 1 ]]; then
  say "  updater      ${Y}no update bundles${N}"
else
  say "  updater      signed update bundles"
fi
for t in "${SELECTED[@]}"; do
  case "$t" in
    linux) f="$(artifact Linux-x86-64 "$DESCRIPTOR" "" AppImage)" ;;
    windows) f="$(artifact Windows_x86-64 "$DESCRIPTOR" setup exe)" ;;
    android) f="$(artifact Android_universal "$DESCRIPTOR" "" apk)" ;;
  esac
  say "  $t   $f"
done
[[ $DRY -eq 1 ]] && say "${D}dry run, nothing is built${N}"
START_Q="Start?"
[[ ${#WARNINGS[@]} -eq 0 ]] || START_Q="Start despite ${#WARNINGS[@]} warning(s)?"
confirm "$START_Q" || { [[ $DRY -eq 1 ]] || die "cancelled"; }

# SECTION: building

run() {
  if [[ $DRY -eq 1 ]]; then say "${D}+ $*${N}"; return 0; fi
  "$@"
}

MARK="$(mktemp)"
trap 'rm -f "$MARK"' EXIT

if [[ $DRY -eq 0 && ! -d node_modules ]]; then
  step "Installing the frontend dependencies"
  npm ci || die "npm ci failed"
fi
[[ $DRY -eq 1 ]] || mkdir -p "$OUT"

# The newest file below src-tauri that matches, preferably one written by this run.
find_new() { # path glob
  local hit
  hit="$(find src-tauri -type f -path "$1" -newer "$MARK" 2>/dev/null | head -n1)"
  if [[ -z "$hit" ]]; then
    hit="$(find src-tauri -type f -path "$1" -printf '%T@ %p\n' 2>/dev/null | sort -nr | head -n1 | cut -d' ' -f2-)"
    [[ -n "$hit" ]] && warn "nothing new matched $1, using $hit from an earlier build"
  fi
  [[ -n "$hit" ]] && echo "$hit"
}

# Copies a build result, with its signature when the build made one.
collect() { # source name
  if [[ $DRY -eq 1 ]]; then say "${D}  would copy to $OUT/$2${N}"; BUILT+=("$2"); return 0; fi
  # a desktop build with update bundles has to come with its signature, or the update cannot be verified
  if [[ $UPDATER_ON -eq 1 && $UNSIGNED -eq 0 && "$2" != *.apk && ! -f "$1.sig" ]]; then
    warn "no $1.sig, the build made no update bundle"
    return 1
  fi
  cp -f "$1" "$OUT/$2" || return 1
  [[ -f "$1.sig" ]] && cp -f "$1.sig" "$OUT/$2.sig"
  BUILT+=("$2")
}

# Looks into the finished AppImage and succeeds when it carries its icon: the desktop file names one,
# a PNG of that name is in it, and .DirIcon, which file managers show, points to something.
# --appimage-extract only unpacks, the app does not start. Fails only when the AppImage could be
# opened and has no icon, a bundle that cannot be inspected is a warning.
check_appimage_icon() { # path
  local app tmp root desktop name found
  app="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
  tmp="$(mktemp -d)"; root="$tmp/squashfs-root"
  (
    cd "$tmp" || exit 1
    "$app" --appimage-extract '.DirIcon' >/dev/null 2>&1
    "$app" --appimage-extract '*.desktop' >/dev/null 2>&1
  )
  desktop="$(find "$root" -maxdepth 1 -name '*.desktop' 2>/dev/null | head -n1)"
  if [[ -z "$desktop" && ! -e "$root/.DirIcon" && ! -L "$root/.DirIcon" ]]; then
    warn "could not look into the AppImage, its icon is not checked"
    rm -rf "$tmp"; return 0
  fi
  name="$(sed -n 's/^Icon=//p' "$desktop" 2>/dev/null | head -n1)"
  if [[ -z "$name" ]]; then
    warn "the AppImage has no Icon= in its desktop file"; rm -rf "$tmp"; return 1
  fi
  ( cd "$tmp" && "$app" --appimage-extract "$name.png" >/dev/null 2>&1; "$app" --appimage-extract 'usr/share/icons/*' >/dev/null 2>&1 )
  found="$(find "$root" -name "$name.png" 2>/dev/null | head -n1)"
  if [[ -z "$found" ]]; then
    warn "the AppImage names the icon \"$name\" but contains no $name.png"; rm -rf "$tmp"; return 1
  fi
  if [[ ! -e "$root/.DirIcon" ]]; then
    warn "the AppImage has no working .DirIcon, file managers show a generic icon"; rm -rf "$tmp"; return 1
  fi
  local sizes; sizes="$(find "$root/usr/share/icons" -name "$name.png" 2>/dev/null | sed -n 's#.*/hicolor/\([^/]*\)/apps/.*#\1#p' | sort -V | tr '\n' ' ')"
  say "AppImage icon ok, ${name}.png${sizes:+ in $sizes}"
  rm -rf "$tmp"
}

build_linux() {
  # NO_STRIP: the strip of linuxdeploy fails on libraries of newer distros.
  # APPIMAGE_EXTRACT_AND_RUN: linuxdeploy is itself an AppImage, this works without FUSE.
  run env NO_STRIP=true APPIMAGE_EXTRACT_AND_RUN=1 npm run tauri -- build --bundles appimage || return 1
  [[ $DRY -eq 1 ]] && { collect "" "$(artifact Linux-x86-64 "$DESCRIPTOR" "" AppImage)"; return; }
  local src; src="$(find_new '*/bundle/appimage/*.AppImage')"
  [[ -n "$src" ]] || { warn "no AppImage found"; return 1; }
  check_appimage_icon "$src" || return 1
  collect "$src" "$(artifact Linux-x86-64 "$DESCRIPTOR" "" AppImage)"
}

build_windows() {
  if [[ "$HOST" == windows ]]; then
    run npm run tauri -- build --bundles nsis || return 1
  else
    # an experimental path of Tauri, the installer is made on a host that is not Windows
    run rustup target add x86_64-pc-windows-msvc || return 1
    run npm run tauri -- build --runner cargo-xwin --target x86_64-pc-windows-msvc --bundles nsis || return 1
  fi
  [[ $DRY -eq 1 ]] && { collect "" "$(artifact Windows_x86-64 "$DESCRIPTOR" setup exe)"; return; }
  local src; src="$(find_new '*/bundle/nsis/*-setup.exe')"
  [[ -n "$src" ]] || { warn "no setup exe found"; return 1; }
  collect "$src" "$(artifact Windows_x86-64 "$DESCRIPTOR" setup exe)"
}

build_android() {
  local sdk; sdk="$(android_sdk)"
  export ANDROID_HOME="$sdk"
  if [[ -z "${NDK_HOME:-}" ]]; then
    NDK_HOME="$(find "$sdk/ndk" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | sort -V | tail -n1)"
    export NDK_HOME
    say "Using the NDK at $NDK_HOME"
  fi
  local missing=()
  if command -v rustup >/dev/null; then
    local installed t; installed="$(rustup target list --installed)"
    for t in "${ANDROID_TARGETS[@]}"; do grep -qx "$t" <<<"$installed" || missing+=("$t"); done
    if [[ ${#missing[@]} -gt 0 ]]; then
      confirm "Add the Rust targets ${missing[*]}?" || { warn "the universal APK needs all four Rust targets"; return 1; }
      run rustup target add "${missing[@]}" || return 1
    fi
  fi
  # no --split-per-abi, so every architecture ends up in one APK
  run npm run tauri -- android build --apk || return 1
  [[ $DRY -eq 1 ]] && { collect "" "$(artifact Android_universal "$DESCRIPTOR" "" apk)"; return; }
  local src; src="$(find_new '*/outputs/apk/universal/release/*.apk')"
  [[ -n "$src" ]] || { warn "no APK found"; return 1; }
  # an unsigned APK cannot update an installed app, which is what the keystore is for
  [[ "$src" != *unsigned* ]] || { warn "the APK is unsigned, check keystore.properties"; return 1; }
  collect "$src" "$(artifact Android_universal "$DESCRIPTOR" "" apk)"
}

for t in "${SELECTED[@]}"; do
  step "Building $(label "$t")"
  touch "$MARK"
  if "build_$t"; then say "${G}$t done${N}"; else FAILED+=("$t"); warn "$t failed"; fi
done

# SECTION: latest.json

# The manifest of the updater plugin: the version, the notes and, per platform, the url of the bundle
# and the content of its .sig file. It is made from the files of the output folder, so a later run
# for one platform keeps the other. Android is not in it, the app finds the APK by itself.
write_manifest() {
  [[ $UPDATER_ON -eq 1 && $UNSIGNED -eq 0 && -n "$RELEASE_REPO" ]] || return 0
  local base="https://github.com/$RELEASE_REPO/releases/download/$TAG" args=() missing=() pair key file
  for pair in "linux-x86_64|$(artifact Linux-x86-64 "$DESCRIPTOR" "" AppImage)" \
              "windows-x86_64|$(artifact Windows_x86-64 "$DESCRIPTOR" setup exe)"; do
    key="${pair%%|*}"; file="${pair#*|}"
    if [[ $DRY -eq 1 ]]; then
      [[ " ${SELECTED[*]} " == *" ${key%%-*} "* ]] && args+=("$key" "$file") || missing+=("$key")
    elif [[ -f "$OUT/$file" && -f "$OUT/$file.sig" ]]; then
      args+=("$key" "$file")
    else
      missing+=("$key")
    fi
  done
  [[ ${#missing[@]} -eq 0 ]] ||
    warn "latest.json has no entry for ${missing[*]}, an install of that platform gets an error when it checks for updates"
  [[ ${#args[@]} -gt 0 ]] || return 0
  if [[ $DRY -eq 1 ]]; then say "${D}  would write $OUT/latest.json for ${args[0]}...${N}"; return 0; fi
  node -e '
    const fs = require("fs"), path = require("path");
    const [out, version, notes, base, ...rest] = process.argv.slice(1);
    const platforms = {};
    for (let i = 0; i < rest.length; i += 2) {
      const file = rest[i + 1];
      platforms[rest[i]] = {
        signature: fs.readFileSync(path.join(out, file + ".sig"), "utf8").trim(),
        url: base + "/" + encodeURIComponent(file),
      };
    }
    const manifest = { version, pub_date: new Date().toISOString().replace(/\.\d+Z$/, "Z"), platforms };
    if (notes) manifest.notes = notes;
    console.log(JSON.stringify(manifest, null, 2));
  ' "$OUT" "$VERSION" "$NOTES" "$base" "${args[@]}" >"$OUT/latest.json" || { warn "could not write latest.json"; return 1; }
  say "Wrote $OUT/latest.json"
}

step "Update manifest"
write_manifest || FAILED+=("latest.json")

# SECTION: result

sha() { if command -v sha256sum >/dev/null; then sha256sum "$@"; else shasum -a 256 "$@"; fi; }

step "Result"
if [[ ${#BUILT[@]} -gt 0 && $DRY -eq 0 ]]; then
  (
    cd "$OUT" || exit 1
    # every file of the folder, so a later partial run keeps the earlier entries
    # shellcheck disable=SC2046
    sha $(ls -1 | grep -v '^SHA256SUMS$') >SHA256SUMS
    for f in $(ls -1 | grep -v '^SHA256SUMS$'); do
      printf '  %s  %s\n' "$(du -h "$f" | cut -f1)" "$f"
    done
  )
  say ""
  say "Written to $OUT, with SHA256SUMS."
  say ""
  say "Upload to the GitHub release ${B}$TAG${N}:"
  for f in $(ls -1 "$OUT" | grep -v '^SHA256SUMS$'); do say "  $f"; done
  if [[ $UPDATER_ON -eq 1 || " ${SELECTED[*]} " == *" android "* ]]; then
    say "${Y}Publish it as the latest release, not as a pre-release.${N} GitHub's /releases/latest skips pre-releases,"
    say "and both the desktop updater (latest.json) and the Android check read it."
  fi
elif [[ $DRY -eq 1 ]]; then
  for f in "${BUILT[@]}"; do say "  $f"; done
fi
[[ $UNSIGNED -eq 1 ]] && warn "built without update bundles, there are no .sig files"

STATUS=0
if [[ ${#FAILED[@]} -gt 0 ]]; then
  printf '%sfailed: %s%s\n' "$R" "${FAILED[*]}" "$N" >&2
  STATUS=1
fi
if [[ $ALL_REQUESTED -eq 1 && ${#UNAVAILABLE[@]} -gt 0 ]]; then
  printf '%sthe release is incomplete, %d target(s) were skipped%s\n' "$R" "${#UNAVAILABLE[@]}" "$N" >&2
  STATUS=1
fi
exit $STATUS
