# Eve Rails CLI Roadmap

Eve Rails CLI is becoming the compiler and operating interface for an agentic company: declarative manifests in, Eve filesystem agents out, with validation, inspection, graphing, dry-runs, and deployment gates around the result.

This roadmap captures the next improvements discovered while modeling a 20-department, 150-role cognitive agentic digital company.

## 1. First-Class Subagents

Today subagents are rendered from a simple list on each parent agent. That is enough for filesystem structure, but not enough for rich role modeling.

- Add manifest support for subagent objects with `name`, `title`, `role_id`, `responsibility`, `model`, `tools`, `skills`, `memory`, `channels`, `approvals`, and runtime policy.
- Allow parent agents to declare principal subagents and delegate subagents explicitly.
- Render role-specific `instructions.md` and `agent.ts` from subagent metadata.
- Include subagents in `inspect`, `graph`, `doctor`, lockfiles, and update reports as first-class entities.

## 2. Runtime Policy And Sandbox Profiles

Runtime policy currently lives in custom metadata. It should become a validated CLI concept.

- Add a typed `runtime_policy` schema for agents and subagents.
- Support sandbox profiles such as `read-only`, `network-read`, `workspace-write`, `external-write-gated`, `production-write-gated`, and `approval-gated`.
- Validate that granted tools are compatible with the selected sandbox.
- Validate that non-read tools have explicit approval coverage.
- Render runtime boundaries into generated instructions and machine-readable manifests.

## 3. Tool Contract Maturity

The generated company needs tool families as contracts before dangerous implementations are added.

- Support catalog tool categories such as `artifact_read`, `workspace_write`, `github_issue`, `github_pr_review`, `figma_inspect`, `design_corpus_read`, `customer_ticket_read`, `analytics_query`, `run_local_check`, `production_change_request`, and `contract_review_request`.
- Add tool metadata for side effects, required approvals, required environment variables, required connectors, and sandbox compatibility.
- Generate safer placeholder implementations that describe inputs, outputs, side effects, and failure modes.
- Extend `doctor` to validate tool metadata against runtime policy and environment configuration.

## 4. Department And Role Templates

Templates should express organizational topology, not only generic identity.

- Add dedicated templates for parent department agents and role subagents.
- Render principal roles, delegate roles, routing rules, escalation rules, allowed tools, forbidden actions, and expected output shape.
- Allow template selection by department, role family, sandbox, or risk level.
- Keep generated files deterministic so `render --check` remains useful.

## 5. Channels As Intake Surfaces

Channels should act as adapters, not business logic.

- Model department-specific intake surfaces such as Slack, GitHub, Figma, Jira/Linear, email, and HTTP.
- Add channel metadata for allowed users, allowed channels, trigger rules, visibility, and injected context.
- Validate connector requirements with `doctor --env`.
- Render channel adapters that preserve privacy boundaries and route work to the right parent agent.

## 6. Real Evals

Current evals are skeletons. The company needs tests that prove delegation and safety behavior.

- Add standard eval suites for delegation, subagent scope, approval escalation, tool selection, refusal, and handoff quality.
- Generate department-aware eval fixtures from the manifest.
- Validate that risky departments have relevant eval coverage.
- Make deploy gates require passing evals, not only eval references.

## 7. Policy-Aware Inspect And Graph

`inspect` and `graph` should explain the operating model, not only list components.

- Show sandbox, allowed tools, approval gates, forbidden actions, channels, and eval coverage.
- Include subagent-level detail in `inspect`.
- Add graph modes for topology, tools, approvals, runtime policy, and channels.
- Export machine-readable reports for dashboards and review workflows.

## 8. Dry-Run And Deploy Gates

Dry-runs are already valuable. They should become the default review workflow before any risky action.

- Improve deploy dry-run output with exact failed doctor checks.
- Add `--env staging` defaults during project init.
- Validate deployment readiness per target agent rather than always scanning the full fleet when appropriate.
- Add preflight checks for secrets, connectors, corpus approval, eval pass status, and rollback metadata.

## 9. Normalization Pipeline

The company manifest should be regenerated from source material without losing intent.

- Keep source extractors and normalizers as repeatable scripts.
- Emit both CLI-native fields and richer metadata for future CLI features.
- Validate source counts, ID corrections, slug stability, and topology inference.
- Add tests for normalizer output so department and role policies stay stable.

## 10. Operating Interface

The CLI should answer the core questions of an agentic company.

- What agents exist?
- What can each agent do?
- Who can each agent delegate to?
- What tools are allowed?
- What requires approval?
- What is safe to hotload?
- What is deployable?
- What changed since the last generated version?

The goal is not more scaffolding for its own sake. The goal is a boring, inspectable operating system for agent fleets.
