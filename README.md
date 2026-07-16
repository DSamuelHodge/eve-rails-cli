# Eve Rails CLI

[![CI](https://github.com/DSamuelHodge/eve-rails-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/DSamuelHodge/eve-rails-cli/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.97%2B-orange.svg)](https://www.rust-lang.org/)
[![Eve](https://img.shields.io/badge/Eve-0.24%2B-black.svg)](https://www.npmjs.com/package/eve)

Rails-inspired conventions for building, composing, testing, approving, versioning, deploying, and operating Eve agent fleets.

Eve Rails CLI turns agent development into a predictable software workflow: YAML manifests describe the fleet, Jinja-compatible templates render Eve-compatible agents, lockfiles capture component versions, and doctor/deploy gates keep changes reviewable before they reach production.

## Why It Matters

Agent teams need more than prompts in folders. They need repeatable defaults, reusable components, eval gates, approval policy, memory conventions, hot-load rules, rollback plans, and diagnostics that a human or a future model can follow. Eve Rails CLI provides that backbone so teams can generate and operate many agents without each one becoming a bespoke project.

## Install

```sh
git clone https://github.com/DSamuelHodge/eve-rails-cli.git
cd eve-rails-cli
cargo install --path .
eve-rails-cli --help
```

## Requirements

- Rust 1.97 or newer.
- Node.js 24 or newer when testing generated Eve agents.
- Optional Vercel AI Gateway credentials for live model calls.

## Commands

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

## Repository Layout

```txt
src/                         Rust CLI implementation
templates/agent/             Reusable agent rendering templates
examples/basic-fleet/        Runnable demo fleet and generated Eve agents
catalog/                     Reserved catalog package area
SPEC.md                      Implementation spec
RAILS_PHILOSOPHY_FOR_EVE.md  Product and framework philosophy
.github/                     CI, release automation, issue forms, PR template
```

The CLI defaults assume a user's own project has root-level `manifests/` and `agents/` directories. This repository keeps demo assets under [`examples/basic-fleet`](examples/basic-fleet/README.md) so generated examples are not confused with framework source.

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md), [CHANGELOG.md](CHANGELOG.md), and the GitHub issue and pull request templates.
