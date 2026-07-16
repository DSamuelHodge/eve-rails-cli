# Eve Rails CLI

[![CI](https://github.com/DSamuelHodge/eve-rails-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/DSamuelHodge/eve-rails-cli/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.97%2B-orange.svg)](https://www.rust-lang.org/)
[![Eve](https://img.shields.io/badge/Eve-0.24%2B-black.svg)](https://www.npmjs.com/package/eve)

Rails-inspired conventions for building, composing, testing, approving, versioning, deploying, and operating Eve agent fleets.

Eve Rails CLI turns agent development into a predictable software workflow: YAML manifests describe the fleet, Jinja-compatible templates render Eve-compatible agents, lockfiles capture component versions, and doctor/deploy gates keep changes reviewable before they reach production.

## Why It Matters

Agent teams need more than prompts in folders. They need repeatable defaults, reusable components, eval gates, approval policy, memory conventions, hot-load rules, rollback plans, and diagnostics that a human or a future model can follow. Eve Rails CLI provides that backbone so teams can generate and operate many agents without each one becoming a bespoke project.

## Current Status

The CLI core is implemented in Rust and has been validated against real Eve local development flows.

Implemented:

- Manifest loading and catalog-aware validation.
- Strict Jinja-compatible rendering through `minijinja`.
- Agent, tool, skill, eval, approval, memory, and channel generators.
- Batch plan/apply reporting.
- Structured `doctor` diagnostics.
- Deterministic version resolution and lockfiles.
- Update, hot-load, migration, deploy preflight, and rollback planning.
- `inspect` and `graph` views for fleet composition.
- Eve-compatible generated example agents under `examples/basic-fleet`.

Validated:

- `cargo test`: 31 passing tests.
- Generated `support` agent starts with `eve dev`.
- Eve session creation and streaming work locally.
- Vercel AI Gateway model call completed on `openai/gpt-5.5`.

## Install

### From Source

```sh
git clone https://github.com/DSamuelHodge/eve-rails-cli.git
cd eve-rails-cli
cargo install --path .
eve-rails-cli --help
```

### For Development

```sh
git clone https://github.com/DSamuelHodge/eve-rails-cli.git
cd eve-rails-cli
cargo test
cargo run -- --help
```

## Requirements

- Rust 1.97 or newer.
- Node.js 24 or newer for real Eve integration checks.
- Eve CLI through npm for generated-agent smoke tests.
- Optional: `AI_GATEWAY_API_KEY` or `eve link` credentials for live model calls.

## Quick Start

Run against the bundled example fleet:

```sh
cargo run -- plan examples/basic-fleet/manifests/agents.yml \
  --catalog examples/basic-fleet/manifests/catalog.yml

cargo run -- doctor --all \
  --manifest examples/basic-fleet/manifests/agents.yml \
  --catalog examples/basic-fleet/manifests/catalog.yml

cargo run -- inspect --agent support \
  --manifest examples/basic-fleet/manifests/agents.yml \
  --catalog examples/basic-fleet/manifests/catalog.yml

cargo run -- graph --all \
  --manifest examples/basic-fleet/manifests/agents.yml \
  --catalog examples/basic-fleet/manifests/catalog.yml \
  --format mermaid
```

Test the generated Eve example:

```sh
cd examples/basic-fleet/agents/support
npm install
npm run typecheck
npm exec -- eve info --json
npm exec -- eve dev --no-ui
```

For a live model call, configure Vercel AI Gateway credentials first:

```sh
export AI_GATEWAY_API_KEY="..."
# or
npm exec -- eve link
```

## Repository Layout

```txt
src/                         Rust CLI implementation
templates/agent/             Reusable agent rendering templates
examples/basic-fleet/        Runnable demo fleet and generated Eve agents
catalog/                     Reserved catalog package area
SPEC.md                      Implementation spec, PR plan, checklists, metrics
RAILS_PHILOSOPHY_FOR_EVE.md  Product and framework philosophy
.github/                     CI, issue templates, and PR template
```

The CLI defaults still assume a user's own project has root-level `manifests/` and `agents/` directories. This repository keeps demo assets under `examples/basic-fleet/` to avoid confusing generated examples with framework source.

## Common Commands

```sh
eve-rails-cli --help
eve-rails-cli plan manifests/agents.yml
eve-rails-cli render --all --check
eve-rails-cli doctor --all
eve-rails-cli generate agent support --dry-run
eve-rails-cli outdated --agent support
eve-rails-cli update --agent support --minor
eve-rails-cli hotload --agent support --skill handle_refund@2.0.1
eve-rails-cli deploy --agent support --env staging --require-evals --require-doctor
eve-rails-cli rollback --agent support --to 1.3.2
eve-rails-cli inspect --agent support
eve-rails-cli graph --all --format mermaid
```

## Contributing

Contributions are welcome. Start with [CONTRIBUTING.md](CONTRIBUTING.md), use the issue templates for bugs/features, and open pull requests with the checklist in [PULL_REQUEST_TEMPLATE.md](.github/PULL_REQUEST_TEMPLATE.md).

## Contributors

Eve Rails CLI is currently maintained by Derrick Hodge. Contributors are welcome through issues and pull requests; the project uses GitHub issue forms and a PR checklist to keep discussion, review, tests, and release notes consistent.

## CI/CD

Continuous integration runs on pushes and pull requests to `main`:

- `cargo fmt --check`
- `cargo test --locked`
- `cargo build --locked`
- Example Eve agent install, typecheck, and `eve info`

Release automation runs when a `v*` tag is pushed. It builds Linux and macOS binaries and publishes them to a GitHub Release.

## Releases

Release notes are tracked in [CHANGELOG.md](CHANGELOG.md). Version `1.0.0` is planned as the first stable release once the CLI has packaged binaries, CI-published artifacts, and documented compatibility guarantees.

To prepare the future `1.0.0` release:

```sh
cargo test --locked
git tag v1.0.0
git push origin v1.0.0
```

## License

MIT. See [LICENSE](LICENSE).
