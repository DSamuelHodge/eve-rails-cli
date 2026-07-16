---
name: eve
description: Build, generate, validate, test, deploy, and operate Eve agents in this repository using Eve Rails CLI conventions. Use when creating, editing, debugging, rendering, validating, deploying, hot-loading, or reviewing Eve/Eve-Rails agents, manifests, templates, tools, skills, subagents, channels, schedules, approvals, evals, memory, auth, or generated agent output.
---

# Eve Rails CLI

This repository contains Eve Rails CLI: a Rails-inspired convention layer over
Eve. Eve is still the runtime source of truth; Eve Rails CLI is the project and
fleet workflow for generating, composing, validating, versioning, testing, and
deploying Eve agents predictably.

Use this skill to behave like an Eve Rails maintainer: inspect first, generate
from manifests, keep Eve-native files in Eve-supported locations, keep
Rails-only metadata out of Eve's `agent/` tree, run gates before claiming
success, and never commit secrets.

## Source of Truth

Read these local files before changing behavior:

1. `SPEC.md` for the Eve Rails CLI contract, commands, file layout, manifest
   schema, auth profiles, versioning, doctor checks, and hot-load policy.
2. `README.md` for user-facing install and command examples.
3. `examples/basic-fleet/README.md` when working on the demo fleet.
4. `CHANGELOG.md` when release-facing behavior changes.
5. `RAILS_PHILOSOPHY_FOR_EVE.md` when making product or convention decisions.

For Eve runtime behavior, read the installed Eve docs that match the local Eve
version:

```sh
node_modules/eve/docs/
```

Start with `node_modules/eve/docs/README.md`, then read the relevant guide
before writing Eve-native code. If a generated agent has its own `node_modules`,
prefer that agent's `node_modules/eve/docs/` because it matches the deployed
runtime.

## Required Workflow

Before making changes:

- Run `eve-rails-cli --help` or `cargo run -- --help` to confirm the current CLI
  surface.
- For command details, use command help such as
  `cargo run -- doctor --help`, `cargo run -- render --help`, or
  `cargo run -- deploy --help`.
- Inspect the relevant manifests, catalog entries, templates, generated files,
  and tests before editing.

When changing generated agents:

- Edit YAML manifests, catalog entries, or templates first.
- Render generated output with `eve-rails-cli render` or
  `cargo run -- render`.
- Use `render --all --check` to prove generated files are fresh.
- Do not hand-edit generated regions unless explicitly asked to debug a
  generated artifact.

When validating:

```sh
cargo fmt --check
cargo test --locked
cargo build --locked
```

For the example fleet:

```sh
cd examples/basic-fleet
cargo run -- render --all --check --templates ../../templates/agent
cargo run -- doctor --manifest manifests/agents.yml --catalog manifests/catalog.yml
```

For a generated Eve agent:

```sh
cd examples/basic-fleet/agents/support
npm run typecheck
npm exec -- eve info --json
```

## Eve Rails Layout Rules

Eve-native surfaces belong under the generated agent's `agent/` tree only where
Eve supports them:

- `agent/agent.ts`
- `agent/instructions.md`
- `agent/tools/`
- `agent/skills/`
- `agent/subagents/`
- `agent/channels/`
- `agent/schedules/`
- other Eve-supported authored slots from the installed Eve docs

Top-level Eve evals belong in the generated app root:

- `evals/evals.config.ts`
- `evals/*.eval.ts`

Rails-only convention metadata belongs under app-root `.eve-rails/`, not under
`agent/`, so Eve discovery stays warning-free:

- `.eve-rails/approvals/`
- `.eve-rails/evals/`
- `.eve-rails/memory/`
- `.eve-rails/fixtures/`

If `npm exec -- eve info --json` reports unsupported directories under
`agent/`, fix the generator or stale generated output rather than accepting the
warning.

## Auth Rules

Auth is configurable. Do not hardcode one team's production policy into the
framework template.

Supported generated profiles currently include:

- `platform-oauth`: Eve `vercelOidc()` plus `localDev()`.
- `http-basic-env`: Eve `vercelOidc()`, `localDev()`, and `httpBasic()` using
  `EVE_RAILS_BASIC_AUTH_USERNAME` and `EVE_RAILS_BASIC_AUTH_PASSWORD`.

Rules:

- Never commit credentials or generated `.env*` files.
- Production protected routes must fail closed when unauthenticated.
- Use env-backed auth only for smoke testing or an explicit project policy.
- For other teams, add explicit profiles rather than changing the default.

## Hot-Load And Deploy

Use Eve Rails CLI to classify changes:

```sh
eve-rails-cli hotload --agent support --current 1.0.0 skill:summarize_thread@1.0.1
```

Interpretation:

- `hotload` means the change is compatible by Eve Rails policy.
- `redeploy` means runtime or safety assumptions require a deployment.
- Eve production changes are still verified through real Eve/Vercel deployment
  unless a native Eve hot-load command exists in the installed runtime.

Before production deploy:

```sh
eve-rails-cli deploy --agent support --env production --require-evals --require-doctor --require-approvals --dry-run
```

For real deployment, delegate through Eve from the generated agent directory:

```sh
cd examples/basic-fleet/agents/support
npm exec -- eve deploy
```

After deploy, verify both safety and runtime behavior:

- Public health route returns ready.
- Protected routes reject unauthenticated requests.
- Authenticated `/eve/v1/info` reports zero discovery warnings.
- A real session can be created and streamed when credentials are available.

## Command Reference

Prefer the live CLI help, but common commands are:

```sh
eve-rails-cli init my-fleet --template customer-support --dry-run
eve-rails-cli plan manifests/agents.yml
eve-rails-cli apply manifests/agents.yml
eve-rails-cli render --all --check
eve-rails-cli doctor --all --env production --connections --budgets
eve-rails-cli generate agent support --auth platform-oauth --dry-run
eve-rails-cli generate schedule weekday_triage --schedule "0 9 * * 1-5"
eve-rails-cli outdated
eve-rails-cli update --agent support --minor --plan
eve-rails-cli eval --agent support --dry-run
eve-rails-cli test --agent support
eve-rails-cli preview --agent support --dry-run
eve-rails-cli migrate --agent support --env production --dry-run
eve-rails-cli rollback --agent support --to 1.3.2
eve-rails-cli inspect --agent support
eve-rails-cli graph --all --format mermaid
```

## Safety Checklist

Before finalizing work:

- `Cargo.toml` and `Cargo.lock` agree on the crate version when releasing.
- `CHANGELOG.md` is updated for user-visible behavior.
- `SPEC.md` and `README.md` match the actual CLI help.
- Generated files are fresh.
- `eve info` has zero diagnostics for generated example agents.
- No secrets are present in tracked files.
- Release tags are not moved after publishing.
