# Architecture

HzPilot is a single-process, event-driven Windows desktop utility. It has no visible main window, service, network client, or GUI framework.

## Runtime model

The process owns one hidden top-level Win32 window. The window receives notification-area callbacks and system messages such as display changes, device changes, and Explorer restarts. A single-instance mutex prevents duplicate tray processes.

Win32 APIs can re-enter a window procedure while a modal menu or display operation is active. `WindowState` protects mutable application state with `RefCell` and defers display refreshes when re-entry occurs.

## Modules

| Module | Responsibility |
| --- | --- |
| `main.rs` | Process entry point and fatal-error boundary |
| `app.rs` | Window lifecycle, message routing, actions, and single-instance behavior |
| `display.rs` | Display discovery, mode filtering, validation, and application |
| `tray.rs` | Notification icon, native popup menus, tooltip, and notifications |
| `settings.rs` | Per-user startup registration in `HKCU` |
| `strings.rs` | Static Russian and English UI text |

## Display pipeline

1. `QueryDisplayConfig` and `DisplayConfigGetDeviceInfo` resolve active paths, friendly monitor names, exact rational refresh rates, and Windows 11 Dynamic Refresh Rate state.
2. `EnumDisplayDevicesW` enumerates active GDI display sources.
3. `EnumDisplaySettingsExW` enumerates monitor-compatible modes without `EDS_RAWMODE`.
4. Modes are restricted to the current resolution, color depth, orientation, fixed-output mode, and scan-line ordering.
5. A selected driver-provided `DEVMODEW` is validated with `CDS_TEST`.
6. Only after validation, `ChangeDisplaySettingsExW` applies and persists the mode for the current user.

Mirrored targets sharing one Windows display source are presented as a combined display group because Windows changes that source atomically.

## Safety invariants

- Never offer raw or custom display modes.
- Never change resolution, position, topology, scaling, or HDR intentionally.
- Never require administrator rights.
- Never perform runtime network requests or telemetry.
- Keep all Win32 ownership boundaries explicit and release tray, menu, registry, and kernel handles.
- Treat real-hardware mode changes as manual integration tests, not automated CI tests.

## Packaging

`winresource` embeds the application icon, metadata, compatibility manifest, and Per-Monitor V2 DPI declaration. Inno Setup produces a per-user installer under `%LOCALAPPDATA%\Programs\HzPilot`. GitHub Actions builds portable and installer artifacts from version tags.
