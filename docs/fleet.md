# Building A Fleet

Eve Rails CLI is most useful when several agents share the same catalog of
tools, skills, evals, approvals, channels, schedules, and memory conventions.

## Fleet Shape

`manifests/agents.yml` names each agent and the reusable pieces it uses:

```yaml
defaults:
  model: openai/gpt-5.5
  owner: agent-platform
  channels: []
  schedules: []
  evals: [standard]

agents:
  - name: support
    responsibility: Triage and resolve support requests.
    tools:
      search_customers: 1.0.0
    skills:
      triage_customer_issue: 1.0.0

  - name: billing
    responsibility: Answer billing questions and prepare safe refund actions.
    tools:
      search_customers: 1.0.0
      prepare_refund: 1.0.0
    skills:
      triage_customer_issue: 1.0.0
    approvals:
      prepare_refund: required
```

`manifests/catalog.yml` defines the reusable pieces once:

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

## Money And Approval

Tools with side effects such as `write`, `external`, `money`, or `production`
should have approval coverage before deployment. `doctor` checks this so risky
tools do not quietly ship without a policy.

```sh
eve-rails-cli doctor --all
eve-rails-cli apply manifests/agents.yml
eve-rails-cli render --all --check
```

Use `eve-rails-cli --help` and `eve-rails-cli <command> --help` for the current
command flags.
