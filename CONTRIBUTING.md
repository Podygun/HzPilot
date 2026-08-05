# Contributing

Contributions are welcome through focused issues and pull requests.

## Development

HzPilot targets Windows 10 and Windows 11 with Rust 1.85 or newer and the MSVC toolchain. Install Visual Studio Build Tools with the Windows SDK before building.

Before opening a pull request, run:

```powershell
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

Changes to display-mode filtering or application must be tested on real hardware. Include the Windows version, GPU, driver version, monitor topology, resolution, and tested refresh rates in the pull request.

Do not add telemetry, network access, elevation requirements, or custom/raw monitor modes without prior discussion.
