# Changelog

All notable changes to Eve Rails CLI will be documented here.

This project follows semantic versioning once `1.0.0` is released.

## [Unreleased]

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
