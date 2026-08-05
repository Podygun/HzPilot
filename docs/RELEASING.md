# Releasing

HzPilot uses semantic version tags and GitHub Actions to build release artifacts. Public releases should only be created from `main` after CI and manual hardware checks pass.

## Prepare

1. Update `package.version` in `Cargo.toml`.
2. Run `cargo check` to refresh `Cargo.lock` if needed.
3. Move relevant entries from `Unreleased` in `CHANGELOG.md` into a versioned section.
4. Run the full checklist in [TESTING.md](TESTING.md).
5. Merge the release pull request into `main` using squash merge.

## Publish

Create and push a matching tag:

```powershell
git tag -a v0.1.0 -m "HzPilot 0.1.0"
git push origin v0.1.0
```

The release workflow verifies that the tag matches `Cargo.toml`, builds the optimized executable, compiles the Inno Setup installer, creates a portable ZIP, generates SHA-256 checksums, and publishes a GitHub release.

## Signing

The current workflow produces unsigned artifacts. Do not describe an unsigned build as trusted or production-ready. Add Authenticode signing before promoting HzPilot to a stable release or submitting it to WinGet.

## Rollback

Do not move or reuse a published tag. If a release is defective, document the issue, publish a corrective patch version, and mark the affected release as unavailable or pre-release when appropriate.
