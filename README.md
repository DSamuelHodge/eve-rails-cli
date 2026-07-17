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

**Eve Rails CLI adds a convention layer for teams building more than one agent. "Rails" here means convention over configuration: put fleet intent in YAML, reuse catalog components, render Eve-compatible agents, and run safety checks before deploys.**

## Why It Matters

Agent teams need more than prompts in folders. They need repeatable defaults, reusable tools and skills, eval gates, approval policy, memory conventions, hot-load rules, rollback plans, and diagnostics that a human or coding agent can follow. Eve Rails CLI gives that backbone so many agents can be generated, reviewed, updated, and operated consistently.

## Quick Start

Build a small fleet from shared catalog components:

```sh
git clone https://github.com/DSamuelHodge/eve-rails-cli.git
cd eve-rails-cli
cargo install --path .

eve-rails-cli init support-fleet --template basic --yes
cd support-fleet

eve-rails-cli generate tool search_customers --side-effects read
eve-rails-cli generate tool prepare_refund --side-effects money
eve-rails-cli generate skill triage_customer_issue
eve-rails-cli generate eval standard

eve-rails-cli generate agent support \
  --with-tools search_customers \
  --with-skills triage_customer_issue \
  --with-evals standard

eve-rails-cli generate agent billing \
  --with-tools search_customers,prepare_refund \
  --with-skills triage_customer_issue \
  --with-evals standard \
  --approval required

eve-rails-cli apply manifests/agents.yml
eve-rails-cli doctor --all
eve-rails-cli render --all --check
```

Then verify any generated Eve agent:

```sh
cd agents/support
npm install
npm run typecheck
npm exec -- eve info --json
```

## Requirements

- Rust 1.97 or newer. Install Rust with [rustup](https://rustup.rs/); many OS package managers ship older Rust versions that cannot compile Rust 2024 crates.
- Node.js 24 or newer when testing generated Eve agents.
- Optional Vercel AI Gateway credentials for live model calls.

## Fleet Model

Eve Rails CLI keeps fleet intent in YAML and renders one Eve project per agent:

```text
manifests/agents.yml   # fleet defaults and agent composition
manifests/catalog.yml  # reusable tools, skills, evals, approvals, channels
templates/agent/       # render templates
agents/<name>/         # generated Eve projects
```

The fleet manifest names the agents and what each one uses:

```yaml
defaults:
  model: openai/gpt-5.5
  owner: agent-platform
  channels: []
  schedules: []
  evals: [standard]

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

The catalog defines reusable components once:

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
```

Tools, skills, evals, approvals, channels, schedules, memory, auth, and
deployment metadata can all be generated and reused this way. Run
`eve-rails-cli <command> --help` for the current options.

## Commands

Use `eve-rails-cli --help` for the command list and
`eve-rails-cli <command> --help` for flags.

- Create: `init`, `generate`
- Render: `plan`, `apply`, `render`
- Validate: `doctor`, `test`, `eval`, `preview`
- Operate: `outdated`, `update`, `hotload`, `migrate`, `deploy`, `rollback`
- Inspect: `inspect`, `graph`

## Eve Resources

- Eve repository: [github.com/vercel/eve](https://github.com/vercel/eve)
- Eve documentation: [eve.dev/docs](https://eve.dev/docs)
- Eve package: [npmjs.com/package/eve](https://www.npmjs.com/package/eve)
- Eve community: [GitHub Discussions](https://github.com/vercel/eve/discussions)

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](.github/CONTRIBUTING.md), [CHANGELOG.md](CHANGELOG.md), and the GitHub issue and pull request templates.
