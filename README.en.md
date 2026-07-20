<div align="center">

# Koofr-GUI

**Windows-first desktop client for Koofr**

Built with Tauri v2 + React + TypeScript + Rust. Aims to feel like a native file manager, not a web wrapper.

[![Release](https://img.shields.io/github/v/release/zenwinhow/Koofr-GUI?include_prereleases&sort=semver)](https://github.com/zenwinhow/Koofr-GUI/releases)
[![CI](https://img.shields.io/github/actions/workflow/status/zenwinhow/Koofr-GUI/release.yml?branch=main&label=release)](https://github.com/zenwinhow/Koofr-GUI/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-0078D6?logo=windows)](#requirements)
[![Tauri](https://img.shields.io/badge/Tauri-v2-24C8DB?logo=tauri)](https://v2.tauri.app/)

[中文](README.md) · [Download](https://github.com/zenwinhow/Koofr-GUI/releases) · [Building](docs/BUILDING.md) · [Architecture](docs/ARCHITECTURE.md) · [Changelog](CHANGELOG.md)

</div>

---

## Table of contents

- [Koofr-GUI](#koofr-gui)
  - [Table of contents](#table-of-contents)
  - [Overview](#overview)
  - [Screenshots](#screenshots)
  - [Features](#features)
    - [Working today](#working-today)
    - [Not yet](#not-yet)
  - [Download](#download)
  - [Requirements](#requirements)
    - [Running](#running)
    - [Building from source](#building-from-source)
  - [Building from source](#building-from-source-1)
  - [Common commands](#common-commands)
  - [Architecture \& security boundary](#architecture--security-boundary)
  - [Where data is stored](#where-data-is-stored)
  - [Roadmap](#roadmap)
  - [Contributing](#contributing)
  - [Security](#security)
  - [License](#license)
  - [Acknowledgements](#acknowledgements)

## Overview

[Koofr](https://koofr.eu) is a European cloud storage provider that aggregates Google Drive, OneDrive, Dropbox and other third-party accounts under one login. Its web UI is not always smooth from mainland China, and there is no official Windows desktop client.

Koofr-GUI aims to fill that gap: a small, native-feeling desktop client with resumable large-file transfers, that does not stuff tokens into a WebView. The current release is **1.3.3**, covering the main Koofr file-management flows, resumable transfers and public-link management. Koofr Vault (end-to-end encryption) is not implemented yet.

## Screenshots

> Screenshots pending. UI targets 1280×720 desktop with an 980×640 minimum, following the design tokens in [DESIGN.md](DESIGN.md).

## Features

### Working today

- **Sign-in & credentials**
  - Sign in with a Koofr app-specific password. Session restore on next launch; explicit sign-out.
  - Optional password persistence through the Windows Credential Manager. Never written to plain config files or WebView storage.

- **File browsing & operations**
  - Browse mounts, folders, recents, shared items and trash.
  - Detects existing Koofr, Google Drive, OneDrive, Dropbox and other connected storage mounts.
  - Create folder, upload, download, rename, move, copy, delete, restore from trash.
  - Public links: list, create, revoke download and file-receive links (revoke requires a second click).

- **Transfers**
  - Live transfer panel with progress, cancel and pause.
  - **Single-file download**: HTTP Range + on-disk checkpoint for true byte-level resume.
  - **Split upload**: chops a large file into user-sized `part-*.bin` chunks in a user-named remote folder. Resumes from the last confirmed complete chunk. Parts contain no proprietary header — reassemble with `copy /b` on Windows or `cat` on POSIX.
  - **Recursive folder download**: staged into a temporary directory, cleaned up on failure or cancel.

- **Settings**
  - Configurable default download folder, optional "ask each time" per-download prompt.
  - Metadata cache: memory / disk / off.
  - Optional automatic retry for transfers that fail with `network_error`, with a configurable fixed interval and either a finite or unlimited retry count.
  - Five themes (koofr / ocean / iris / coral / berry) — accent tokens only.

### Not yet

- Koofr Vault unlock, encrypt/decrypt, Vault transfers (`src-tauri/src/crypto/` and `vault_core/` are placeholders).
- OAuth sign-in and add/remove/re-authorize third-party mounts. Blocked on Koofr publishing desktop client registration info and a public authorization API. Today the app can list existing mounts and link out to the official Koofr account page.
- macOS / Linux support.

> **Why can't regular uploads resume?** Koofr's public upload API only exposes whole-file `FilesPut` — no chunk sessions, no server-side confirmed offsets. So a broken regular upload has to restart from zero. Split upload is the deliberate interop escape hatch. See [ARCHITECTURE.md](docs/ARCHITECTURE.md#split-upload-design) for details.

## Download

Grab the latest NSIS installer from the [Releases page](https://github.com/zenwinhow/Koofr-GUI/releases) (`Koofr-GUI_x.y.z_x64-setup.exe`).

> ⚠️ **Unsigned binaries**: releases are not code-signed. Windows SmartScreen may warn about an "unknown publisher". **Only download from this repository's Releases page** — never third-party mirrors. See [RELEASING.md](docs/RELEASING.md) for verification.

First launch needs Microsoft Edge WebView2 Runtime, which is usually pre-installed on supported Windows 10 / 11.

## Requirements

### Running

- Windows 10 or 11 (x64)
- Microsoft Edge WebView2 Runtime (usually pre-installed)
- A Koofr account with an [app-specific password](https://app.koofr.net/app-password)

### Building from source

- Node.js **24 LTS** (recommended; 22.12+ 22.x LTS also works)
- npm **10+**
- Rust **1.88+** with the `x86_64-pc-windows-msvc` toolchain
- Visual Studio 2022 Build Tools with the "Desktop development with C++" workload + Windows SDK

Full install instructions in [BUILDING.md](docs/BUILDING.md#1-环境要求).

## Building from source

```powershell
git clone https://github.com/zenwinhow/Koofr-GUI.git
Set-Location Koofr-GUI
npm ci
npm run dev:desktop          # dev mode: full desktop app
```

Frontend-only dev server (no Tauri commands):

```powershell
npm run dev
```

Release builds:

```powershell
npm run check                # full check: lint + tests + Rust fmt/clippy
npm run build:desktop        # produces src-tauri/target/release/koofr-gui.exe
npm run build:installer      # explicitly build the NSIS installer
```

See [BUILDING.md](docs/BUILDING.md) for the full walk-through and troubleshooting.

## Common commands

| Command | What it does |
| --- | --- |
| `npm ci` | Install pinned deps from `package-lock.json` |
| `npm run dev` | Vite frontend dev server only (browser) |
| `npm run dev:desktop` | Full Tauri desktop dev environment |
| `npm run build` | Type-check + emit `dist/` frontend bundle |
| `npm run build:desktop` | Build the release exe, no installer |
| `npm run build:installer` | Explicitly build the NSIS installer |
| `npm run verify:quick` | Fast gate: lint + split-upload tests + frontend build |
| `npm run check` | Full gate: lint + tests + build + Rust fmt + clippy + Rust tests |
| `npm run clean` | Clean build artifacts, keep `node_modules/` |
| `npm run clean:all` | Also wipe `node_modules/` — need `npm ci` afterwards |

## Architecture & security boundary

```
┌──────────────────────────────────────────────┐
│ React + TypeScript UI (src/)                 │
│ - only calls narrowly-typed Tauri commands   │
│ - never touches passwords, tokens, Safe Keys │
└──────────────┬───────────────────────────────┘
               │ typed Tauri commands + events
               ▼
┌──────────────────────────────────────────────┐
│ Rust + Tauri core (src-tauri/src/)           │
│ ├─ file_ops/           path / op validation  │
│ ├─ transfer/           up/down + resume      │
│ ├─ koofr_api/          Koofr REST client     │
│ ├─ credential_manager  Windows Credentials   │
│ ├─ metadata_cache      memory / disk cache   │
│ ├─ crypto/             reserved (Vault)      │
│ └─ vault_core/         reserved (Vault)      │
└──────────────────────────────────────────────┘
```

Rules that matter:

- **Credentials never cross the Rust boundary.** Session tokens live in memory only; the optional app-password persistence uses the Windows Credential Manager.
- **Paths are validated.** Remote paths reject `.`, `..`, NUL, and overlong names. Download parents must be absolute, existing, non-symlink directories.
- **Never overwrite.** Single-file downloads land in a `.koofr-part-*` staging file. Same-directory name clashes get a stable suffix.
- **Error messages leak nothing** — stable error code + safe message. No paths, tokens, or response bodies.

Full design and data flow in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Where data is stored

App data lives under the current Windows user's local data directory (`identifier = net.koofr.desktop.gui`):

```
%LOCALAPPDATA%\net.koofr.desktop.gui\
├─ settings.json                # app settings
├─ transfer-checkpoints.json    # resumable transfer checkpoints
├─ cache/metadata-cache.json    # default disk cache location (configurable)
└─ logs/koofr-gui*.jsonl       # redacted diagnostic logs (configurable)
```

**Credentials are NOT here** — the Koofr app-specific password is stored in the Windows Credential Manager and survives reinstalls and rebuilds.

## Roadmap

- [ ] Koofr Vault unlock + encrypt/decrypt + Vault transfers (rclone crypt compatible)
- [ ] OAuth sign-in & third-party mount management (blocked on public Koofr desktop API)
- [ ] Code signing / SmartScreen reputation
- [ ] macOS support
- [ ] Linux support

## Contributing

Issues and pull requests welcome. Before opening a PR:

1. Read [CONTRIBUTING.md](CONTRIBUTING.md).
2. Run `npm run check` — lint, tests, Rust fmt and clippy must be green.
3. Any new Tauri command must validate paths, identifiers and operation scope on the Rust side.

## Security

**Do not** file public issues for vulnerabilities. Use the private channel in [SECURITY.md](SECURITY.md).

## License

[MIT](LICENSE) © 2026 Koofr-GUI contributors

This project is **not affiliated** with Koofr d.o.o. "Koofr" is a trademark of [Koofr d.o.o.](https://koofr.eu).

## Acknowledgements

- [Koofr](https://koofr.eu) for the cloud storage service and their reference [Go client](https://github.com/koofr/go-koofrclient) / [Java SDK](https://github.com/koofr/java-koofr).
- The [Tauri](https://tauri.app) team for the desktop framework.
- [rclone](https://rclone.org) for the Koofr backend and crypt format — reference for split upload and future Vault compatibility.
- [Lucide](https://lucide.dev) for the icon set.
