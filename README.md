<div align="center">
  <a href="https://github.com/vercel/eve">
    <img alt="eve logo" src="https://raw.githubusercontent.com/vercel/eve/main/.github/assets/eve.svg" height="96">
  </a>
  <h1>Eve Rails CLI</h1>

[![CI](https://github.com/DSamuelHodge/eve-rails-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/DSamuelHodge/eve-rails-cli/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.97%2B-orange.svg)](https://www.rust-lang.org/)
[![Eve](https://img.shields.io/badge/Eve-0.24%2B-black.svg)](https://www.npmjs.com/package/eve)

</div>

[Eve](https://github.com/vercel/eve) is Vercel's filesystem-first framework for durable AI agents. Eve agents keep runtime capabilities in conventional folders like `agent/instructions.md`, `agent/tools/`, `agent/skills/`, `agent/channels/`, and `agent/schedules/`.

Eve Rails CLI adds a convention layer for teams building more than one agent. "Rails" here means convention over configuration: put fleet intent in YAML, reuse catalog components, render Eve-compatible agents, and run safety checks before deploys.

## Why It Matters

Agent teams need more than prompts in folders. They need repeatable defaults, reusable tools and skills, eval gates, approval policy, memory conventions, hot-load rules, rollback plans, and diagnostics that a human or coding agent can follow. Eve Rails CLI gives that backbone so many agents can be generated, reviewed, updated, and operated consistently.

## Quick Start

```sh
git clone https://github.com/DSamuelHodge/eve-rails-cli.git
cd eve-rails-cli
cargo install --path .

eve-rails-cli init my-fleet --template basic --model openai/gpt-5.5 --owner agent-platform --yes
cd my-fleet

eve-rails-cli generate tool search_customers --side-effects read
eve-rails-cli generate skill triage_customer_issue
eve-rails-cli generate channel eve
eve-rails-cli generate schedule weekday_triage --schedule "0 9 * * 1-5"
eve-rails-cli generate eval standard
eve-rails-cli generate memory customer_profile --retention 180d

eve-rails-cli generate agent support \
  --with-tools search_customers \
  --with-skills triage_customer_issue \
  --with-channels eve \
  --with-schedules weekday_triage \
  --with-evals standard \
  --with-memory customer_profile \
  --approval required \
  --auth platform-oauth \
  --visibility internal

eve-rails-cli plan manifests/agents.yml
eve-rails-cli apply manifests/agents.yml
eve-rails-cli doctor --all --templates --updates
eve-rails-cli render --all --check
```

Then verify the generated Eve agent:

```sh
cd agents/support
npm install
npm run typecheck
npm exec -- eve info --json
```

## Requirements

- Rust 1.97 or newer.
- Node.js 24 or newer when testing generated Eve agents.
- Optional Vercel AI Gateway credentials for live model calls.

## YAML Shape

Single-agent project:

```yaml
defaults:
  model: openai/gpt-5.5
  owner: support-platform
  channels: [eve]
  schedules: []
  evals: [standard]

agents:
  - name: support
    version: 1.0.0
    responsibility: Resolve customer support requests.
    tools:
      search_customers: 1.0.0
    skills:
      triage_customer_issue: 1.0.0
    memory:
      customer_profile: 1.0.0
    approvals:
      search_customers: required
    auth: platform-oauth
    visibility: internal
```

Fleet project:

```yaml
defaults:
  model: openai/gpt-5.5
  owner: agent-platform
  channels: [eve]
  schedules: [weekday_triage]
  evals: [standard]

shared:
  tools:
    search_customers: 1.0.0
  skills:
    triage_customer_issue: 1.0.0
  memory:
    customer_profile: 1.0.0

agents:
  - name: support
    version: 1.0.0
    responsibility: Triage and resolve support requests.
    tools:
      search_customers: 1.0.0
    skills:
      triage_customer_issue: 1.0.0
    approvals:
      search_customers: required

  - name: billing
    version: 1.0.0
    responsibility: Answer billing questions and prepare safe refund actions.
    tools:
      search_customers: 1.0.0
      prepare_refund: 1.0.0
    skills:
      triage_customer_issue: 1.0.0
    approvals:
      prepare_refund: required
```

Catalog entries referenced by those manifests live in `manifests/catalog.yml`:

```yaml
tools:
  search_customers:
    version: 1.0.0
    side_effects: read
  prepare_refund:
    version: 1.0.0
    side_effects: money
skills:
  triage_customer_issue:
    version: 1.0.0
evals:
  standard:
    version: 1.0.0
approvals:
  required:
    version: 1.0.0
memory:
  customer_profile:
    version: 1.0.0
    retention: 180d
channels:
  eve:
    version: 1.0.0
schedules:
  weekday_triage:
    version: 1.0.0
    schedule: "0 9 * * 1-5"
```

## Commands

Project setup and generation:

```sh
eve-rails-cli init my-fleet --template basic --dry-run
eve-rails-cli generate agent support --dry-run
eve-rails-cli generate tool search_customers --side-effects read
eve-rails-cli generate schedule weekday_triage --schedule "0 9 * * 1-5"
eve-rails-cli generate batch manifests/batch.yml --dry-run
```

Planning, rendering, and inspection:

```sh
eve-rails-cli plan manifests/agents.yml
eve-rails-cli apply manifests/agents.yml
eve-rails-cli render --all --check
eve-rails-cli inspect --agent support
eve-rails-cli graph --all --format mermaid
```

Safety and runtime checks:

```sh
eve-rails-cli doctor --all --env production --connections --budgets
eve-rails-cli eval --agent support --dry-run
eve-rails-cli test --agent support
eve-rails-cli preview --agent support --dry-run
```

Versioning, migrations, deploys, and rollback:

```sh
eve-rails-cli outdated --agent support
eve-rails-cli update --agent support --minor --plan
eve-rails-cli hotload --agent support skill:triage_customer_issue@1.0.1
eve-rails-cli migrate --agent support --env production --dry-run
eve-rails-cli deploy --agent support --env staging --require-evals --require-doctor --dry-run
eve-rails-cli rollback --agent support --to 1.0.0 --dry-run
```

## Coding Agent Skill

This repository includes a self-contained [SKILL.md](SKILL.md) for coding agents. It explains Eve, Eve Rails CLI conventions, manifests, commands, safety checks, auth, migrations, hot-load, deploys, and completion criteria without requiring the agent to read the rest of the repository first.

Copy this command into a coding agent environment to fetch the skill text:

```sh
curl -L https://raw.githubusercontent.com/DSamuelHodge/eve-rails-cli/main/SKILL.md
```

Or copy the raw skill URL:

```text
https://raw.githubusercontent.com/DSamuelHodge/eve-rails-cli/main/SKILL.md
```

## Eve Resources

- Eve repository: [github.com/vercel/eve](https://github.com/vercel/eve)
- Eve documentation: [eve.dev/docs](https://eve.dev/docs)
- Eve package: [npmjs.com/package/eve](https://www.npmjs.com/package/eve)
- Eve community: [GitHub Discussions](https://github.com/vercel/eve/discussions)

## Contributing

Contributions are welcome. See [.github/CONTRIBUTING.md](CONTRIBUTING.md), [CHANGELOG.md](CHANGELOG.md), and the GitHub issue and pull request templates.
