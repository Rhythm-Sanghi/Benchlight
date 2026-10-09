# Contributing

Bug reports and focused pull requests are welcome. Before opening a pull request,
check existing issues and describe the behavior you want to change. For larger
changes, open an issue first so we can agree on the scope.

## Development

You need Windows 11 x86-64, stable Rust (at least 1.98), Visual Studio C++ build
tools with the Windows SDK, Node.js 24 and WebView2. From `apps/desktop`, run
`npm ci`, then `npm run tauri dev`. Tauri prepares the CLI sidecar automatically.

Run `./scripts/check.ps1` from the repository root before submitting a change.
It checks Rust formatting, Clippy, Rust tests, frontend formatting, lint, types,
contrast, frontend tests and the production UI build. Use `npm run tauri build`
in `apps/desktop` to build the Windows installer.

## Changes and tests

Keep filesystem analysis in Rust so the desktop and CLI share the same behavior.
Use temporary directories for filesystem tests. Never point cleanup tests at
personal project folders. Preserve explicit confirmation and revalidation when
changing cleanup code; see [the safety model](docs/safety-model.md).

Keep pull requests focused. Explain the problem, the resulting behavior, how you
tested it and any remaining limitations. Include screenshots for visible UI
changes. New features should keep working locally without accounts or telemetry.
Do not add credentials, local databases or generated build output to commits.

For vulnerabilities, follow [SECURITY.md](SECURITY.md) instead of opening a public
issue.
