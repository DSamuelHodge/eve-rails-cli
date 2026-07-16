# Eve Rails CLI

Rails-inspired convention layer for generating, validating, versioning, and operating Eve agent fleets.

## Primary Language

Rust is the primary implementation language for the CLI core.

Why Rust:

- Fast startup for local and CI workflows.
- Strong typing for manifests, lockfiles, and compatibility checks.
- Single-binary distribution path.
- Good fit for deterministic filesystem tooling.

Jinja2-compatible templates are handled through Rust's `minijinja` crate. YAML manifests are parsed with `serde_yaml`.

## Runtime Requirements

- Rust 1.97 or newer.
- Node.js 24 or newer for testing real Eve flows through `npx eve@latest`.

This project keeps the CLI core in Rust, but Eve itself is distributed through npm, so Node is required for integration tests against the real framework.

## Current Status

This repository is at the environment and scaffold stage.

Implemented:

- Cargo project setup.
- Rust CLI skeleton.
- YAML manifest loading.
- Basic manifest validation.
- Seed manifests.
- Seed Jinja2 templates.
- Smoke-tested commands: `plan`, `doctor`, `inspect`, `graph`.

Stubbed for later PRs:

- File rendering.
- Generators.
- Version resolver and lockfile.
- Hot-load compatibility.
- Migrations.
- Deploy delegation.

## Development

```sh
cargo test
cargo run -- plan examples/basic-fleet/manifests/agents.yml --catalog examples/basic-fleet/manifests/catalog.yml
cargo run -- doctor --all --manifest examples/basic-fleet/manifests/agents.yml --catalog examples/basic-fleet/manifests/catalog.yml
cargo run -- inspect --agent support --manifest examples/basic-fleet/manifests/agents.yml --catalog examples/basic-fleet/manifests/catalog.yml
cargo run -- graph --all --manifest examples/basic-fleet/manifests/agents.yml --catalog examples/basic-fleet/manifests/catalog.yml --format mermaid
npx eve@latest --help
```

## Project Documents

- `RAILS_PHILOSOPHY_FOR_EVE.md`: product and framework philosophy.
- `SPEC.md`: implementation spec, PR plan, checklists, and metrics.
