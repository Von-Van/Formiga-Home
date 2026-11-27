#!/usr/bin/env bash
# Writes a new Home version, and optionally a new Formiga Desktop release tag, into every place
# that names them, and refreshes Cargo.lock to match, so a release commit never misses one.
#
# Desktop's crates are normally pinned to a release tag. Between releases they may be pinned to a
# commit on Desktop's main branch instead (`rev = "..."`); every Desktop crate must still name the
# same one. Giving a Desktop tag moves a commit pin back onto that tag, in the README too.
#
# The changelog stays a person's to write: this script only says when the new version has no
# section in CHANGELOG.md yet, which the release workflow needs for its notes. A new Desktop tag
# can bring new travel and household versions; the README names them beside the tag, so check it.
#
# Usage:
#   scripts/set-version.sh <version> [<desktop tag>]   e.g. scripts/set-version.sh 0.67.4 v0.67.4
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
# Every Desktop crate's pin, as `tag=v1.2.3` or `rev=<commit>`: they must all be one and the same.
pins="$(sed -nE 's/.*Formiga-Desktop", (tag|rev) = "([^"]*)".*/\1=\2/p' Cargo.toml | sort -u)"
desktop_crates="$(grep -c 'git = "https://github.com/Von-Van/Formiga-Desktop"' Cargo.toml || true)"
pinned_crates="$(grep -cE 'Formiga-Desktop", (tag|rev) = "' Cargo.toml || true)"
pin_kind="${pins%%=*}"
pin="${pins#*=}"
if [ -z "$current" ] || [ -z "$pins" ] || [ "$(printf '%s\n' "$pins" | wc -l)" -ne 1 ] \
  || [ "$desktop_crates" -ne "$pinned_crates" ] \
  || { [ "$pin_kind" = tag ] && ! [[ "$pin" =~ ^v[0-9] ]]; } \
  || { [ "$pin_kind" = rev ] && ! [[ "$pin" =~ ^[0-9a-f]{7,40}$ ]]; }; then
  echo "Cargo.toml needs one version and one Formiga Desktop tag or commit for all of Desktop's" \
    "crates; it has '$current' and '$(printf '%s' "$pins" | tr '\n' ' ')'" >&2
  exit 1
fi
if [ "$pin_kind" = tag ]; then
  tag="$pin"
  described="Formiga Desktop $pin"
else
  tag=""
  described="Formiga Desktop commit ${pin:0:7}"
fi

if [ "${1:-}" = "--check" ]; then
  missing=0
  grep -qF "<string>$current</string>" "$plist" || { echo "$plist does not name $current" >&2; missing=1; }
  # On a commit between releases the README still names the last tag, which is left alone.
  if [ "$pin_kind" = tag ]; then
    for file in "${tag_docs[@]}"; do
      grep -qF "\`$tag\`" "$file" || { echo "$file does not name $tag" >&2; missing=1; }
    done
  fi
  locked="$(awk '$0 == "name = \"formiga-home\"" { getline; gsub(/version = |"/, ""); print; exit }' Cargo.lock)"
  [ "$locked" = "$current" ] || { echo "Cargo.lock has formiga-home $locked, not $current" >&2; missing=1; }
  if grep 'source = "git+https://github.com/Von-Van/Formiga-Desktop' Cargo.lock \
    | grep -vqF "?$pin_kind=$pin#"; then
    echo "Cargo.lock takes some of Desktop's crates from another $pin_kind than $pin" >&2
    missing=1
  fi
  [ "$missing" -eq 0 ] && echo "Every place names $current and $described."
  exit "$missing"
fi

new="${1:-}"
new="${new#v}"
new_tag="${2:-$tag}"
# Pinned to a commit and given no tag, the pin stays as it is and only Home's version moves.
[ -n "$new_tag" ] && [ "${new_tag#v}" = "$new_tag" ] && new_tag="v$new_tag"
if ! [[ "$new" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] \
  || { [ -n "$new_tag" ] && ! [[ "$new_tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; }; then
  echo "usage: $(basename "$0") <version> [<desktop tag>]   e.g. $(basename "$0") 0.67.4 v0.67.4" >&2
  echo "       $(basename "$0") --check" >&2
  exit 2
fi
# Running it again with the same values is how an interrupted run is finished: the files are left
# as they are and Cargo.lock is brought up to them.

# The dots are literal, and 0.1.1 must not match inside 0.1.10.
literal() { printf '%s' "$1" | sed 's/\./\\./g'; }
old_version="$(literal "$current")"

perl -pi -e "s/^version = \"$old_version\"\$/version = \"$new\"/" Cargo.toml
perl -pi -e "s/<string>$old_version<\\/string>/<string>$new<\\/string>/" "$plist"
if [ "$pin_kind" = tag ]; then
  old_tag="$(literal "$tag")"
  for file in Cargo.toml "${tag_docs[@]}"; do
    perl -pi -e "s/(?<![0-9.])$old_tag(?![0-9])/$new_tag/g" "$file"
  done
  described="Formiga Desktop $new_tag"
elif [ -n "$new_tag" ]; then
  # Back from a commit on Desktop's main branch onto a release tag: the crates take the tag, and
  # the README's "currently `vX.Y.Z`" names it.
  perl -pi -e "s/(Formiga-Desktop\", )rev = \"$pin\"/\${1}tag = \"$new_tag\"/" Cargo.toml
  for file in "${tag_docs[@]}"; do
    perl -pi -e "s/currently \\\`v[0-9]+\\.[0-9]+\\.[0-9]+\\\`/currently \\\`$new_tag\\\`/" "$file"
    grep -qF "\`$new_tag\`" "$file" || echo "Still to write by hand: $file does not name $new_tag yet" >&2
  done
  described="Formiga Desktop $new_tag"
fi
# Only Desktop's crates and this app's own entry move; nothing else in the lockfile is touched.
crates=()
while read -r crate; do crates+=(-p "$crate"); done \
  < <(sed -n 's/^\([a-z-]*\) = { git = "https:\/\/github.com\/Von-Van\/Formiga-Desktop".*/\1/p' Cargo.toml)
cargo update --quiet "${crates[@]}"
cargo update --workspace --quiet

echo "Home is now $new on $described."
grep -qF "## $new " CHANGELOG.md || echo "Still to write by hand: the CHANGELOG.md section, ## $new ($(date +%F))"
