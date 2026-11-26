#!/bin/bash
# packaging-release · family version 4
# Build ERAVON.app and wrap it in a .dmg and a .pkg.
#
# Usage: packaging/macos/build.sh [--identity NAME]
#   --identity NAME   code signing identity (default: "-", an ad-hoc signature)
#
# Output goes to dist/: ERAVON-<version>-macos-<arch>.dmg and .pkg.
# The .pkg installs ERAVON.app into /Applications and the `eravon` command line into /usr/local/bin.
# Both carry the third-party notices (ERAVON.app/Contents/Resources/THIRD-PARTY-LICENSES.txt) and nothing of
# the machine that built them: no extended attributes, owners and dates neutral in the .dmg's catalog, partition
# names in no one's language, and in the .pkg no owner, inode, device or system build.
set -euo pipefail

IDENTITY="-"
while [ $# -gt 0 ]; do
  case "$1" in
    --identity) IDENTITY="$2"; shift 2 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
VERSION="$(sed -n 's/^version *= *"\(.*\)"/\1/p' Cargo.toml | head -1)"
ARCH="$(uname -m)"
# One build of the binaries, with the system notifier (`real-notify`): no other build turns the notifier on.
FEATURES=app/real-notify
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
DIST="$ROOT/dist"
export CARGO_TARGET_DIR="$TARGET_DIR"
WORK="$DIST/macos-work"
APP="$WORK/ERAVON.app"
PACK="$TARGET_DIR/release/eravon-pack"
# Every date the packages record: the moment of the commit being built, unless SOURCE_DATE_EPOCH says another.
export SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH:-$(git log -1 --format=%ct 2>/dev/null || echo 0)}"

# Keep local paths (home directory, checkout, build directory) out of the shipped binaries: the packaging tool
# is built first and gives the flags, from the one table of path mappings (`eravon-pack rustflags`).
cargo build --release --locked -p eravon-pack
CARGO_ENCODED_RUSTFLAGS="$("$PACK" rustflags --target-dir "$TARGET_DIR")"
export CARGO_ENCODED_RUSTFLAGS
cargo build --release --locked --features "$FEATURES" -p app -p eravon-cli
# The built binaries carry no path of this machine (`eravon-pack no-paths`); no package is made if they do.
"$PACK" no-paths "$TARGET_DIR/release/eravon-app" "$TARGET_DIR/release/eravon"

rm -rf "$WORK"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources" "$DIST"
# The window binary and the command line must differ by more than case: macOS volumes are usually case-insensitive.
# The bundle (both binaries and Info.plist, made from packaging/macos/Info.plist) is laid by the app itself
# (`eravon-app bundle`, crates/app/src/bundle.rs), so a package and a local bundle carry the same plist.
# Copies carry no extended attributes (`-X`, and `xattr -cr` below): no provenance or quarantine marks of this machine.
"$TARGET_DIR/release/eravon-app" bundle --at "$WORK" > /dev/null
cp -X packaging/icon/eravon.icns "$APP/Contents/Resources/eravon.icns"
# The notices for everything the bundle holds, with the licences of the two fonts the macOS build embeds.
"$PACK" notices --target "$(rustc -vV | sed -n 's/^host: //p')" --root app --root eravon-cli \
  --with crates/eravon-ui/fonts/OFL-Inter.txt --with crates/eravon-ui/fonts/OFL-JetBrainsMono.txt \
  --out "$APP/Contents/Resources/THIRD-PARTY-LICENSES.txt"
xattr -cr "$APP"

codesign --force --sign "$IDENTITY" --timestamp=none "$APP/Contents/MacOS/eravon"
codesign --force --sign "$IDENTITY" --timestamp=none "$APP/Contents/MacOS/eravon-app"
codesign --force --sign "$IDENTITY" --timestamp=none "$APP"
codesign --verify --strict "$APP"

# .dmg: the app and a link to /Applications. Made first as a read/write image with no partition map, so the
# volume starts at the image's first byte and its catalog can be rewritten: owners and groups of this user
# become the "unknown" owner (whoever opens it), every date the commit's moment. Then compressed, and the
# partition names (written in this machine's language) rewritten in one neutral form.
DMG_SRC="$WORK/dmg"
mkdir -p "$DMG_SRC"
ditto --noextattr --noqtn "$APP" "$DMG_SRC/ERAVON.app"
ln -s /Applications "$DMG_SRC/Applications"
DMG="$DIST/ERAVON-$VERSION-macos-$ARCH.dmg"
RAW="$WORK/dmg-raw.dmg"
rm -f "$DMG" "$RAW"
hdiutil create -quiet -volname "ERAVON $VERSION" -srcfolder "$DMG_SRC" -fs HFS+ -layout NONE -format UDRW "$RAW"
"$PACK" hfs-owners "$RAW" --epoch "$SOURCE_DATE_EPOCH"
hdiutil convert -quiet "$RAW" -format UDZO -o "$DMG"
"$PACK" dmg-names "$DMG"
hdiutil verify -quiet "$DMG"
codesign --force --sign "$IDENTITY" --timestamp=none "$DMG"

# .pkg: the app into /Applications, the command line into /usr/local/bin.
PKG_ROOT="$WORK/pkgroot"
mkdir -p "$PKG_ROOT/Applications" "$PKG_ROOT/usr/local/bin"
ditto --noextattr --noqtn "$APP" "$PKG_ROOT/Applications/ERAVON.app"
cp -X "$APP/Contents/MacOS/eravon" "$PKG_ROOT/usr/local/bin/eravon"
xattr -cr "$PKG_ROOT"
PKG="$DIST/ERAVON-$VERSION-macos-$ARCH.pkg"
FLAT="$WORK/flat.pkg"
rm -f "$PKG"
pkgbuild --quiet --root "$PKG_ROOT" --identifier com.kaptonia.eravon.pkg --version "$VERSION" \
  --install-location / "$FLAT"
# The flat package's table of contents records the builder's owner, group, inodes, device and extended
# attributes for its own members, and PackageInfo names the system build of the packaging tools. The members
# are taken out, PackageInfo loses the system build, and the archive is made again without those properties
# (the payload byte for byte as pkgbuild made it).
EXP="$WORK/pkg-members"
pkgutil --expand "$FLAT" "$EXP"
sed -E -i '' 's/(generator-version="[^" ]*) \([^)]*\)"/\1"/' "$EXP/PackageInfo"
xattr -cr "$EXP"
( cd "$EXP" && xar -c -f "$PKG" --compression gzip --no-compress '^Payload$' \
    --prop-exclude user --prop-exclude uid --prop-exclude group --prop-exclude gid \
    --prop-exclude inode --prop-exclude deviceno --prop-exclude ea --prop-exclude FinderCreateTime \
    --prop-exclude atime --prop-exclude ctime --prop-exclude mtime \
    Bom Payload PackageInfo )

rm -rf "$WORK"
( cd "$DIST" && shasum -a 256 "$(basename "$DMG")" "$(basename "$PKG")" > "SHA256SUMS-macos-$ARCH.txt" )
echo "built: $DMG"
echo "built: $PKG"
