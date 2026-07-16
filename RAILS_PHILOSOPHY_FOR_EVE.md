# A Rails Philosophy for Eve Agents

## Thesis

Eve gives agents a filesystem-native runtime. A Rails philosophy can give teams the social contract around that runtime: conventions that make agents fast to create, safe to change, easy to compose, and boring to operate.

The goal is not to turn Eve into Rails. The goal is to bring Rails' best lesson to agent engineering:

> Put the common decisions in the framework so builders can spend their attention on the product behavior.

For agents, that means clear locations for identity, tools, skills, state, channels, approvals, evals, observability, and delegation.

## What Eve Already Gets Right

Eve's public preview already has the right primitive: an agent is a directory. Core capabilities live in conventional places:

- `agent/instructions.md` for identity and standing behavior.
- `agent/agent.ts` for model and runtime configuration.
- `agent/tools/` for typed capabilities.
- `agent/skills/` for reusable procedures loaded on demand.
- `agent/subagents/` for delegated specialists.
- `agent/channels/` for Slack, Discord, Teams, web, HTTP, and similar surfaces.
- `agent/schedules/` for autonomous recurring work.
- Built-in durability, sandboxing, approvals, evals, Vercel deployment, and observability.

That is the same kind of move Rails made for web apps: choose the obvious project shape, then let teams move quickly inside it.

## The Rails Layer

Rails was not only MVC. It was a philosophy:

1. Convention over configuration.
2. Opinionated defaults.
3. Integrated full-stack development.
4. A short path from idea to production.
5. Composable primitives with clear boundaries.
6. Environments, migrations, tests, and generators as first-class workflows.

For Eve agents, the equivalent philosophy should be:

1. Agents are products, not prompts.
2. Directories are contracts.
3. Skills are playbooks, tools are capabilities, subagents are boundaries.
4. Every risky action has an approval policy.
5. Every agent ships with evals.
6. Every agent can be observed, replayed, and improved.
7. Composition should feel like routing, not orchestration glue.

## Proposed Agent Shape

```txt
agent/
  instructions.md
  agent.ts

  tools/
    create_invoice.ts
    search_customers.ts

  skills/
    handle_refund.md
    triage_customer_issue.md

  subagents/
    researcher/
      instructions.md
      tools/
      skills/

  channels/
    slack.ts
    web.ts

  schedules/
    daily_digest.ts

  approvals/
    policy.ts

  evals/
    refund_policy.eval.ts
    customer_tone.eval.ts

  memory/
    schema.ts
    retention.md

  fixtures/
    customers.json
    tickets.json
```

Some of these folders may map directly to Eve today. Others are proposed conventions layered on top. The important part is that every agent advertises what it can do, where it acts, how it is tested, what it remembers, and when it must ask for permission.

## Conventions

### Instructions Are Controllers

`instructions.md` should define the agent's role, decision style, escalation rules, and non-negotiable constraints. It should not become a junk drawer for every workflow.

Keep it short and durable. Move task-specific procedures into `skills/`.

### Tools Are Models

Tools are the agent's typed interface to the outside world. They should be small, named clearly, validated at the boundary, and safe by default.

Each tool should answer:

- What can it do?
- What inputs does it accept?
- What side effects can it create?
- Does it require approval?
- What errors should the agent know how to handle?

### Skills Are Service Objects

Skills should encode reusable workflows: refund handling, PR triage, release-note drafting, incident response, onboarding, data analysis, or policy interpretation.

A good skill has:

- A clear trigger.
- Step-by-step operating guidance.
- Known pitfalls.
- Required tools.
- Expected output shape.
- Verification guidance.

### Subagents Are Bounded Contexts

Use subagents when the work needs a separate context, separate tools, separate safety boundary, or specialized judgment. Do not use them just to make the system look more agentic.

Good subagent boundaries:

- Researcher.
- Code reviewer.
- Data analyst.
- Support specialist.
- Finance approver.
- Security auditor.

Poor boundaries:

- "Think harder agent."
- "General helper."
- "Do everything agent."

### Channels Are Adapters

Channels should not contain business logic. Slack, web, Discord, email, and HTTP should adapt messages into the same agent behavior.

The same agent should be testable without the channel.

### Approvals Are Policy

Approval rules should be centralized and inspectable. If a tool spends money, writes to production, emails a customer, changes access, deletes data, or publishes externally, the approval requirement should be obvious from the file tree.

### Evals Are Tests

Every agent should ship with evals the way every Rails app ships with tests.

Minimum eval set:

- Golden path behavior.
- Refusal or escalation behavior.
- Tool selection behavior.
- Tone and policy adherence.
- Regression cases from production incidents.

## Generators

Rails won because `rails generate scaffold Post title:string body:text` collapsed a lot of ceremony.

Eve should have similarly opinionated generators:

```sh
npx eve generate agent support
npx eve generate tool refund_customer --approval required
npx eve generate skill handle_refund
npx eve generate subagent researcher
npx eve generate channel slack
npx eve generate eval refund_policy
```

Generators should create working files, tests/evals, examples, and safe defaults.

### CLI as the Framework Interface

If Eve is filesystem-first, the CLI is the operating interface for the filesystem. The CLI should not only scaffold files. It should encode the conventions, validate them, compose reusable parts, and make production readiness visible.

Proposed command families:

```sh
npx eve init support-agent
npx eve generate agent support
npx eve generate tool refund_customer
npx eve generate skill handle_refund
npx eve generate subagent researcher
npx eve generate channel slack
npx eve generate approval refund_customer
npx eve generate eval refund_policy
npx eve generate memory customer_profile

npx eve doctor
npx eve eval
npx eve test
npx eve preview
npx eve deploy
npx eve rollback
npx eve update
npx eve inspect
npx eve graph
```

The Rails parallel is important: `rails generate`, `rails test`, `rails routes`, `rails db:migrate`, and `rails server` make the app legible through commands. Eve needs the same command-level legibility for agents.

### Generator Flags

Generators should be useful in both human and machine workflows. A coding model should be able to call the same commands a senior engineer would call.

Common flags:

```sh
--agent support
--with-tools search_customers,create_ticket,refund_customer
--with-skills triage_customer_issue,handle_refund
--with-subagents researcher,policy_checker
--with-channels slack,web,http
--with-approvals required
--with-evals standard,policy,tone,tool-selection
--with-memory customer_profile
--env development
--model openai/gpt-5.5
--template customer-support
--dry-run
--force
--json
--yes
```

Specialized flags:

```sh
--approval never|on-risk|required
--risk low|medium|high
--side-effects none|read|write|external|money|production
--auth none|connect|oauth|api-key
--schedule "0 9 * * 1-5"
--channel slack
--visibility private|team|public
--owner support-platform
--cost-budget 10.00
--token-budget 200000
--timeout 10m
--version 1.2.0
--pin
--upgrade minor
```

The important design principle: flags should describe intent and risk, not only file output.

### Batch Generation

The unlock for GPT-5.5 or a future high-capability coding model is batch generation. A model should be able to turn a product map into a fleet of agents with reusable tools, skills, channels, approvals, evals, and memory schemas.

Proposed commands:

```sh
npx eve generate batch agents.yml
npx eve generate fleet agents.yml
npx eve plan agents.yml
npx eve apply agents.yml
```

Example batch manifest:

```yaml
defaults:
  model: openai/gpt-5.5
  channels: [web, slack]
  evals: [standard, tone, tool-selection]
  approvals: on-risk

shared:
  tools:
    - search_customers
    - create_ticket
    - lookup_policy
  skills:
    - triage_customer_issue
    - summarize_thread
  memory:
    - customer_profile

agents:
  - name: support
    responsibility: Resolve customer support requests.
    tools: [search_customers, create_ticket, lookup_policy]
    skills: [triage_customer_issue, summarize_thread]
    subagents: [policy_checker]

  - name: billing
    responsibility: Answer billing questions and prepare refund requests.
    tools: [search_customers, lookup_policy, prepare_refund]
    skills: [handle_refund]
    approvals:
      prepare_refund: required

  - name: growth
    responsibility: Draft campaign ideas from product updates.
    tools: [read_release_notes, draft_campaign]
    skills: [campaign_brief]
    channels: [web]
```

Batch generation should be two-phase:

1. `plan`: show what files, tools, approvals, evals, and connections will be created.
2. `apply`: write the files and produce a manifest that can be reviewed.

This keeps large-scale generation inspectable. A model can create one hundred agents, but a human or another agent should still be able to review the plan before the files land.

### Doctor

`doctor` should be the command that answers: "Can this agent safely run?"

Proposed checks:

- Required files exist.
- Tools compile and expose valid schemas.
- Skills have triggers and verification guidance.
- Subagents have bounded responsibilities.
- Channels do not contain business logic.
- Schedules are valid and have owners.
- Approval policies cover risky tools.
- Evals exist and pass.
- Memory schemas define retention and privacy behavior.
- Required connections are configured.
- Environment variables are present.
- Deployment target is reachable.
- Observability is enabled.
- Cost and token budgets are configured.
- Component versions resolve and match `versions.lock`.
- Hot-load compatibility is declared for any pending update.

Example:

```sh
npx eve doctor --agent support --env staging
npx eve doctor --all --json
npx eve doctor --updates
npx eve doctor --fix
```

`doctor --fix` should only apply safe mechanical repairs: missing placeholder eval files, invalid naming, stale generated indexes, or formatting. It should not silently weaken approval policy, add credentials, or bypass failing evals.

### Deploy

Deployment should be first-class because agents are operational systems, not static code artifacts.

Proposed deploy flow:

```sh
npx eve deploy --agent support --env staging
npx eve deploy --agent support --env production --require-evals --require-doctor
npx eve deploy --fleet agents.yml --env production --canary 10
```

Deployment gates:

- `doctor` passes.
- Evals pass.
- Approval coverage is complete.
- Connections are configured.
- Required secrets are present.
- Observability is enabled.
- Rollback target exists.

Useful deploy flags:

```sh
--require-evals
--require-doctor
--require-approvals
--canary 10
--promote
--rollback-to <deployment-id>
--dry-run
--json
```

The philosophy: local structure, generated manifests, and deployment checks should all agree about what the agent is allowed to do.

### Inspect and Graph

Large agent systems need maps.

```sh
npx eve inspect --agent support
npx eve graph --all
npx eve graph --agent support --format mermaid
```

`inspect` should summarize identity, tools, skills, subagents, approvals, memory, channels, schedules, eval status, owners, budgets, and deployment state.

`graph` should show composition:

```txt
support
  -> policy_checker
  -> researcher
  -> tools/search_customers
  -> tools/create_ticket
  -> skills/triage_customer_issue
  -> memory/customer_profile
```

This is the agent equivalent of `rails routes`: a command that makes hidden behavior visible.

## Environments

Agents need explicit environments:

- `development`: verbose logs, fake integrations, local fixtures.
- `test`: deterministic model settings, mocked tools, eval fixtures.
- `staging`: real auth with restricted permissions.
- `production`: durable workflows, observability, approval enforcement.

The philosophy should be: no agent reaches production with unknown tools, unknown memory behavior, or unknown approval boundaries.

## Composition

Composing agents should feel like composing routes and services:

```txt
main-agent/
  subagents/
    billing/
    support/
    research/
```

The parent agent should delegate by contract:

- Task.
- Inputs.
- Allowed tools.
- Expected output.
- Time or cost budget.
- Verification requirement.

Subagents should return structured results that the parent can inspect, summarize, reject, or ask to revise.

## Scaling

Rails scaled culturally before it scaled technically: it gave teams shared patterns. Eve can do the same for agents.

The proposed scaling model:

- One directory per agent.
- One bounded responsibility per agent.
- Shared skills for reusable procedures.
- Shared tools for stable domain capabilities.
- Shared evals for cross-agent policies.
- Explicit approval gates for side effects.
- Observable runs for debugging and product iteration.

## Agent Fleets

The Rails-inspired layer should assume that teams will not only build one agent. They will build agent fleets.

In that world, the framework needs shared catalogs:

```txt
catalog/
  tools/
    search_customers.ts
    create_ticket.ts
    lookup_policy.ts

  skills/
    triage_customer_issue.md
    summarize_thread.md
    handle_refund.md

  evals/
    standard.eval.ts
    tone.eval.ts
    tool_selection.eval.ts

  approvals/
    money_movement.ts
    external_publish.ts
    production_write.ts

  memory/
    customer_profile.ts
    account_context.ts

  channels/
    slack.ts
    web.ts
```

Agents should compose from the catalog rather than copy-paste capabilities. A batch generator should reference reusable parts by name and pin versions when needed.

```yaml
agents:
  - name: support
    uses:
      tools:
        - catalog/tools/search_customers@1
        - catalog/tools/create_ticket@2
      skills:
        - catalog/skills/triage_customer_issue@1
      evals:
        - catalog/evals/standard@1
        - catalog/evals/tone@1
```

This gives a high-capability coding model the right substrate. GPT-5.5 should not invent one hundred versions of `search_customers`. It should compose one well-reviewed tool into one hundred agents, with agent-specific instructions and evals around it.

## Versioning and Hot-Loading

Agent systems need versioning for the same reason Rails apps need migrations and dependency manifests: production behavior changes over time, and teams need to understand what changed, when it changed, whether it is compatible, and how to roll it back.

Versioning should apply at multiple levels:

- Agent version.
- Tool version.
- Skill version.
- Subagent version.
- Channel adapter version.
- Approval policy version.
- Eval suite version.
- Memory schema version.
- Runtime/framework version.

Proposed files:

```txt
agent/
  agent.manifest.yml
  versions.lock
  migrations/
    202607160101_add_customer_profile_memory.ts
    202607160102_upgrade_refund_policy_skill.ts
```

Example manifest:

```yaml
name: support
version: 1.4.0
runtime: eve@0.8.0
model: openai/gpt-5.5

uses:
  tools:
    search_customers: 2.1.0
    create_ticket: 1.3.0
    prepare_refund: 1.0.0
  skills:
    triage_customer_issue: 1.5.0
    handle_refund: 2.0.0
  approvals:
    money_movement: 1.2.0
  evals:
    standard: 1.0.0
    tone: 1.1.0
    refund_policy: 2.0.0
  memory:
    customer_profile: 1.0.0

compatibility:
  hot_load: true
  requires_restart: false
  min_runtime: eve@0.8.0
```

Example lockfile:

```yaml
resolved:
  catalog/tools/search_customers@2.1.0:
    digest: sha256:abc123
  catalog/skills/handle_refund@2.0.0:
    digest: sha256:def456
```

The manifest describes intent. The lockfile records exactly what is running.

### Semantic Versioning

Reusable agent parts should use semantic versioning:

- Patch: text fixes, safer validation, clearer errors, non-behavioral eval additions.
- Minor: backward-compatible capability additions.
- Major: changed behavior, changed schema, removed capability, stricter approval requirements, or altered memory semantics.

For agent systems, "breaking" should include behavior and policy changes, not only TypeScript API changes. If a skill changes when it escalates to a human, that may be a major version.

### Hot-Load Rules

Hot-loading should be explicit and conservative.

Safe to hot-load:

- Skill copy changes that preserve trigger, required tools, and output contract.
- Eval additions.
- Approval policy additions that make behavior stricter.
- Tool implementation patch fixes with the same schema and side-effect class.
- Channel adapter fixes that do not change message semantics.

Require restart or redeploy:

- Tool schema changes.
- New external permissions.
- Memory schema changes.
- Runtime version changes.
- Approval policy weakening.
- Schedule changes that increase autonomous activity.
- Subagent contract changes.

Unsafe without migration:

- Removing memory fields.
- Changing tool side effects.
- Changing a subagent's responsibility.
- Replacing a shared skill with different policy meaning.

### Update Commands

The CLI should make updates inspectable:

```sh
npx eve outdated
npx eve update --agent support
npx eve update --agent support --minor
npx eve update --agent support --pin handle_refund@2.0.0
npx eve update --fleet agents.yml --plan
npx eve update --fleet agents.yml --apply
npx eve hotload --agent support --skill handle_refund@2.0.1
```

Update flow:

1. Resolve available versions.
2. Show a diff of manifest, lockfile, generated files, approvals, evals, and memory schemas.
3. Run compatibility checks.
4. Run affected evals.
5. Apply hot-load, restart, or deploy based on compatibility.
6. Record update metadata.

Example output:

```txt
support
  skill handle_refund 2.0.0 -> 2.0.1
  compatibility: hot-loadable
  affected evals: refund_policy, tone
  approval change: none
  memory change: none
```

### Migrations

Agents need migrations for behavior and memory, not only databases.

Migration types:

- Memory schema migrations.
- Skill contract migrations.
- Tool schema migrations.
- Approval policy migrations.
- Channel message format migrations.
- Eval baseline migrations.

Commands:

```sh
npx eve generate migration add_customer_tier_to_memory
npx eve migrate --agent support --env staging
npx eve migrate --fleet agents.yml --env production --dry-run
```

The guiding rule: if an update changes persisted state, external contract, approvals, or observable behavior, it deserves a migration or a clearly recorded compatibility note.

### Rollback

Rollback should operate on versions, not only deployments.

```sh
npx eve rollback --agent support --to 1.3.2
npx eve rollback --agent support --component skill:handle_refund@2.0.0
npx eve rollback --fleet agents.yml --deployment dep_123
```

Rollback must restore:

- Agent manifest.
- Lockfile.
- Component versions.
- Deployment target.
- Approval policy.
- Eval baseline.
- Compatible memory schema, when possible.

If memory cannot be safely rolled back, the CLI should say so and require an explicit migration plan.

### Fleet Versioning

Fleet manifests should support version policies:

```yaml
version_policy:
  default: minor
  security: patch-auto
  approvals: pin
  memory: pin
  tools:
    money_movement: pin

agents:
  - name: support
    version: 1.4.0
    uses:
      skills:
        handle_refund: "^2.0.0"
      tools:
        search_customers: "~2.1.0"
      approvals:
        money_movement: "1.2.0"
```

Suggested defaults:

- Pin approvals.
- Pin memory schemas.
- Pin money-moving or production-writing tools.
- Allow patch updates for low-risk tools and skills.
- Allow minor updates only after evals pass.
- Require review for major updates.

This gives teams controlled freshness. Agents can receive safe hot fixes quickly without turning every update into a full redesign.

## Templates and Manifests

Use both YAML files and Jinja2 templates, but give them different jobs.

YAML should describe intent:

- Which agents exist.
- What each agent is responsible for.
- Which tools, skills, subagents, channels, approvals, evals, and memory schemas it uses.
- Which versions are pinned.
- Which environments and deployment policies apply.

Jinja2 should describe file generation:

- `instructions.md`.
- `agent.ts`.
- Tool wrappers.
- Skill markdown files.
- Eval files.
- Approval policies.
- Memory schemas.
- Channel adapters.
- README and operational docs.

The key rule:

> YAML is the source of truth. Jinja2 is the rendering layer.

Suggested layout:

```txt
templates/
  agent/
    instructions.md.j2
    agent.ts.j2
    tool.ts.j2
    skill.md.j2
    eval.ts.j2
    approval.ts.j2
    memory.ts.j2
    channel.ts.j2

manifests/
  agents.yml
  catalog.yml
  environments.yml
```

Example manifest:

```yaml
agents:
  - name: support
    responsibility: Resolve customer support requests.
    tone: calm, precise, helpful
    model: openai/gpt-5.5
    tools:
      search_customers: 2.1.0
      create_ticket: 1.3.0
    skills:
      triage_customer_issue: 1.5.0
    channels: [web, slack]
    approvals:
      create_ticket: on-risk
    evals: [standard, tone, tool-selection]
    memory:
      customer_profile: 1.0.0
```

Example Jinja2 template:

```jinja2
# Identity

You are the {{ agent.name }} agent.

## Responsibility

{{ agent.responsibility }}

## Tone

{{ agent.tone }}

## Operating Rules

{% for rule in agent.rules %}
- {{ rule }}
{% endfor %}
```

Rendering command:

```sh
npx eve generate batch manifests/agents.yml --templates templates/agent
npx eve render --agent support --dry-run
npx eve render --all --check
```

`render --check` should fail if generated files are stale, which keeps hand-edited drift from sneaking into production.

### Template Rules

Templates should be boring and strongly typed. Avoid putting business logic in Jinja2.

Good template logic:

- Loops over tools, skills, channels, and evals.
- Conditional sections for optional memory or approvals.
- Standard headers and metadata.
- Consistent import paths and naming.

Bad template logic:

- Complex policy decisions.
- Tool selection rules.
- Approval risk classification.
- Environment-specific secrets.
- Agent behavior that should live in YAML, skills, or instructions.

The generator should validate YAML with a schema before rendering templates. That gives consistency without hiding behavior in template conditionals.

### Why Not YAML Only?

YAML-only works for manifests, but it becomes awkward for long-form Markdown instructions, TypeScript tools, eval files, and docs. You either embed too much code inside YAML blocks or lose control over formatting.

Jinja2 keeps generated files readable while YAML keeps the system declarative.

### Why Not Jinja2 Only?

Jinja2-only makes it too easy for the template layer to become the real application logic. That weakens reviewability.

The safer model is:

- Review YAML for intent.
- Review templates for consistency.
- Review generated output for concrete behavior.

### Consistency Checks

`doctor` should verify template consistency:

```sh
npx eve doctor --templates
npx eve render --all --check
```

Checks should include:

- Manifest validates against schema.
- Templates render without undefined variables.
- Generated files match committed output.
- Generated files include metadata pointing back to the manifest and template version.
- No manual edits exist inside generated regions.
- Template versions match `versions.lock`.

## Fleet Safety

When a model can generate one hundred agents, the framework must make safe generation the easy path.

Fleet-level requirements:

- Every generated agent has an owner.
- Every generated agent has a bounded responsibility.
- Every tool is classified by side-effect level.
- Every risky side effect has an approval rule.
- Every agent has a minimum eval suite.
- Every memory schema has retention rules.
- Every channel adapter is logic-light.
- Every deployment has rollback metadata.
- Every generated file is traceable to a manifest entry.
- Every batch generation has a reviewable plan.
- Every reusable component has a version.
- Every running agent has a lockfile.
- Every hot-loaded change records compatibility metadata.
- Every generated file traces back to a YAML manifest and template version.

Proposed metadata:

```yaml
generated_by:
  command: eve generate batch agents.yml
  model: openai/gpt-5.5
  created_at: 2026-07-16T00:00:00Z
  manifest: agents.yml
  review_required: true
```

This turns model-generated software into reviewable software. The key is not preventing generation at scale. The key is making generation auditable at scale.

## Anti-Patterns

Avoid these early:

- Giant `instructions.md` files that contain every process.
- Tools that silently perform broad side effects.
- Skills without trigger conditions.
- Subagents without real boundaries.
- Channel-specific business logic.
- Agents deployed without evals.
- Memory with no retention or privacy policy.
- Prompts used where typed tools should exist.

## The Slogan

Rails said:

> Optimize for programmer happiness.

For Eve, the corresponding philosophy could be:

> Optimize for agent legibility.

If an engineer, product owner, security reviewer, or another agent can open the directory and quickly understand identity, capabilities, risks, tests, and operating surfaces, the framework is doing its job.

## Practical Next Step

Create a starter template that proves the philosophy:

```txt
rails-style-eve-template/
  agent/
    instructions.md
    agent.ts
    tools/
    skills/
    subagents/
    channels/
    schedules/
    approvals/
    evals/
    memory/
    fixtures/
  README.md
```

Then build one real internal agent with it. The philosophy should be judged by whether the second agent is faster, safer, and more legible than the first.
