# Contributing

Thanks for helping make Eve Rails CLI calmer, sharper, and more useful.

## Development Setup

```sh
git clone https://github.com/DSamuelHodge/eve-rails-cli.git
cd eve-rails-cli
cargo test
```

For Eve integration checks, install Node.js 24 or newer and run commands from an example agent:

```sh
cd examples/basic-fleet/agents/support
npm install
npm run typecheck
npm exec -- eve info --json
```

Live model calls require Vercel AI Gateway credentials through `AI_GATEWAY_API_KEY` or `eve link`.

## Branches

Use short, descriptive branch names:

```txt
feature/gateway-doctor
fix/render-check-paths
docs/readme-install
```

## Pull Requests

Before opening a PR:

- Run `cargo fmt --check`.
- Run `cargo test`.
- Update README, SPEC, examples, or changelog when behavior changes.
- Add or update tests for CLI behavior.
- Keep generated/runtime artifacts out of commits.

## Issue Handling

Use the GitHub issue templates:

- Bug report: broken behavior, regression, confusing error, or failed check.
- Feature request: new command, convention, flag, workflow, or integration.
- Task: docs, cleanup, release, CI, examples, or project maintenance.

Good issues include the command used, expected behavior, actual behavior, environment, and any relevant manifest snippet.

## Commit Style

Prefer concise imperative commit messages:

```txt
Add gateway doctor diagnostics
Move demo fleet into examples
Document release process
```

## Release Contributions

Release work should update:

- `Cargo.toml` version when appropriate.
- `CHANGELOG.md`.
- README install instructions if packaging changes.
- CI/release workflow files.
