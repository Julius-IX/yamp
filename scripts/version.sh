#!/usr/bin/env bash
# Sets the version of yamp in every file that carries it, so they cannot drift apart:
#
#   src-tauri/tauri.conf.json   the one the updater and the release name use
#   package.json, package-lock.json
#   src-tauri/Cargo.toml, src-tauri/Cargo.lock
#
# Usage: scripts/version.sh <version | major | minor | patch> [options]
#
#   scripts/version.sh 1.0.0          set it
#   scripts/version.sh v1.2.3-alpha   set 1.2.3, see below for the -alpha
#   scripts/version.sh minor          0.1.0 -> 0.2.0
#
# The descriptor (alpha, beta) is not part of the version in the files. scripts/build.sh adds it to
# the folder, the tag and the file names with -d, and an updater compares only the numbers anyway.
# A suffix given here is therefore left out and the script says what to pass to build.sh. With
# --keep-suffix it is written as given, which makes build.sh name things v1.2.3-alpha-alpha.
#
#   -n, --dry-run      show what would change, write nothing
#       --keep-suffix  write the suffix into the files
#   -h, --help         this text
#
# Nothing is written unless every file has a version to replace.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 1

DRY=0
KEEP=0
ARG=""

if [[ -t 1 ]]; then
  B=$'\033[1m' G=$'\033[32m' Y=$'\033[33m' R=$'\033[31m' D=$'\033[2m' N=$'\033[0m'
else
  B="" G="" Y="" R="" D="" N=""
fi
die() { printf '%serror:%s %s\n' "$R" "$N" "$*" >&2; exit 1; }

while [[ $# -gt 0 ]]; do
  case "$1" in
    -n|--dry-run) DRY=1; shift ;;
    --keep-suffix) KEEP=1; shift ;;
    -h|--help) sed -n '2,/^set -uo/p' "${BASH_SOURCE[0]}" | sed '$d' | sed 's/^# \{0,1\}//'; exit 0 ;;
    -*) die "unknown option $1" ;;
    *) [[ -z "$ARG" ]] || die "one version only"; ARG="$1"; shift ;;
  esac
done
[[ -n "$ARG" ]] || die "give a version or major, minor, patch, see --help"
command -v node >/dev/null || die "node is not installed"

DRY="$DRY" KEEP="$KEEP" ARG="$ARG" node - <<'NODE'
const fs = require("fs");
const env = process.env;
const [B, G, Y, D, N] = process.stdout.isTTY
  ? ["\x1b[1m", "\x1b[32m", "\x1b[33m", "\x1b[2m", "\x1b[0m"]
  : ["", "", "", "", ""];

// Each entry: the file and a pattern whose group 1 is everything before the version and group 2
// everything after it. Only the first match is replaced.
const FILES = [
  ["src-tauri/tauri.conf.json", /("version"\s*:\s*")[^"]*(")/],
  ["package.json", /("version"\s*:\s*")[^"]*(")/],
  ["package-lock.json", /^(\{\s*"name":\s*"[^"]*",\s*"version":\s*")[^"]*(")/],
  ["package-lock.json", /("packages":\s*\{\s*"":\s*\{[^}]*?"version":\s*")[^"]*(")/],
  ["src-tauri/Cargo.toml", /(\[package\][^[]*?\nversion\s*=\s*")[^"]*(")/],
  ["src-tauri/Cargo.lock", /(\[\[package\]\]\r?\nname = "yamp"\r?\nversion = ")[^"]*(")/],
];

const SEMVER = /^v?(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?$/;
const fail = (m) => { console.error(`error: ${m}`); process.exit(1); };

const read = (f) => { try { return fs.readFileSync(f, "utf8"); } catch { return fail(`cannot read ${f}`); } };

const confMatch = /"version"\s*:\s*"([^"]*)"/.exec(read(FILES[0][0]));
if (!confMatch) fail(`no version in ${FILES[0][0]}`);
const current = confMatch[1];
const cur = SEMVER.exec(current);
if (!cur) fail(`the current version "${current}" is not a dotted version`);

let next;
let suffix = "";
if (["major", "minor", "patch"].includes(env.ARG)) {
  let [a, b, c] = [+cur[1], +cur[2], +cur[3]];
  if (env.ARG === "major") [a, b, c] = [a + 1, 0, 0];
  else if (env.ARG === "minor") [b, c] = [b + 1, 0];
  else c += 1;
  next = `${a}.${b}.${c}`;
} else {
  const m = SEMVER.exec(env.ARG);
  if (!m) fail(`"${env.ARG}" is not a version like 1.2.3 or 1.2.3-alpha, or major, minor, patch`);
  next = `${m[1]}.${m[2]}.${m[3]}`;
  suffix = m[4] || "";
}
const written = suffix && env.KEEP === "1" ? `${next}-${suffix}` : next;

// everything is computed before anything is written
const texts = new Map();
const rows = [];
for (const [file, re] of FILES) {
  const text = texts.get(file) ?? read(file);
  const m = re.exec(text);
  if (!m) fail(`found no version to replace in ${file}, nothing was written`);
  const old = text.slice(m.index + m[1].length, m.index + m[0].length - m[2].length);
  texts.set(file, text.slice(0, m.index + m[1].length) + written + text.slice(m.index + m[0].length - m[2].length));
  rows.push([file, old]);
}

const width = Math.max(...rows.map(([f]) => f.length));
console.log(`${B}${current} -> ${written}${N}${env.DRY === "1" ? `  ${D}(dry run)${N}` : ""}`);
for (const [file, old] of rows) {
  const same = old === current;
  console.log(`  ${file.padEnd(width)}  ${same ? "" : Y}${old}${same ? "" : N}${old === written ? `  ${D}unchanged${N}` : ""}`);
}
const odd = rows.filter(([, old]) => old !== current);
if (odd.length) console.log(`${Y}warning:${N} ${odd.length} file(s) did not match ${current} before, they are set now`);

if (env.DRY !== "1") {
  for (const [file, text] of texts) fs.writeFileSync(file, text);
  console.log(`${G}written${N}`);
}

if (suffix && env.KEEP !== "1") {
  console.log(`\nThe suffix "-${suffix}" is not in the files. Build it as: scripts/build.sh --release -d ${suffix}`);
} else if (suffix) {
  console.log(`\n${Y}warning:${N} the suffix is in the files, scripts/build.sh will name the release v${written}-<descriptor>`);
}
if (env.DRY !== "1") {
  console.log(`\nNext: git commit -am "chore: bump to ${written}"`);
}
NODE
