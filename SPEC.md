# Rails-Style Eve CLI Layer Spec

## Purpose

Build a Rails-inspired convention layer for Eve that lets teams generate, compose, validate, version, update, deploy, and operate many agents consistently.

The philosophy is captured in `RAILS_PHILOSOPHY_FOR_EVE.md`. This spec turns that philosophy into buildable work.

## Goals

- Generate agents from YAML manifests and Jinja2 templates.
- Compose agents from reusable tools, skills, subagents, channels, approvals, evals, and memory schemas.
- Support batch generation for many agents.
- Keep generated code consistent and reviewable.
- Validate agents with `doctor`.
- Track component versions with manifests and lockfiles.
- Support safe updates, hot-loading, migrations, deployment checks, and rollback.
- Make large agent fleets inspectable through CLI output and graphs.

## Non-Goals

- Replacing Eve's core runtime.
- Building a new model provider abstraction.
- Building a custom deployment platform.
- Hiding generated code from review.
- Letting templates contain business logic.

## Core Commands

### Generate

```sh
npx eve generate agent support
npx eve generate tool refund_customer
npx eve generate skill handle_refund
npx eve generate subagent researcher
npx eve generate channel slack
npx eve generate approval refund_customer
npx eve generate eval refund_policy
npx eve generate memory customer_profile
```

### Batch

```sh
npx eve plan manifests/agents.yml
npx eve apply manifests/agents.yml
npx eve generate batch manifests/agents.yml
```

In this repository, runnable demo assets live under `examples/basic-fleet/`.
The root-level `manifests/` and `agents/` paths remain the convention for a user's own project.

### Render

```sh
npx eve render --agent support
npx eve render --all --check
```

### Validate

```sh
npx eve doctor
npx eve doctor --all
npx eve doctor --updates
npx eve doctor --templates
npx eve doctor --env production --connections --budgets
npx eve doctor --fix
```

### Version and Update

```sh
npx eve outdated
npx eve update --agent support --minor
npx eve hotload --agent support --skill handle_refund@2.0.1
npx eve rollback --agent support --to 1.3.2
```

### Deploy and Operate

```sh
npx eve deploy --agent support --env staging
npx eve deploy --agent support --env production --require-evals --require-doctor --require-approvals --dry-run
npx eve eval --agent support --dry-run
npx eve test --agent support
npx eve preview --agent support --dry-run
npx eve migrate --agent support --env production --dry-run
npx eve inspect --agent support
npx eve graph --all --format mermaid
```

## File Structure

```txt
.
  examples/
    basic-fleet/
      manifests/
      fixtures/
      agents/

  manifests/
    agents.yml
    catalog.yml
    environments.yml

  templates/
    agent/
      instructions.md.j2
      agent.ts.j2
      tool.ts.j2
      skill.md.j2
      schedule.ts.j2
      eval.ts.j2
      approval.ts.j2
      memory.ts.j2
      fixture.json.j2
      agent.README.md.j2
      channel.ts.j2

  catalog/
    tools/
    skills/
    evals/
    approvals/
    memory/
    channels/

  agents/
    support/
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
        agent.manifest.yml
        versions.lock
        migrations/
```

## Manifest Schema

Minimum agent manifest fields:

```yaml
agents:
  - name: support
    version: 1.0.0
    owner: support-platform
    responsibility: Resolve customer support requests.
    model: openai/gpt-5.5
    tools: {}
    skills: {}
    subagents: []
    channels: []
    approvals: {}
    evals: []
    memory: {}
```

Validation rules:

- `name`, `version`, `owner`, `responsibility`, and `model` are required.
- Names use lowercase kebab-case or snake_case consistently.
- All referenced catalog components must exist.
- All referenced versions must resolve.
- Risky tools must have approval policies.
- Memory schemas must declare retention.
- Production agents must have evals.

## Template Rules

- YAML is the source of truth.
- Jinja2 is the rendering layer.
- Templates must not contain business policy.
- Templates must fail on undefined variables.
- Generated files must include metadata pointing to manifest and template versions.
- `render --check` must fail when generated files are stale.

## Versioning Rules

- Every reusable component has a semantic version.
- Every generated agent has `agent.manifest.yml`.
- Every generated agent has `versions.lock`.
- Patch updates may be hot-loadable.
- Minor updates require affected evals.
- Major updates require explicit review.
- Memory schema changes require migrations.
- Approval weakening requires explicit review and redeploy.

## Doctor Checks

`doctor` passes only when:

- Required files exist.
- Manifest validates.
- Templates render.
- Generated output is fresh.
- Tools compile and expose valid schemas.
- Skills have triggers and verification guidance.
- Subagents have bounded responsibilities.
- Channels are adapter-only.
- Schedules have owners.
- Risky tools have approval coverage.
- Evals exist and pass where required.
- Memory schemas define retention.
- Connections and environment variables are present.
- Observability is enabled for deploy targets.
- Versions resolve and match `versions.lock`.
- Hot-load compatibility is declared for pending updates.

## Hot-Load Policy

Hot-load allowed:

- Skill text patch preserving trigger, required tools, and output contract.
- Eval additions.
- Approval policy additions that make behavior stricter.
- Tool implementation patch with same schema and side-effect class.

Hot-load denied:

- Tool schema changes.
- New external permissions.
- Memory schema changes.
- Runtime upgrades.
- Approval weakening.
- Schedule changes increasing autonomous activity.
- Subagent contract changes.

## PR Plan

### PR 1: Project Skeleton and Manifest Parser

Deliverables:

- CLI package skeleton.
- Manifest schema.
- YAML parser.
- Validation errors with readable messages.
- Fixture manifests.

Checklist:

- `plan` loads `manifests/agents.yml`.
- Repository demo coverage loads `examples/basic-fleet/manifests/agents.yml`.
- Invalid manifests fail with actionable errors.
- Component references resolve against `catalog.yml`.
- Unit tests cover required fields and missing components.

### PR 2: Jinja2 Rendering

Deliverables:

- Template loader.
- Strict rendering mode.
- Generated metadata headers.
- `render --agent` and `render --all --check`.

Checklist:

- Undefined template variables fail.
- Rendering creates deterministic output.
- `render --check` catches stale files.
- Snapshot tests cover generated files.

### PR 3: Generators

Deliverables:

- `generate agent`.
- `generate tool`.
- `generate skill`.
- `generate eval`.
- `generate approval`.
- `generate memory`.
- Common generator flags.

Checklist:

- Generators produce valid manifest entries and files.
- `--dry-run` prints planned changes.
- `--json` returns machine-readable output.
- `--force` behavior is explicit and tested.

### PR 4: Batch Plan and Apply

Deliverables:

- `plan manifests/agents.yml`.
- `apply manifests/agents.yml`.
- Batch generation report.

Checklist:

- Plan shows files to create, update, and skip.
- Apply writes only planned files.
- Generated files trace back to manifest and template versions.
- Batch fixtures cover at least 10 agents.

### PR 5: Doctor

Deliverables:

- `doctor`.
- `doctor --all`.
- `doctor --templates`.
- `doctor --updates`.

Checklist:

- Doctor reports pass/fail per check.
- Doctor exits nonzero on failure.
- Doctor emits JSON for automation.
- Risky tools without approvals fail.
- Stale generated files fail.

### PR 6: Versioning and Lockfile

Deliverables:

- `agent.manifest.yml`.
- `versions.lock`.
- Version resolver.
- `outdated`.
- Update planning.

Checklist:

- Lockfile records exact component versions and digests.
- Version ranges resolve deterministically.
- Outdated reports patch/minor/major updates.
- Updates show affected agents and evals.

### PR 7: Hot-Load and Migrations

Deliverables:

- Hot-load compatibility classifier.
- `hotload`.
- Migration generator.
- Migration dry-run.

Checklist:

- Safe patch changes can hot-load.
- Unsafe changes require redeploy or migration.
- Memory schema changes require migration.
- Approval weakening cannot hot-load.

### PR 8: Deploy Gates and Rollback

Deliverables:

- Deploy preflight checks.
- `deploy --require-evals --require-doctor`.
- Rollback by agent version.
- Rollback by component version.

Checklist:

- Deploy fails when doctor fails.
- Deploy fails when evals fail.
- Rollback restores manifest and lockfile.
- Unsafe memory rollback requires explicit migration.

### PR 9: Inspect and Graph

Deliverables:

- `inspect --agent`.
- `graph --all`.
- Mermaid output.
- JSON output.

Checklist:

- Inspect summarizes identity, tools, skills, approvals, evals, memory, versions, channels, schedules, and deployment state.
- Graph shows agent composition.
- JSON output can feed dashboards or CI.

### PR 10: Repo and CLI Hygiene

Deliverables:

- Updated post-v1 implementation checklist.
- Help text without stale PR references.
- Changelog entries for v1.1.
- Philosophy gap status.

Checklist:

- `eve-rails-cli --help` has no stale PR references.
- `apply` describes render/apply behavior.
- `deploy` describes preflight and Eve/Vercel delegation.

### PR 11: Doctor Safety Expansion

Deliverables:

- `doctor --fix`.
- `doctor --env <name>`.
- `doctor --connections`.
- `doctor --budgets`.
- Environment and observability checks.
- Schedule owner/auth/visibility checks.
- Budget validation.

Checklist:

- Doctor JSON and text output include the new checks.
- `--fix` reports planned safe mechanical repairs before writing.
- Doctor rejects unsafe missing approval, schedule, budget, and production observability states.

### PR 12: Complete Agent Template Rendering

Deliverables:

- Generated `agent/tools/`.
- Generated `agent/skills/`.
- Generated `agent/subagents/`.
- Generated `agent/channels/`.
- Generated `agent/schedules/`.
- Generated `agent/approvals/`.
- Generated `agent/evals/` contracts.
- Generated top-level Eve `evals/*.eval.ts`.
- Generated `agent/memory/`.
- Generated `agent/fixtures/`.
- Generated `agent/README.md`.

Checklist:

- Example fleet renders all expected directories.
- Generated support agent passes `npm run typecheck`.
- Generated support agent returns ready from `npm exec -- eve info --json`.
- `render --all --check` detects stale generated files.

### PR 13: Init and Example Project Scaffolding

Deliverables:

- `init <name>`.
- `--template basic`.
- `--template customer-support`.
- `--model`.
- `--owner`.
- `--yes`.
- `--dry-run`.
- `--json`.
- `--force`.

Checklist:

- Init never overwrites existing files unless `--force`.
- Init output can run `plan`, `doctor`, and `render`.
- JSON output lists planned changes.

### PR 14: Eval/Test/Preview Delegation

Deliverables:

- `eval`.
- `test`.
- `preview`.
- `--agent`.
- `--manifest`.
- `--catalog`.
- `--env`.
- `--json`.
- `--dry-run`.

Checklist:

- Missing agent package gives actionable errors.
- `test` runs `npm run typecheck` and `npm exec -- eve info --json`.
- `eval --dry-run` reports the delegated Eve command without credentials.
- `preview --dry-run` documents the `eve dev --no-ui` command.

### PR 15: Migrate Command

Deliverables:

- `migrate --agent <name> --env <env> --dry-run`.
- `migrate --fleet <manifest> --env <env> --dry-run`.
- `migrate --apply`.

Checklist:

- Dry-run migration plans are JSON-compatible.
- Pending migration files are listed per agent.
- Migration status changes only happen with `--apply`.

### PR 16: Version Policy and Compatibility Metadata

Deliverables:

- `version_policy` manifest support.
- Generated compatibility metadata.
- Runtime version recorded in generated manifests and lockfiles.
- Hot-load classifier treats schedule activity increases as redeploy-only.

Checklist:

- Patch/minor/major resolution follows policy.
- Approvals and memory can be pinned.
- Schedule changes that increase autonomous activity are not hot-loadable.

### PR 17: Deployment Delegation and Release Hardening

Deliverables:

- `deploy --require-approvals`.
- `deploy --promote`.
- `deploy --rollback-to <deployment-id>`.
- `deploy --dry-run`.
- Non-dry deploy delegation to `npm exec -- eve deploy`.

Checklist:

- Dry-run reports gates and delegated command.
- Production deploy requires doctor/evals/approvals unless explicitly relaxed outside production.
- Tests never perform accidental production deploys.

## Philosophy Gap Status

- Implemented: manifest/catalog validation, strict rendering, generators, batch plan/apply, doctor diagnostics, lockfiles, version reports, hot-load classification, migration generation/planning, deploy preflight/delegation, rollback planning, inspect, graph, schedules, init scaffolding, runtime eval/test/preview delegation.
- Implemented as Eve-native: `agent.ts`, instructions, tools, skills, subagents, channels, schedules, top-level evals, package files, and TypeScript checks.
- Implemented as Rails-layer conventions: approvals, memory, fixtures, and `agent/evals/` contract metadata. Eve currently ignores these folders when placed directly under `agent/`, so generated metadata is explicit that these are framework conventions until Eve consumes them.
- Partial: `doctor --fix` reports safe repair plans; destructive or policy-changing fixes remain intentionally unsupported.
- Partial: deployment delegates to Eve but does not replace Vercel release management.
- Partial: migrations are planned and status-tracked, not executed destructively by default.

## Metrics

### Build Metrics

- Time to generate first valid agent.
- Time to generate 100 valid agents from a manifest.
- Percentage of generated files passing `render --check`.
- Number of manual edits inside generated regions.

### Quality Metrics

- Doctor pass rate.
- Eval pass rate.
- Percentage of risky tools with approval coverage.
- Percentage of agents with owners.
- Percentage of memory schemas with retention policies.
- Percentage of generated agents with lockfiles.

### Operational Metrics

- Deployment success rate.
- Rollback success rate.
- Hot-load success rate.
- Mean time to update a shared skill across all affected agents.
- Number of agents affected by a catalog component update.

### Safety Metrics

- Approval bypass count.
- Production-write tools without approval.
- External-publish tools without approval.
- Failed hot-load compatibility checks.
- Memory migration failures.

### Developer Experience Metrics

- CLI command success rate.
- Average time from manifest change to generated output.
- Average time from generated output to passing doctor.
- Number of actionable error messages versus generic failures.

## Acceptance Criteria

The first production-ready milestone is complete when:

- A single YAML manifest can generate at least 10 agents.
- Generated agents include tools, skills, approvals, evals, memory, manifests, and lockfiles.
- `render --all --check` passes.
- `doctor --all` passes.
- A shared skill patch can be planned, applied, evaluated, and hot-loaded.
- A major skill change is blocked from hot-loading.
- A deploy preflight fails correctly when evals or doctor fail.
- `inspect` and `graph` make the fleet understandable.

## Open Questions

- Should this be implemented as a wrapper around `eve`, a contribution to Eve, or a companion package?
- Should generated files be committed, regenerated in CI, or both?
- Should templates be user-editable per repo or pulled from a versioned registry?
- Should catalog components live in the same repo, a package registry, or a dedicated marketplace?
- What is the minimum viable eval API for the first milestone?
