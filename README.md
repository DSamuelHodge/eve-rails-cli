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

| Command | Example |
| --- | --- |
| ![help](https://img.shields.io/badge/help-reference-64748b) | `eve-rails-cli --help` |
| ![init](https://img.shields.io/badge/init-project-0ea5e9) | `eve-rails-cli init my-fleet --template customer-support --dry-run` |
| ![plan](https://img.shields.io/badge/plan-preview-2563eb) | `eve-rails-cli plan manifests/agents.yml` |
| ![render](https://img.shields.io/badge/render-files-7c3aed) | `eve-rails-cli render --all --check` |
| ![doctor](https://img.shields.io/badge/doctor-diagnostics-059669) | `eve-rails-cli doctor --all --env production --connections --budgets` |
| ![generate](https://img.shields.io/badge/generate-scaffold-f97316) | `eve-rails-cli generate agent support --dry-run` |
| ![schedule](https://img.shields.io/badge/schedule-cron-9333ea) | `eve-rails-cli generate schedule weekday_triage --schedule "0 9 * * 1-5"` |
| ![outdated](https://img.shields.io/badge/outdated-updates-eab308) | `eve-rails-cli outdated --agent support` |
| ![update](https://img.shields.io/badge/update-versions-0891b2) | `eve-rails-cli update --agent support --minor` |
| ![hotload](https://img.shields.io/badge/hotload-compatible-db2777) | `eve-rails-cli hotload --agent support skill:summarize_thread@1.0.1` |
| ![eval](https://img.shields.io/badge/eval-delegate-9333ea) | `eve-rails-cli eval --agent support --dry-run` |
| ![test](https://img.shields.io/badge/test-runtime-0284c7) | `eve-rails-cli test --agent support` |
| ![preview](https://img.shields.io/badge/preview-dev-65a30d) | `eve-rails-cli preview --agent support --dry-run` |
| ![migrate](https://img.shields.io/badge/migrate-plan-92400e) | `eve-rails-cli migrate --agent support --env production --dry-run` |
| ![deploy](https://img.shields.io/badge/deploy-gates-16a34a) | `eve-rails-cli deploy --agent support --env staging --require-evals --require-doctor --dry-run` |
| ![rollback](https://img.shields.io/badge/rollback-restore-dc2626) | `eve-rails-cli rollback --agent support --to 1.3.2` |
| ![inspect](https://img.shields.io/badge/inspect-compose-4f46e5) | `eve-rails-cli inspect --agent support` |
| ![graph](https://img.shields.io/badge/graph-map-0f766e) | `eve-rails-cli graph --all --format mermaid` |

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
