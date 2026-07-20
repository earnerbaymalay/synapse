# LLM Neurosurgeon — Release Packaging & Distribution Guide

This document specifies the release packaging configurations, installer scripts, platform bundlers, code signing workflows, and package manager formulae for **LLM Neurosurgeon** (Desktop GUI and CLI engine).

---

## 1. Overview & Distribution Strategy

LLM Neurosurgeon is distributed in two primary forms:
1. **Desktop GUI Application**: A Tauri v2 application shipping native installers for macOS (`.dmg`), Windows (`.msi`), and Linux (`.AppImage`, `.deb`).
2. **CLI Engine (`neurosurgeon`)**: A standalone, zero-dependency Rust binary distributed via Homebrew, shell installer scripts (`install.sh`, `install.ps1`), and direct release tarballs/zip archives.

---

## 2. Tauri v2 Bundle Configuration

Below is the production `bundle` configuration block for `apps/desktop/src-tauri/tauri.conf.json` targeting Tauri v2:

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "LLM Neurosurgeon",
  "version": "0.7.4",
  "identifier": "dev.llmneurosurgeon.desktop",
  "build": {
    "frontendDist": "../dist"
  },
  "app": {
    "security": {
      "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:;"
    },
    "windows": [
      {
        "title": "LLM Neurosurgeon",
        "width": 1280,
        "height": 800,
        "resizable": true,
        "fullscreen": false
      }
    ]
  },
  "bundle": {
    "active": true,
    "targets": ["dmg", "msi", "appimage", "deb"],
    "identifier": "dev.llmneurosurgeon.desktop",
    "publisher": "LLM Neurosurgeon Core Team",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "resources": [],
    "copyright": "Copyright © 2026 LLM Neurosurgeon Contributors",
    "category": "DeveloperTool",
    "shortDescription": "Universal agent harness and multi-tool config migration engine",
    "longDescription": "LLM Neurosurgeon provides deterministic sync, conflict resolution, security sandboxing, and drift detection across 12 AI developer tools.",
    "macOS": {
      "frameworks": [],
      "minimumSystemVersion": "10.15.0",
      "exceptionDomain": "",
      "signing": {
        "identity": "Developer ID Application: LLM Neurosurgeon LLC (XYZ1234567)"
      },
      "entitlements": "entitlements.plist",
      "dmg": {
        "background": "assets/dmg_background.png",
        "windowSize": {
          "width": 660,
          "height": 400
        },
        "appPosition": {
          "x": 180,
          "y": 170
        },
        "applicationFolderPosition": {
          "x": 480,
          "y": 170
        }
      }
    },
    "windows": {
      "certificateThumbprint": null,
      "digestAlgorithm": "sha256",
      "timestampUrl": "http://timestamp.digicert.com",
      "wix": {
        "upgradeCode": "e89c6f2a-5b12-4d39-9f1e-7b89c0d1e2f3",
        "language": ["en-US"],
        "bannerPath": "assets/wix_banner.bmp",
        "dialogImagePath": "assets/wix_dialog.bmp"
      }
    },
    "linux": {
      "deb": {
        "depends": [
          "libc6 (>= 2.31)",
          "libgtk-3-0 (>= 3.24.0)",
          "libwebkit2gtk-4.1-0 (>= 2.38.0)",
          "libayatana-appindicator3-1"
        ],
        "desktopTemplate": "assets/llm-neurosurgeon.desktop",
        "section": "devel",
        "priority": "optional"
      },
      "appimage": {
        "bundleMediaFramework": false
      }
    }
  }
}
```

---

## 3. Platform Installer Specifications

### 3.1 macOS Package (.dmg) & Notarization
- **Artifact**: `LLM_Neurosurgeon_0.7.4_x64.dmg`, `LLM_Neurosurgeon_0.7.4_aarch64.dmg`
- **Signing**: Signed via `codesign` using Apple Developer ID Application certificate.
- **Notarization Workflow**:
  ```bash
  # Submit DMG for Apple Notarization
  xcrun notarytool submit target/release/bundle/dmg/LLM_Neurosurgeon_0.7.4_aarch64.dmg \
    --apple-id "$APPLE_ID" \
    --team-id "$APPLE_TEAM_ID" \
    --password "$APPLE_APP_SPECIFIC_PASSWORD" \
    --wait

  # Staple ticket to DMG
  xcrun stapler staple target/release/bundle/dmg/LLM_Neurosurgeon_0.7.4_aarch64.dmg
  ```

### 3.2 Windows Package (.msi) & Authenticode
- **Artifact**: `LLM_Neurosurgeon_0.7.4_x64_en-US.msi`
- **Engine**: WiX Toolset v3/v4 wrapped by Tauri CLI.
- **Signing**: Authenticode signing via Azure Key Vault / Signtool:
  ```powershell
  signtool.exe sign `
    /tr http://timestamp.digicert.com `
    /td sha256 `
    /fd sha256 `
    /sha1 "$CODE_SIGNING_CERT_THUMBPRINT" `
    target/release/bundle/msi/LLM_Neurosurgeon_0.7.4_x64_en-US.msi
  ```

### 3.3 Linux Debian Package (.deb)
- **Artifact**: `llm-neurosurgeon_0.7.4_amd64.deb`
- **Dependencies**: `libc6`, `libgtk-3-0`, `libwebkit2gtk-4.1-0`, `libayatana-appindicator3-1`.
- **Maintainer Scripts**:
  - `postinst`: Registers MIME types and updates desktop database (`update-desktop-database -q`).
  - `postrm`: Deletes desktop caches on removal.

### 3.4 Linux AppImage
- **Artifact**: `LLM_Neurosurgeon_0.7.4_amd64.AppImage`
- **Portable Design**: Self-contained runtime bundling standard webkit dependencies, using `AppRun` script launcher.

---

## 4. Universal Shell Installer Script (`install.sh`)

POSIX-compliant shell installer script for CLI deployment on Linux and macOS:

```bash
#!/bin/sh
# LLM Neurosurgeon CLI Installer
# Usage: curl -fsSL https://raw.githubusercontent.com/llm-neurosurgeon/llm-neurosurgeon/main/scripts/install.sh | sh

set -e

REPO="llm-neurosurgeon/llm-neurosurgeon"
VERSION="${VERSION:-latest}"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

detect_platform() {
  OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
  ARCH="$(uname -m)"

  case "$OS" in
    linux) OS="unknown-linux-gnu" ;;
    darwin) OS="apple-darwin" ;;
    *) echo "Error: Unsupported OS $OS"; exit 1 ;;
  esac

  case "$ARCH" in
    x86_64|amd64) ARCH="x86_64" ;;
    arm64|aarch64) ARCH="aarch64" ;;
    *) echo "Error: Unsupported architecture $ARCH"; exit 1 ;;
  esac

  TARGET="${ARCH}-${OS}"
}

fetch_release() {
  detect_platform
  echo "Installing LLM Neurosurgeon CLI ($TARGET)..."

  if [ "$VERSION" = "latest" ]; stream
    DOWNLOAD_URL="https://github.com/${REPO}/releases/latest/download/neurosurgeon-${TARGET}.tar.gz"
    CHECKSUM_URL="https://github.com/${REPO}/releases/latest/download/SHA256SUMS"
  else
    DOWNLOAD_URL="https://github.com/${REPO}/releases/download/v${VERSION}/neurosurgeon-${TARGET}.tar.gz"
    CHECKSUM_URL="https://github.com/${REPO}/releases/download/v${VERSION}/SHA256SUMS"
  fi

  TMP_DIR="$(mktemp -d)"
  trap 'rm -rf "$TMP_DIR"' EXIT

  echo "Downloading $DOWNLOAD_URL..."
  curl -sSL "$DOWNLOAD_URL" -o "$TMP_DIR/neurosurgeon.tar.gz"
  curl -sSL "$CHECKSUM_URL" -o "$TMP_DIR/SHA256SUMS"

  echo "Verifying SHA-256 checksum..."
  (cd "$TMP_DIR" && grep "neurosurgeon-${TARGET}.tar.gz" SHA256SUMS | sha256sum -c -)

  echo "Extracting binary..."
  tar -xzf "$TMP_DIR/neurosurgeon.tar.gz" -C "$TMP_DIR"

  echo "Installing to $INSTALL_DIR..."
  if [ -w "$INSTALL_DIR" ]; then
    mv "$TMP_DIR/neurosurgeon" "$INSTALL_DIR/neurosurgeon"
  else
    sudo mv "$TMP_DIR/neurosurgeon" "$INSTALL_DIR/neurosurgeon"
  fi

  chmod +x "$INSTALL_DIR/neurosurgeon"
  echo "Successfully installed neurosurgeon to $INSTALL_DIR/neurosurgeon"
  "$INSTALL_DIR/neurosurgeon" --version
}

fetch_release
```

---

## 5. Windows PowerShell Installer Script (`install.ps1`)

PowerShell script for automated Windows CLI installation:

```powershell
# LLM Neurosurgeon CLI Windows Installer
# Usage: iwr -useb https://raw.githubusercontent.com/llm-neurosurgeon/llm-neurosurgeon/main/scripts/install.ps1 | iex

$ErrorActionPreference = 'Stop'

$Repo = "llm-neurosurgeon/llm-neurosurgeon"
$InstallDir = "$env:LOCALAPPDATA\Programs\LLMNeurosurgeon\bin"

function Install-Neurosurgeon {
    $Arch = if ([Environment]::Is64BitOperatingSystem) { "x86_64" } else { throw "32-bit Windows is not supported." }
    $Target = "$Arch-pc-windows-msvc"
    
    $DownloadUrl = "https://github.com/$Repo/releases/latest/download/neurosurgeon-$Target.zip"
    $ChecksumUrl = "https://github.com/$Repo/releases/latest/download/SHA256SUMS"

    $TmpDir = New-Item -ItemType Directory -Path ([System.IO.Path]::GetTempPath() + [System.Guid]::NewGuid().ToString())
    try {
        Write-Host "Downloading LLM Neurosurgeon CLI ($Target)..." -ForegroundColor Cyan
        Invoke-WebRequest -Uri $DownloadUrl -OutFile "$TmpDir\neurosurgeon.zip"
        Invoke-WebRequest -Uri $ChecksumUrl -OutFile "$TmpDir\SHA256SUMS"

        Write-Host "Verifying checksum..." -ForegroundColor Cyan
        $ExpectedHash = (Get-Content "$TmpDir\SHA256SUMS" | Select-String "neurosurgeon-$Target.zip").Line.Split(" ")[0]
        $ActualHash = (Get-FileHash "$TmpDir\neurosurgeon.zip" -Algorithm SHA256).Hash.ToLower()

        if ($ExpectedHash -ne $ActualHash) {
            throw "Checksum verification failed! Expected: $ExpectedHash, Got: $ActualHash"
        }

        Write-Host "Extracting binary..." -ForegroundColor Cyan
        Expand-Archive -Path "$TmpDir\neurosurgeon.zip" -DestinationPath $TmpDir -Force

        if (!(Test-Path $InstallDir)) {
            New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
        }

        Move-Item -Path "$TmpDir\neurosurgeon.exe" -Destination "$InstallDir\neurosurgeon.exe" -Force

        # User PATH updates
        $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
        if ($UserPath -notlike "*$InstallDir*") {
            [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
            Write-Host "Added $InstallDir to User PATH." -ForegroundColor Green
        }

        Write-Host "LLM Neurosurgeon CLI successfully installed to $InstallDir\neurosurgeon.exe" -ForegroundColor Green
    }
    finally {
        Remove-Item -Recurse -Force $TmpDir
    }
}

Install-Neurosurgeon
```

---

## 6. Homebrew Formula Draft (`llm-neurosurgeon.rb`)

Formula draft for Homebrew deployment (`brew install llm-neurosurgeon`):

```ruby
class LlmNeurosurgeon < Formula
  desc "Universal agent harness & multi-tool config migration engine"
  homepage "https://github.com/llm-neurosurgeon/llm-neurosurgeon"
  url "https://github.com/llm-neurosurgeon/llm-neurosurgeon/releases/download/v0.7.4/neurosurgeon-src-v0.7.4.tar.gz"
  sha256 "a1b2c3d4e5f67890123456789abcdef0123456789abcdef0123456789abcdef0"
  license "MIT"
  head "https://github.com/llm-neurosurgeon/llm-neurosurgeon.git", branch: "main"

  bottle do
    root_url "https://github.com/llm-neurosurgeon/llm-neurosurgeon/releases/download/v0.7.4"
    sha256 cellar: :any_skip_relocation, arm64_sequoia: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    sha256 cellar: :any_skip_relocation, ventura:       "123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0"
    sha256 cellar: :any_skip_relocation, x86_64_linux:  "23456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef01"
  end

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: "apps/cli")
  end

  test do
    assert_match "neurosurgeon #{version}", shell_output("#{bin}/neurosurgeon --version")
    
    # Smoke test scan command on empty directory
    test_dir = testpath/"test_project"
    test_dir.mkpath
    output = shell_output("#{bin}/neurosurgeon scan --json", 0)
    assert_match "detected_tools", output
  end
end
```

---

## 7. Build Verification Checklist

Before publishing any release package:
- [ ] All `.dmg`, `.msi`, `.deb`, and `.AppImage` bundles pass automated smoke tests.
- [ ] Codesign signatures on `.dmg` and `.msi` are validated via `spctl` and `signtool`.
- [ ] `SHA256SUMS` file signed with release PGP / minisign key.
- [ ] Homebrew formula passes `brew audit --strict --online llm-neurosurgeon.rb`.
- [ ] Install scripts tested on fresh macOS (ARM64/x86_64), Ubuntu 22.04/24.04, and Windows 11.
