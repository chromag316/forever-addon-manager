# Development

Forever AddOn Manager is built with [Tauri 2](https://tauri.app): the window is plain
HTML/CSS/JS, the addon logic is Rust.

## Requirements
- [Rust](https://rustup.rs) (stable)
- [Node.js](https://nodejs.org) 20 or newer (only for the Tauri command-line tool)
- Linux only: `libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`
- `git` on PATH if you want to try Git installs

## Project layout
| Path | What's there |
|---|---|
| `ui/` | The window: `index.html`, `style.css`, `app.js` (no build step) |
| `ui/dev-mock.js` | Sample data, used only when `ui/` is opened in a normal browser |
| `src-tauri/src/lib.rs` | Tauri commands the window calls |
| `src-tauri/src/core/` | Addon logic: fingerprinting, CurseForge API, Git/folder/zip installs, saved state |
| `src-tauri/tauri.conf.json` | App name, version, window and installer settings |
| `src-tauri/icons/` | App icons, generated from `Resources/AppIcon-1024.png` |
| `Scripts/` | macOS Swift scripts that draw the app icon and other image helpers |
| `.github/workflows/build.yml` | CI: build, test and release |
| `packaging/aur/` | Arch Linux (AUR) package: `PKGBUILD`, `.SRCINFO`, `update.sh` |

## Run and build
```bash
npm install
npm run dev        # run the app in development mode
npm run build      # build installers for this system into src-tauri/target/release/bundle/
```

To work on the look of the window without the Rust side, serve `ui/` with any static web server
(for example `python3 -m http.server --directory ui`) and open it in a browser; `dev-mock.js`
supplies sample addons.

## Tests
```bash
cargo test --manifest-path src-tauri/Cargo.toml
```
Two extra tests are skipped by default:
- `real_addons_match_curseforge` compares fingerprints with real WoW Forever addons on this
  machine (macOS default install path).
- `network_smoke` calls the CurseForge API with your saved key.

Run them with `cargo test --manifest-path src-tauri/Cargo.toml -- --ignored`.

## Releasing
1. Set the new version in `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` and
   `package.json`, and update the download links in `README.md`.
2. Commit and push to `main`, and wait for CI to pass on macOS, Windows and Linux.
3. Tag and push: `git tag -a v1.2.3 -m "Forever AddOn Manager 1.2.3" && git push origin v1.2.3`.
   The tag must match the version in `tauri.conf.json`, or the release job stops.

CI then builds the installers (`Forever-AddOn-Manager-<version>-macOS.dmg`,
`…-Windows-x64-setup.exe`, `…-Linux-amd64.deb`, `…-Linux-x86_64.rpm`) and publishes the
release. It also deletes an older release with the same tag, and draft releases whose tag no
longer exists (deleting a tag turns its release into a hidden draft).

## App icon
`Scripts/make-list-icon.swift` (macOS) draws `Resources/AppIcon-1024.png`. Regenerate the app
icons from it with:
```bash
swift Scripts/make-list-icon.swift Resources/AppIcon-1024.png
npx tauri icon Resources/AppIcon-1024.png -o src-tauri/icons
```
Then delete the `android/`, `ios/` and Windows Store (`Square*`, `StoreLogo`) images it also
creates; only the desktop icons are used.

## Arch Linux (AUR)
`packaging/aur/` holds the `forever-addon-manager-bin` package. It installs the release's `.deb`,
lets pacman provide WebKitGTK (`webkit2gtk-4.1`), and installs `LICENSE` (fetched from the release
tag) to `/usr/share/licenses/`, as Arch requires for MIT-licensed packages. `.SRCINFO` is generated
by `update.sh`. There's no AppImage: it would have to bundle
WebKitGTK itself and comes out around 80 MB.

After publishing a release:
```bash
packaging/aur/update.sh 1.2.3        # sets pkgver, the .deb's sha256 and regenerates .SRCINFO
```
Then publish to the AUR from a clone of the AUR repository (needs an AUR account with your SSH key):
```bash
git clone ssh://aur@aur.archlinux.org/forever-addon-manager-bin.git
cp packaging/aur/PKGBUILD packaging/aur/.SRCINFO forever-addon-manager-bin/
cd forever-addon-manager-bin && git add PKGBUILD .SRCINFO && git commit -m "Update to 1.2.3" && git push
```
On an Arch machine, `makepkg -si` in `packaging/aur/` builds and installs the package locally, and
`makepkg --printsrcinfo` should match the generated `.SRCINFO`.
