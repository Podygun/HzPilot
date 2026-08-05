<div align="center">
  <img src="assets/hzpilot.png" width="112" alt="HzPilot icon">
  <h1>HzPilot</h1>
  <p><strong>A tiny native Windows tray utility for switching monitor refresh rates safely.</strong></p>
  <p>
    <a href="https://github.com/Podygun/HzPilot/actions/workflows/ci.yml"><img src="https://github.com/Podygun/HzPilot/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI"></a>
    <a href="LICENSE"><img src="https://img.shields.io/github/license/Podygun/HzPilot?color=0e7490" alt="MIT license"></a>
    <img src="https://img.shields.io/badge/Windows-10%20%7C%2011-0078d4" alt="Windows 10 and 11">
    <img src="https://img.shields.io/badge/Rust-1.85%2B-b7410e" alt="Rust 1.85 or newer">
  </p>
  <p>
    <a href="README.ru.md">Русский</a> ·
    <a href="#quick-start">Quick start</a> ·
    <a href="docs/ARCHITECTURE.md">Architecture</a> ·
    <a href="CONTRIBUTING.md">Contributing</a>
  </p>
</div>

HzPilot lives in the Windows notification area and exposes the refresh rates already supported by each active monitor and graphics driver. It uses direct Win32 APIs, ships as a roughly 200 KB native executable, and requires no administrator rights or separate runtime.

## Features

- Enumerates every active display with a friendly monitor name and exact current rate.
- Lists only driver-provided rates for the current resolution and mode family.
- Validates each selection with Windows before applying it.
- Supports extended desktops and mirrored display groups.
- Detects Windows 11 Dynamic Refresh Rate state.
- Restores its tray icon after Explorer restarts.
- Offers optional per-user startup.
- Uses the Windows display language for Russian or English UI.
- Performs no telemetry, runtime network access, or background service installation.

## At a glance

| | |
| --- | --- |
| Platform | Windows 10 1809+ and Windows 11 x64 |
| Runtime | None; native Rust and Win32 |
| Privileges | Current user, no UAC |
| Installation | Portable ZIP or per-user Inno Setup installer |
| License | MIT |

## Quick start

No public binary release has been published yet. To test the current MVP, build it from source:

```powershell
git clone https://github.com/Podygun/HzPilot.git
cd HzPilot
cargo build --release --locked
.\target\release\hzpilot.exe
```

The application has no main window. Windows may initially place the icon under the tray overflow arrow; only the user can pin it next to the clock.

| Action | Result |
| --- | --- |
| Left-click | Open displays and available refresh rates |
| Right-click | Startup, Windows display settings, About, and Exit |
| Hover | Show active displays and current rates |

## Safety model

HzPilot calls `EnumDisplaySettingsExW` without `EDS_RAWMODE`, filters results to the current resolution, color depth, orientation, fixed-output mode, and scan-line ordering, then validates the selected `DEVMODEW` with `CDS_TEST`. Only after validation does it call `ChangeDisplaySettingsExW`.

Raw and custom modes are never offered. HzPilot does not intentionally change resolution, scaling, HDR, topology, or monitor position. See [Architecture](docs/ARCHITECTURE.md) for the complete display pipeline and invariants.

The MVP controls fixed rates. Selecting a numeric rate while Windows 11 Dynamic Refresh Rate is active requests that fixed rate. Remote Desktop and virtual GPUs may expose no switchable modes.

## Build and test

Install Rust 1.85 or newer, Visual Studio Build Tools, and the Windows SDK. The resource build uses `rc.exe`.

```powershell
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo build --release --locked
```

Real display changes require manual hardware testing. Follow [Testing](docs/TESTING.md) before publishing a release.

## Project

- [Architecture](docs/ARCHITECTURE.md)
- [Testing guide](docs/TESTING.md)
- [Release process](docs/RELEASING.md)
- [Contributing](CONTRIBUTING.md)
- [Support](SUPPORT.md)
- [Security policy](SECURITY.md)
- [Changelog](CHANGELOG.md)

## License

HzPilot is available under the [MIT License](LICENSE).
