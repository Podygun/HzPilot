# Contributing

Focused issues and pull requests are welcome. By participating, you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Workflow

1. Search existing issues and discussions before proposing a change.
2. Open an issue for behavior changes that affect display safety, installation, privileges, or project scope.
3. Branch from `main` using a concise prefix such as `feat/`, `fix/`, `docs/`, or `build/`.
4. Keep commits focused and use Conventional Commit-style subjects, for example `fix: preserve display orientation`.
5. Open a pull request and complete its validation and hardware sections.

The project uses squash merge to keep `main` linear. Do not force-push shared branches.

## Development

HzPilot targets Windows 10 and Windows 11 with Rust 1.85 or newer and the MSVC toolchain. Install Visual Studio Build Tools with the Windows SDK before building.

Before opening a pull request, run:

```powershell
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo +1.85.0 check --locked
cargo build --release --locked
```

Changes to display-mode filtering or application must be tested on real hardware. Include the Windows version, GPU, driver version, monitor topology, resolution, and tested refresh rates in the pull request. See [Testing](docs/TESTING.md) for the complete checklist.

## Design constraints

Do not add telemetry, network access, elevation requirements, or custom/raw monitor modes without prior discussion.

Prefer the smallest correct implementation. New dependencies require a concrete benefit and must be compatible with the MIT license and the minimum supported Rust version.

## License

By contributing, you agree that your contribution is licensed under the project's [MIT License](LICENSE).
