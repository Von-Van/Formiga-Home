#!/usr/bin/env bash
# Writes a new Home version, and optionally a new Formiga Desktop release tag, into every place
# that names them, and refreshes Cargo.lock to match, so a release commit never misses one.
#
# The changelog stays a person's to write: this script only says when the new version has no
# section in CHANGELOG.md yet, which the release workflow needs for its notes. A new Desktop tag
# can bring new travel and household versions; the README names them beside the tag, so check it.
#
# Usage:
#   scripts/set-version.sh <version> [<desktop tag>]   e.g. scripts/set-version.sh 0.1.2 v0.67.2
#   scripts/set-version.sh --check                     says whether every place agrees
#
# Tools used: cargo, perl, grep and sed.

set -euo pipefail

repo_dir="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_dir"

plist=packaging/macos/Info.plist
# Files that name the Desktop tag, besides Cargo.toml.
tag_docs=(README.md)

current="$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)"
tag="$(sed -n 's/.*Formiga-Desktop", tag = "\(v[^"]*\)".*/\1/p' Cargo.toml | sort -u)"
if [ -z "$current" ] || [ "$(printf '%s\n' "$tag" | wc -l)" -ne 1 ] || [ -z "$tag" ]; then
  echo "Cargo.toml needs one version and one Formiga Desktop tag; it has '$current' and '$tag'" >&2
  exit 1
fi

if [ "${1:-}" = "--check" ]; then
  missing=0
  grep -qF "<string>$current</string>" "$plist" || { echo "$plist does not name $current" >&2; missing=1; }
  for file in "${tag_docs[@]}"; do
    grep -qF "\`$tag\`" "$file" || { echo "$file does not name $tag" >&2; missing=1; }
  done
  [ "$missing" -eq 0 ] && echo "Every place names $current and $tag."
  exit "$missing"
fi

new="${1:-}"
new="${new#v}"
new_tag="${2:-$tag}"
[ "${new_tag#v}" = "$new_tag" ] && new_tag="v$new_tag"
if ! [[ "$new" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || ! [[ "$new_tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "usage: $(basename "$0") <version> [<desktop tag>]   e.g. $(basename "$0") 0.1.2 v0.67.2" >&2
  echo "       $(basename "$0") --check" >&2
  exit 2
fi
if [ "$new" = "$current" ] && [ "$new_tag" = "$tag" ]; then
  echo "The version is already $current on $tag" >&2
  exit 1
fi

# The dots are literal, and 0.1.1 must not match inside 0.1.10.
literal() { printf '%s' "$1" | sed 's/\./\\./g'; }
old_version="$(literal "$current")"
old_tag="$(literal "$tag")"

perl -pi -e "s/^version = \"$old_version\"\$/version = \"$new\"/" Cargo.toml
perl -pi -e "s/<string>$old_version<\\/string>/<string>$new<\\/string>/" "$plist"
if [ "$new_tag" != "$tag" ]; then
  for file in Cargo.toml "${tag_docs[@]}"; do
    perl -pi -e "s/(?<![0-9.])$old_tag(?![0-9])/$new_tag/g" "$file"
  done
  # Only Desktop's crates move; nothing else in the lockfile is touched.
  crates=()
  while read -r crate; do crates+=(-p "$crate"); done \
    < <(sed -n 's/^\([a-z-]*\) = { git = "https:\/\/github.com\/Von-Van\/Formiga-Desktop".*/\1/p' Cargo.toml)
  cargo update --quiet "${crates[@]}"
fi
cargo update --workspace --quiet

echo "Home is now $new on Formiga Desktop $new_tag."
grep -qF "## $new " CHANGELOG.md || echo "Still to write by hand: the CHANGELOG.md section, ## $new ($(date +%F))"
