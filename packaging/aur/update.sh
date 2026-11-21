#!/bin/bash
# Points the AUR package at a published release: sets pkgver, resets pkgrel, fills in the
# sha256 of the release's .deb and LICENSE, and regenerates .SRCINFO. Works without Arch/makepkg.
# Usage: packaging/aur/update.sh 1.0.1
set -euo pipefail
cd "$(dirname "$0")"

ver="${1:?usage: update.sh <version>}"
repo="chromag316/forever-addon-manager"
deb_url="https://github.com/$repo/releases/download/v$ver/Forever-AddOn-Manager-$ver-Linux-amd64.deb"
lic_url="https://raw.githubusercontent.com/$repo/v$ver/LICENSE"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
echo "Downloading $deb_url"
curl -fsSL -o "$tmp/pkg.deb" "$deb_url"
echo "Downloading $lic_url"
curl -fsSL -o "$tmp/LICENSE" "$lic_url"
deb_sum="$(shasum -a 256 "$tmp/pkg.deb" | cut -d' ' -f1)"
lic_sum="$(shasum -a 256 "$tmp/LICENSE" | cut -d' ' -f1)"
echo "sha256 .deb:    $deb_sum"
echo "sha256 LICENSE: $lic_sum"

sed -i.bak -e "s/^pkgver=.*/pkgver=$ver/" -e "s/^pkgrel=.*/pkgrel=1/" \
    -e "s/^sha256sums=.*/sha256sums=('$lic_sum')/" \
    -e "s/^sha256sums_x86_64=.*/sha256sums_x86_64=('$deb_sum')/" PKGBUILD
rm -f PKGBUILD.bak

# .SRCINFO in the format `makepkg --printsrcinfo` produces.
cat > .SRCINFO <<EOF
pkgbase = forever-addon-manager-bin
	pkgdesc = Small, fast addon manager for WoW Forever (CurseForge, Git and local folders)
	pkgver = $ver
	pkgrel = 1
	url = https://github.com/$repo
	arch = x86_64
	license = MIT
	depends = webkit2gtk-4.1
	depends = gtk3
	depends = hicolor-icon-theme
	optdepends = git: install addons from Git repositories
	provides = forever-addon-manager
	conflicts = forever-addon-manager
	options = !strip
	options = !debug
	source = LICENSE-$ver::$lic_url
	sha256sums = $lic_sum
	source_x86_64 = forever-addon-manager-$ver.deb::$deb_url
	sha256sums_x86_64 = $deb_sum

pkgname = forever-addon-manager-bin
EOF
echo "Updated PKGBUILD and .SRCINFO for $ver"
