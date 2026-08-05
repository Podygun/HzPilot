# HzPilot

HzPilot is a lightweight Windows tray utility for switching monitor refresh rates without opening Windows Settings.

[Русская версия](README.ru.md)

## Features

- Lists every active display and the refresh rates exposed by its driver.
- Changes only the refresh rate while preserving resolution, orientation, color depth, and desktop layout.
- Validates a mode with Windows before applying it.
- Supports multi-monitor extended desktops and mirrored display groups.
- Restores its tray icon after Explorer restarts.
- Optional per-user startup, without administrator rights.
- Russian and English UI based on the Windows display language.
- No telemetry, network access, background service, or separate runtime.

## Requirements

- Windows 10 version 1809 or newer, or Windows 11.
- x64 processor.
- A graphics driver that exposes more than one refresh rate for the current resolution.

## Usage

1. Start `HzPilot.exe`.
2. Left-click the tray icon to open the refresh-rate menu.
3. Select a display and one of its supported rates.
4. Right-click the icon for startup, Windows display settings, About, and Exit.

Windows initially places new notification icons in the overflow area. Only the user can pin HzPilot next to the clock.

## Safety

HzPilot enumerates monitor-compatible modes with `EnumDisplaySettingsExW`, filters them to the current mode family, validates a selection with `CDS_TEST`, and only then calls `ChangeDisplaySettingsExW`. Raw or custom display modes are never offered.

The application controls fixed refresh rates. Windows 11 Dynamic Refresh Rate is detected and shown, but selecting a numeric rate requests that fixed rate. Remote Desktop and some virtual display drivers may expose no switchable modes.

## Install

Release artifacts provide:

- `HzPilot-<version>-windows-x64.zip` for portable use.
- `HzPilot-<version>-setup-x64.exe` for a per-user installation in `%LOCALAPPDATA%\Programs\HzPilot`.

Unsigned development builds can trigger a Microsoft Defender SmartScreen warning. Public signing is planned before a stable release.

## Build

Prerequisites:

- Rust 1.85 or newer with the `x86_64-pc-windows-msvc` target.
- Visual Studio Build Tools with the Windows SDK. The resource build requires `rc.exe`.

```powershell
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

The executable is written with Rust and direct Win32 APIs. There is no GUI framework. Run `tools/generate-icon.ps1` after changing the icon drawing script.

## License

[MIT](LICENSE)
