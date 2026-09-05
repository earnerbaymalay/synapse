# Quick Start — Build Installers

> [!WARNING]
> **This file was heavily rewritten from a fabricated draft** (wrong product
> name, wrong version number, a `.dmg`/`.msi`/signing/notarization pipeline
> that was never built, an `install.sh`/`install.ps1`/Homebrew formula that
> don't exist in this repo). What follows describes what `scripts/
> build-installers.sh` and `apps/desktop/src-tauri/tauri.conf.json` actually
> do today, with the removed aspirational material kept at the bottom under
> "Planned, not yet built" so the intent isn't lost.

## Prerequisites
- Rust 1.75+ (via rustup)
- Node.js 20+ and pnpm
- System libraries (Linux):
  ```bash
  sudo apt install libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libssl-dev libdbus-1-dev libjavascriptcoregtk-4.1-dev libsoup-3.0-dev
  ```
- A 512×512 RGBA source icon at `apps/desktop/src-tauri/icons/icon.png` (the
  build script generates the rest of the icon set from it)

macOS and Windows are not currently packaged — see "Planned, not yet built"
below.

## Build Everything
```bash
./scripts/build-installers.sh
```
This runs, in order: CLI build (`cargo build -p synapse --release`), desktop
frontend build (`pnpm build`), icon generation (`build-icons.py`), the Tauri
Rust build, and `pnpm tauri build` for the bundles. Flags: `--skip-cli`,
`--skip-desktop`, `--help`. See the script itself for the authoritative
option list — it's the source of truth, not this doc.

## Build Artifacts (today)
| What | Location |
|---|---|
| CLI binary | `target/release/synapse` (also builds a `neurosurgeon`-named binary from the same source, kept as a compatibility alias) |
| Desktop frontend | `apps/desktop/dist/` |
| Tauri binary | `apps/desktop/src-tauri/target/release/desktop-app` |
| Tauri bundles | `apps/desktop/src-tauri/target/release/bundle/{deb,appimage}/` |

Real `tauri.conf.json` bundle config: `productName` is `"SYNAPSE"`, `version`
tracks `apps/cli/Cargo.toml` (don't hardcode a version number in this doc —
it will go stale), `bundle.targets` is `["deb", "appimage"]` only. There is
no macOS or Windows bundle config, no code-signing config, and no
notarization step anywhere in the repo.

## CLI Only (no desktop)
```bash
cargo build -p synapse --release
sudo cp target/release/synapse /usr/local/bin/
```

---

## Planned, not yet built

Everything below was in this doc's original draft, presented as if it
already shipped. None of it exists in the repo today — no `.dmg`/`.msi`
bundling, no macOS/Windows signing config in `tauri.conf.json`, no
`install.sh`/`install.ps1`/Homebrew formula anywhere in the tree. Kept here
as a real plan for whoever picks up cross-platform release packaging, not as
a claim about current behavior:

- **macOS `.dmg`** with `codesign` + `xcrun notarytool` notarization,
  requiring an Apple Developer ID.
- **Windows `.msi`** via the WiX toolset (Tauri-bundled), with Authenticode
  signing via `signtool`/Azure Key Vault.
- **Shell installers** (`install.sh` for Linux/macOS, `install.ps1` for
  Windows) that fetch a release tarball, verify a `SHA256SUMS` file, and
  install to `$INSTALL_DIR`/`%LOCALAPPDATA%`.
- **A Homebrew formula** (`brew install <name>`) built via `cargo install`.
- **Checksum signing** of `SHA256SUMS` with a release PGP/minisign key.

When any of this actually lands, replace this section with the real
implementation details (and delete this note) rather than adding a second,
possibly-contradicting section — this doc has already drifted from reality
once.
