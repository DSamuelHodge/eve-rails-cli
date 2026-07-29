<div align="center">
  <a href="https://github.com/vercel/eve">
    <img alt="eve logo" src="https://raw.githubusercontent.com/vercel/eve/main/.github/assets/eve.svg" height="96">
  </a>
  <h1>Eve Rails CLI</h1>

[![CI/CD](https://github.com/DSamuelHodge/eve-rails-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/DSamuelHodge/eve-rails-cli/actions/workflows/ci.yml)
[![Version](https://img.shields.io/badge/version-1.1.0-blue.svg)](Cargo.toml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.97%2B-orange.svg)](https://www.rust-lang.org/)
[![Node](https://img.shields.io/badge/Node.js-24%2B-339933.svg)](https://nodejs.org/)
[![Eve](https://img.shields.io/badge/Eve-0.24%2B-black.svg)](https://www.npmjs.com/package/eve)

</div>

[Eve](https://github.com/vercel/eve) is Vercel's filesystem-first framework for durable AI agents. Eve agents keep runtime capabilities in conventional folders like `agent/instructions.md`, `agent/tools/`, `agent/skills/`, `agent/channels/`, and `agent/schedules/`.

**Eve Rails CLI adds a convention layer for teams building more than one agent. "Rails" here means convention over configuration: put fleet intent in YAML, reuse catalog components, render Eve-compatible agents, and run safety checks before deploys.**

## Why It Matters

Engineering teams need more than prompts in folders. Eve Rails CLI gives a fleet
the operational backbone around Eve:

- fleet manifests for many agents, not one-off project folders;
- reusable catalog tools, skills, evals, approvals, channels, and memory;
- template rendering into Eve-compatible agent projects;
- doctor checks for freshness, safety, approvals, budgets, and runtime config;
- version, hot-load, migration, deploy, rollback, inspect, and graph workflows;
- diagnostics that humans and coding agents can follow.

## Quick Start

### 1. Install

```sh
git clone https://github.com/DSamuelHodge/eve-rails-cli.git
cd eve-rails-cli
cargo install --path .
```

### 2. Create one agent

```sh
eve-rails-cli init my-project --yes
cd my-project

eve-rails-cli generate agent support
eve-rails-cli apply manifests/agents.yml
eve-rails-cli doctor --all
```

You now have a working Eve agent in `agents/support/`.

### 3. Add reusable pieces

```sh
eve-rails-cli generate tool search_customers --side-effects read
eve-rails-cli generate skill triage_customer_issue
```

Then attach them to `support` in `manifests/agents.yml`:

```yaml
agents:
  - name: support
    tools:
      search_customers: 1.0.0
    skills:
      triage_customer_issue: 1.0.0
```

```sh
eve-rails-cli apply manifests/agents.yml
```

Tools and skills live in `manifests/catalog.yml` so they can be reused across
more agents later.

### 4. What just got created

Eve Rails CLI does not make manifests mysterious. `generate` writes reusable
pieces to `manifests/catalog.yml`, while `manifests/agents.yml` says which agent
uses which pieces.

After step 3, `catalog.yml` includes:

```yaml
tools:
  search_customers:
    version: 1.0.0
    side_effects: read
skills:
  triage_customer_issue:
    version: 1.0.0
```

And `agents.yml` connects those reusable pieces to the agent:

```yaml
agents:
  - name: support
    tools:
      search_customers: 1.0.0
    skills:
      triage_customer_issue: 1.0.0
```

Edit the YAML by hand any time. Run `eve-rails-cli apply manifests/agents.yml`
again after manual edits to re-render generated agents.

### 5. Verify the generated agent

```sh
cd agents/support
npm install
npm run typecheck
npm exec -- eve info --json
```

Ready for more than one agent, or tools that need human sign-off, such as
anything that moves money? See [Building a fleet](docs/fleet.md).

## Requirements

- Rust 1.97 or newer. Install Rust with [rustup](https://rustup.rs/); many OS package managers ship older Rust versions that cannot compile Rust 2024 crates.
- Node.js 24 or newer when testing generated Eve agents. The CI Eve smoke job uses Node 24 because the Vercel Eve framework requires the modern Node runtime.
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

## License

Eve Rails CLI is available under the [MIT License](LICENSE). MIT is permissive and allows private use, modification, distribution, sublicensing, and commercial use when the license notice is preserved.

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](.github/CONTRIBUTING.md) and the GitHub issue and pull request templates.
