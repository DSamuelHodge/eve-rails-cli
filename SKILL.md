---
name: eve-rails-cli
description: Build, validate, render, test, version, migrate, and deploy Eve agents and agent fleets using Eve Rails CLI conventions. Use when an agent needs to work inside an Eve Rails CLI project or create Eve-compatible agents from manifests, catalogs, templates, and generated runtime folders.
---

# Eve Rails CLI

Eve is a TypeScript framework for durable backend AI agents. An Eve agent is a
small project with an `agent/` runtime tree, optional tools, skills, subagents,
channels, schedules, evals, and package scripts that can be inspected or
deployed through the Eve CLI.

Eve Rails CLI is the convention layer around Eve. It does not replace Eve. It
adds a Rails-like workflow so fleets of agents can be generated and operated
predictably from source-of-truth YAML:

- manifests describe agents and fleet defaults
- catalogs define reusable tools, skills, evals, channels, schedules, approvals,
  and memory schemas
- templates render Eve-compatible projects
- doctor checks enforce safety, freshness, budgets, approvals, and environment
  readiness
- lockfiles, migrations, hot-load checks, deploy gates, and rollback plans make
  updates reviewable

Use this skill to modify an Eve Rails CLI project as if the reader has never
seen Eve before.

## Operating Rules

- Treat `manifests/*.yml` and catalog entries as the source of truth.
- Treat generated files under `agents/<name>/` as outputs; change manifests,
  catalogs, or templates, then render again.
- Keep Eve-native runtime files only where Eve expects them.
- Keep Eve-Rails-only metadata outside the Eve `agent/` discovery tree.
- Run `doctor` and `render --check` before deployment or release work.
- Never hardcode credentials, tokens, Basic Auth values, API keys, or provider
  secrets.
- Prefer dry runs for risky operations: deploy, migrate, rollback, update, and
  generator overwrites.

## Project Shape

A normal Eve Rails project looks like:

```txt
manifests/
  agents.yml          # fleet defaults and concrete agent definitions
  catalog.yml         # reusable component catalog
  environments.yml    # env-specific requirements and production gates

templates/
  agent/              # Jinja-compatible templates used by render/apply

agents/
  <agent-name>/
    package.json
    tsconfig.json
    evals/
      evals.config.ts
      *.eval.ts
    agent/
      agent.ts
      instructions.md
      tools/
      skills/
      subagents/
      channels/
      schedules/
    .eve-rails/
      approvals/
      evals/
      memory/
      fixtures/
```

Eve discovers runtime surfaces inside `agents/<name>/agent/`. Put only
Eve-native runtime folders there: `tools`, `skills`, `subagents`, `channels`,
and `schedules`. Put Rails-layer contracts such as approval metadata, memory
schemas, fixture data, and eval contract metadata under `.eve-rails/` so Eve
does not treat them as unsupported runtime directories.

## Manifest Model

`manifests/agents.yml` defines defaults and agents:

```yaml
defaults:
  model: openai/gpt-5.5
  owner: agent-platform
  channels: []
  schedules: []
  evals: []

agents:
  - name: support
    version: 1.0.0
    responsibility: Resolve customer support requests.
    model: openai/gpt-5.5
    owner: support-platform
    tools:
      refund_customer: 1.0.0
    skills:
      handle_refund: 1.0.0
    channels: [slack]
    schedules: [weekday_triage]
    evals: [refund_policy]
    memory:
      customer_profile: 1.0.0
    approvals:
      refund_customer: required
    auth: platform-oauth
    visibility: internal
    token_budget: 100000
    cost_budget: 10
    timeout: 30s
```

`manifests/catalog.yml` defines the reusable components referenced by
manifests:

```yaml
tools:
  refund_customer:
    version: 1.0.0
    side_effects: money
skills:
  handle_refund:
    version: 1.0.0
evals:
  refund_policy:
    version: 1.0.0
approvals:
  required:
    version: 1.0.0
memory:
  customer_profile:
    version: 1.0.0
    retention: 180d
channels:
  slack:
    version: 1.0.0
    kind: slack
    connect_uid: slack/support-agent
schedules:
  weekday_triage:
    version: 1.0.0
    schedule: "0 9 * * 1-5"
```

Risky tools with `side_effects: write`, `external`, `money`, or `production`
must have approval policy coverage. Memory schemas need retention. Schedules
must have owner/auth/visibility directly or inherit safe agent defaults.

## Core Commands

Create or inspect a project:

```sh
eve-rails-cli init my-fleet --template basic --model openai/gpt-5.5 --owner agent-platform --dry-run
eve-rails-cli --help
eve-rails-cli <command> --help
```

Plan and write generated agents:

```sh
eve-rails-cli plan manifests/agents.yml
eve-rails-cli apply manifests/agents.yml
eve-rails-cli render --all
eve-rails-cli render --all --check
```

Generate source-of-truth entries:

```sh
eve-rails-cli generate agent support --with-tools refund_customer --approval required --dry-run
eve-rails-cli generate tool refund_customer --side-effects money
eve-rails-cli generate skill handle_refund
eve-rails-cli generate subagent researcher
eve-rails-cli generate channel slack --kind slack --connect-uid slack/support-agent
eve-rails-cli generate channel sms_support --kind twilio --allow-from env:TWILIO_ALLOWED_FROM --messaging-from env:TWILIO_FROM_NUMBER
eve-rails-cli generate schedule weekday_triage --schedule "0 9 * * 1-5"
eve-rails-cli generate approval required
eve-rails-cli generate eval refund_policy
eve-rails-cli generate memory customer_profile --retention 180d
eve-rails-cli generate migration customer_profile_v2
eve-rails-cli generate batch manifests/agents.yml --dry-run
```

Validate safety and freshness:

```sh
eve-rails-cli doctor --all
eve-rails-cli doctor --all --templates --updates
eve-rails-cli doctor --all --env production --connections --budgets
eve-rails-cli doctor --all --fix --dry-run
```

`doctor --fix` may create missing placeholder eval stubs, generated
directories, stale generated output from valid templates/manifests, and
formatted generated metadata. It must not add credentials, weaken approvals,
suppress failing evals, or change production safety policy.

## Runtime Delegation

Eve Rails CLI delegates runtime operations to generated Eve projects when
appropriate:

```sh
eve-rails-cli eval --agent support --dry-run
eve-rails-cli test --agent support
eve-rails-cli preview --agent support --dry-run
```

For direct Eve checks from an agent directory:

```sh
cd agents/support
npm run typecheck
npm exec -- eve info --json
```

`eve info` should report zero discovery errors and zero discovery warnings.
Missing Node, npm, Eve, credentials, or generated files should be reported as
actionable setup problems, not worked around by editing generated output.

## Versioning, Migration, And Hot-Load

Use version commands to classify changes before deploying:

```sh
eve-rails-cli outdated
eve-rails-cli update --agent support --minor --plan
eve-rails-cli hotload --agent support --current 1.0.0 skill:summarize_thread@1.0.1
```

Hot-load means the Eve Rails policy considers the change compatible without a
full redeploy. Redeploy means the runtime surface or safety profile changed.
Schedule changes that increase autonomous activity, approval changes, memory
schema changes, auth changes, and production-write tool changes should not be
treated as casual hot-loads.

Plan migrations before schema or contract changes:

```sh
eve-rails-cli migrate --agent support --env production --dry-run
eve-rails-cli migrate --fleet manifests/agents.yml --env production --dry-run
eve-rails-cli migrate --agent support --env production --apply
```

Dry-run migration output must be safe to review and JSON-compatible when
`--json` is used. Do not apply destructive migrations without an explicit plan.

## Deploy And Rollback

Always run deployment as a gated preflight first:

```sh
eve-rails-cli deploy --agent support --env production --require-evals --require-doctor --require-approvals --dry-run
```

A non-dry deployment delegates to Eve from the selected generated agent
directory. Production deploys should require doctor, evals, approvals, and
environment readiness unless the project explicitly defines a safer
non-production exception.

Rollback is planned, not guessed:

```sh
eve-rails-cli rollback --agent support --to 1.0.0 --dry-run
```

Unsafe rollback, memory schema rollback, approval policy rollback, and tool
contract rollback should require explicit migration or rollback planning.

## Auth And Secrets

Supported generated channel auth profiles include:

- `platform-oauth`: platform identity/OIDC plus local development behavior.
- `http-basic-env`: platform identity/OIDC plus HTTP Basic credentials read
  from environment variables for explicit smoke tests or project policy.

Rules:

- Do not hardcode auth strategy globally for every developer.
- Select auth per manifest, environment, or generated channel policy.
- Store local Basic Auth in environment variables such as
  `EVE_RAILS_BASIC_AUTH_USERNAME` and `EVE_RAILS_BASIC_AUTH_PASSWORD`.
- Store model/provider credentials in environment variables such as
  `AI_GATEWAY_API_KEY`; never commit them.
- Production protected routes should fail closed when unauthenticated.

## Completion Checklist

Before finishing a change:

- `eve-rails-cli render --all --check` passes.
- `eve-rails-cli doctor --all --templates --updates` passes for the relevant
  manifests.
- Production-affecting changes pass `doctor --env production --connections
  --budgets`.
- Generated Eve agents pass `npm run typecheck` and `npm exec -- eve info
  --json` when Node/Eve dependencies are available.
- Rust changes pass `cargo fmt --check`, `cargo test --locked`, and
  `cargo build --locked`.
- Deployment and migration commands are dry-run verified before real execution.
- No secrets, generated local caches, node_modules, or private planning docs are
  tracked.
