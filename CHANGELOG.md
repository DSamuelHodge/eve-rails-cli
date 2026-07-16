# Changelog

All notable changes to Eve Rails CLI will be documented here.

This project follows semantic versioning.

## [Unreleased]

## [1.1.0] - 2026-07-16

### Added

- v1.1 CLI surface for `init`, `eval`, `test`, `preview`, and `migrate`.
- Expanded `doctor` safety checks for environments, connections, budgets, schedule metadata, production observability, generated freshness, and safe fix planning.
- Complete Rails-style generated agent layout including tools, skills, subagents, channels, schedules, app-root `.eve-rails` metadata, top-level Eve evals, memory contracts, fixtures, and agent README files.
- Manifest `version_policy` support plus generated runtime compatibility metadata and runtime lockfile entries.
- Deploy delegation flags for approval gates, promotion, rollback targets, and dry-run command reporting.
- Configurable generated Eve auth profiles, including `platform-oauth` and env-backed `http-basic-env` for smoke testing.

### Changed

- Bumped the crate to `1.1.0`.
- Updated CLI help for `apply` and `deploy` to describe current behavior.
- Example fleet now inherits auth and visibility defaults required by schedule safety checks.
- Moved Rails-only generated conventions out of Eve's `agent/` tree and into `.eve-rails/` so Eve discovery stays warning-free.
- Updated README and SPEC command examples to match the current CLI surface.

### Validated

- `cargo fmt --check`
- `cargo test --locked`
- `cargo build --locked`
- Example fleet `doctor --env production --connections --budgets`
- Generated support agent `npm run typecheck`
- Generated support agent `npm exec -- eve info --json`
- Real Eve eval path with `xai/grok-4.5`
- Real Vercel production deploys for the generated support agent
- Authenticated production session returning `EVE_RAILS_AUTH_OK`
- Hot-load classification for `skill:summarize_thread@1.0.1`
- Post-update production session returning `EVE_RAILS_HOTLOAD_OK`

## [1.0.0] - 2026-07-16

### Added

- Rust CLI for Rails-inspired Eve agent fleet conventions.
- Manifest and catalog validation.
- Strict Jinja-compatible rendering.
- Generators for agents and reusable components.
- Batch plan/apply workflow.
- Structured doctor diagnostics.
- Version resolver, lockfile handling, update reports, hot-load planning, and migrations.
- Deploy preflight and rollback planning.
- Inspect and graph outputs.
- Eve-compatible example fleet under `examples/basic-fleet`.
- CI workflow, issue templates, PR template, and contributor guide.

### Validated

- Generated support agent starts with Eve local dev.
- Eve session creation and streaming work locally.
- Vercel AI Gateway live model call completed with `openai/gpt-5.5`.
