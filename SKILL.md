---
name: eve-rails-cli
description: Build Eve agents and agent fleets using Eve Rails CLI conventions. Use when an agent needs to understand this repository, generate Eve-compatible agents, edit manifests/templates/catalogs, run doctor/render/test/deploy workflows, or operate an Eve Rails CLI project.
---

# Eve Rails CLI

Eve Rails CLI is a Rails-inspired convention layer for building Eve agent
fleets. It does not replace Eve. It gives teams a repeatable workflow around
Eve: YAML manifests, reusable catalogs, templates, generated Eve agents,
doctor checks, version locks, update planning, migrations, deployment gates,
and hot-load classification.

Use this skill when you need to build, modify, validate, test, or deploy agents
with this repository.

## First Principles

- Treat YAML manifests and catalogs as the source of truth.
- Treat generated files as outputs; regenerate them instead of hand-editing.
- Keep Eve-native files where Eve expects them.
- Keep Eve-Rails-only metadata outside Eve's `agent/` tree.
- Validate before deploying.
- Never commit secrets.

## What To Read First

Start with:

```sh
eve-rails-cli --help
eve-rails-cli <command> --help
```

Then inspect:

- `README.md` for install and command examples.
- `examples/basic-fleet/README.md` for the runnable demo project.
- `CHANGELOG.md` for release behavior.
- `templates/agent/` for generated output shape.
- `examples/basic-fleet/manifests/` for manifest and catalog examples.

For Eve runtime details, read the Eve docs bundled with the generated agent's
installed Eve package:

```sh
examples/basic-fleet/agents/support/node_modules/eve/docs/
```

If you are in a different generated agent, prefer that agent's
`node_modules/eve/docs/` because it matches the installed runtime version.

## Project Model

An Eve Rails project typically has:

```txt
manifests/
  agents.yml
  catalog.yml
  environments.yml

templates/
  agent/

agents/
  <agent-name>/
    package.json
    tsconfig.json
    evals/
    agent/
    .eve-rails/
```

In this repository, the demo project lives under `examples/basic-fleet/`.

## Eve-Native Versus Eve-Rails Metadata

Eve-native generated files belong under `agents/<name>/agent/` only when Eve
supports that surface, for example:

- `agent.ts`
- `instructions.md`
- `tools/`
- `skills/`
- `subagents/`
- `channels/`
- `schedules/`

Top-level Eve evals belong in:

- `agents/<name>/evals/evals.config.ts`
- `agents/<name>/evals/*.eval.ts`

Eve-Rails-only metadata belongs in:

- `agents/<name>/.eve-rails/approvals/`
- `agents/<name>/.eve-rails/evals/`
- `agents/<name>/.eve-rails/memory/`
- `agents/<name>/.eve-rails/fixtures/`

Do not place Rails-only folders such as `approvals`, `memory`, `fixtures`, or
eval contract metadata directly under `agent/`; Eve discovery treats unknown
directories there as unsupported.

## Core Workflow

Plan or generate from manifests:

```sh
eve-rails-cli plan manifests/agents.yml
eve-rails-cli apply manifests/agents.yml
eve-rails-cli generate agent support --auth platform-oauth --dry-run
eve-rails-cli generate schedule weekday_triage --schedule "0 9 * * 1-5"
```

Render generated files:

```sh
eve-rails-cli render --all
eve-rails-cli render --all --check
```

Validate:

```sh
eve-rails-cli doctor --all
eve-rails-cli doctor --all --env production --connections --budgets
```

Run local project checks:

```sh
cargo fmt --check
cargo test --locked
cargo build --locked
```

Check a generated Eve agent:

```sh
cd examples/basic-fleet/agents/support
npm run typecheck
npm exec -- eve info --json
```

`eve info` should report zero discovery errors and zero discovery warnings.

## Auth Profiles

Generated Eve channels support configurable auth profiles.

- `platform-oauth`: Vercel OIDC plus local development auth.
- `http-basic-env`: Vercel OIDC, local development auth, and HTTP Basic
  credentials read from `EVE_RAILS_BASIC_AUTH_USERNAME` and
  `EVE_RAILS_BASIC_AUTH_PASSWORD`.

Rules:

- Do not hardcode credentials in manifests, templates, or generated files.
- Use `http-basic-env` for explicit smoke testing or project policy only.
- Production protected routes should fail closed when unauthenticated.

## Versioning And Hot-Load

Use lockfiles and manifests to reason about updates:

```sh
eve-rails-cli outdated
eve-rails-cli update --agent support --minor --plan
eve-rails-cli hotload --agent support --current 1.0.0 skill:summarize_thread@1.0.1
```

The `hotload` command classifies compatibility. A `hotload` classification
means the change is safe by Eve-Rails policy; a `redeploy` classification means
the runtime or safety surface needs a deployment.

Memory schema changes require migrations. Generate and plan them with:

```sh
eve-rails-cli generate migration customer_profile_v2
eve-rails-cli migrate --agent support --env production --dry-run
```

## Deploy And Verify

Always run a dry deploy preflight first:

```sh
eve-rails-cli deploy --agent support --env production --require-evals --require-doctor --require-approvals --dry-run
```

For a real Eve deployment, run Eve from the generated agent directory:

```sh
cd examples/basic-fleet/agents/support
npm exec -- eve deploy
```

After deployment, verify:

- Public health route returns ready.
- Protected routes reject unauthenticated requests.
- Authenticated `/eve/v1/info` returns agent info with zero diagnostics.
- A real session can be created and streamed when credentials are available.

## Before You Finish

Confirm:

- Generated files are fresh with `render --all --check`.
- Rust checks pass with `cargo fmt --check`, `cargo test --locked`, and
  `cargo build --locked`.
- Generated Eve agents pass `npm run typecheck` and `npm exec -- eve info`.
- Changelog and version metadata are updated for release-facing changes.
- No secrets are tracked.
