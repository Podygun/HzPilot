# Security Policy

## Reporting a vulnerability

Please do not disclose suspected vulnerabilities in a public issue or discussion.

Use [GitHub private vulnerability reporting](https://github.com/Podygun/HzPilot/security/advisories/new) and include:

- The affected HzPilot version or commit.
- Reproduction steps and expected impact.
- Relevant Windows, GPU, driver, and monitor details.
- Any proposed mitigation, if available.

The maintainer will acknowledge a complete report as soon as practical, coordinate validation, and publish a security advisory when a fix is ready. Reports made in good faith will be handled respectfully.

## Supported versions

Before the first stable release, only the latest commit on `main` is supported. After releases begin, the latest minor release will receive security fixes.

## Scope

Security-sensitive areas include display-mode validation, installer behavior, startup registration, release artifacts, and the GitHub Actions supply chain. HzPilot must not require elevation, collect telemetry, or make network requests at runtime.
