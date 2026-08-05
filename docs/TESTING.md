# Testing

## Automated checks

Run the same quality gate used by CI:

```powershell
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo +1.85.0 check --locked
cargo build --release --locked
```

Unit tests cover rational refresh-rate formatting, UTF-16 handling, registry command quoting, menu escaping, and fixed-size tray buffers.

## Manual hardware checks

Display switching must be tested on a local physical session. Record the Windows version, GPU, driver, monitor model, connection type, topology, resolution, and tested rates.

### Core checklist

- Launching creates one tray icon and no taskbar window.
- A second launch activates the existing instance and exits.
- The tooltip reports every active display and its current rate.
- Left-click lists only rates for the current resolution and mode family.
- The current rate is checked.
- Selecting another listed rate changes only the refresh rate.
- Right-click actions open settings, toggle startup, show About, and exit.
- Hot-plugging a display refreshes the menu.
- Restarting Explorer restores the icon.
- Installation and uninstallation work without UAC.

### Recommended matrix

- Windows 10 22H2 and current Windows 11.
- Intel, AMD, and NVIDIA graphics.
- Internal laptop panel, HDMI, and DisplayPort.
- Single, extended, and mirrored monitor configurations.
- Integer and fractional rates such as 60, 74.97, 100, 144, and 165 Hz.
- Windows 11 Dynamic Refresh Rate, when available.

## Known testing limits

CI can validate compilation and pure logic but cannot validate real monitor behavior. Remote Desktop and virtual GPUs often expose incomplete or immutable mode lists and are not suitable for release qualification.
