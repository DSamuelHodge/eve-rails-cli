# Changelog

All notable changes to Eve Rails CLI will be documented here.

This project follows semantic versioning once `1.0.0` is released.

## [Unreleased]

### Added

- v1.1 CLI surface for `init`, `eval`, `test`, `preview`, and `migrate`.
- Expanded `doctor` safety checks for environments, connections, budgets, schedule metadata, production observability, generated freshness, and safe fix planning.
- Complete Rails-style generated agent layout including tools, skills, subagents, channels, schedules, approvals, eval contracts, top-level Eve evals, memory contracts, fixtures, and agent README files.
- Manifest `version_policy` support plus generated runtime compatibility metadata and runtime lockfile entries.
- Deploy delegation flags for approval gates, promotion, rollback targets, and dry-run command reporting.

### Changed

- Bumped the crate to `1.1.0`.
- Updated CLI help for `apply` and `deploy` to describe current behavior.
- Example fleet now inherits auth and visibility defaults required by schedule safety checks.

### Validated

- `cargo fmt --check`
- `cargo test --locked`
- Example fleet `doctor --env production --connections --budgets`
- Generated support agent `npm run typecheck`
- Generated support agent `npm exec -- eve info --json`

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
