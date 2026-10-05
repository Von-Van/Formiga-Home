#!/usr/bin/env bash
# Builds Formiga Home as a universal macOS app that Formiga Desktop can find: the bundle id and
# the Home version it reads are the ones formiga-home-contract's discovery module names.
set -euo pipefail

script_dir="$(cd "$(dirname "$0")" && pwd)"
repo_dir="$(cd "$script_dir/.." && pwd)"
dist_dir="$repo_dir/dist"
app_dir="$dist_dir/Formiga Home.app"
version="${FORMIGA_HOME_VERSION:-$(cargo metadata --no-deps --format-version 1 --manifest-path "$repo_dir/Cargo.toml" | python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"] == "formiga-home"))')}"
version="${version#v}"
build_number="${FORMIGA_HOME_BUILD_NUMBER:-1}"
archive="$dist_dir/Formiga-Home-$version-macOS-universal.zip"
disk_image="$dist_dir/Formiga-Home-$version-macOS-universal.dmg"
dmg_staging="$dist_dir/Formiga-Home-dmg"

cd "$repo_dir"
rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo build --release -p formiga-home --target aarch64-apple-darwin
cargo build --release -p formiga-home --target x86_64-apple-darwin

rm -rf "$app_dir" "$dmg_staging"
mkdir -p "$app_dir/Contents/MacOS" "$app_dir/Contents/Resources"
cp "$repo_dir/packaging/macos/Info.plist" "$app_dir/Contents/Info.plist"
lipo -create \
    "$repo_dir/target/aarch64-apple-darwin/release/formiga-home" \
    "$repo_dir/target/x86_64-apple-darwin/release/formiga-home" \
    -output "$app_dir/Contents/MacOS/Formiga Home"
chmod 755 "$app_dir/Contents/MacOS/Formiga Home"

# The icon is drawn by the binary itself, from the same picture as the window's.
icon_dir="$(mktemp -d)"
"$app_dir/Contents/MacOS/Formiga Home" --icon "$icon_dir" > /dev/null
cp "$icon_dir/FormigaHome.icns" "$app_dir/Contents/Resources/FormigaHome.icns"
rm -rf "$icon_dir"

# The Home version comes from the binary itself, so the bundle can never claim another.
home_version="$("$app_dir/Contents/MacOS/Formiga Home" --home-version)"
plist="$app_dir/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $version" "$plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleVersion $build_number" "$plist"
/usr/libexec/PlistBuddy -c "Set :FormigaHomeVersion $home_version" "$plist"

if [[ -n "${FORMIGA_CODESIGN_IDENTITY:-}" ]]; then
    codesign --force --deep --options runtime --timestamp \
        --sign "$FORMIGA_CODESIGN_IDENTITY" "$app_dir"
else
    codesign --force --deep --sign - "$app_dir"
fi

ditto -c -k --sequesterRsrc --keepParent "$app_dir" "$archive"
mkdir -p "$dmg_staging"
ditto "$app_dir" "$dmg_staging/Formiga Home.app"
ln -s /Applications "$dmg_staging/Applications"
cp "$repo_dir/packaging/macos/README.txt" "$dmg_staging/Read Me.txt"
hdiutil create -volname "Formiga Home" -srcfolder "$dmg_staging" -ov -format UDZO "$disk_image"

if [[ -n "${FORMIGA_NOTARY_PROFILE:-}" ]]; then
    xcrun notarytool submit "$disk_image" --keychain-profile "$FORMIGA_NOTARY_PROFILE" --wait
    xcrun stapler staple "$disk_image"
fi

# Checksums name the file alone, so `shasum -c` works wherever the files are downloaded to.
(cd "$dist_dir" && for file in "$archive" "$disk_image"; do
    name="$(basename "$file")"
    shasum -a 256 "$name" > "$name.sha256"
done)

echo "Packaged $disk_image and $archive (Home version $home_version)"
