#!/usr/bin/env bash
set -euo pipefail
trap 'echo "CLI acceptance failed near line ${LINENO}: ${BASH_COMMAND}" >&2' ERR

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="$ROOT/target/debug/eve-rails-cli"

cd "$ROOT"

cargo build --locked --bin eve-rails-cli

echo "== help =="
set -x
"$BIN" --help >/dev/null
for cmd in init plan apply render doctor generate outdated update hotload deploy eval test preview migrate rollback inspect graph; do
  "$BIN" "$cmd" --help >/dev/null
done
for sub in agent tool skill subagent channel schedule approval eval memory batch migration; do
  "$BIN" generate "$sub" --help >/dev/null
done

echo "== temp project commands =="
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

"$BIN" init "$TMP/demo-json" --template basic --model openai/gpt-5.5 --owner cli-test --yes --dry-run --json >/dev/null
"$BIN" init "$TMP/demo" --template basic --model openai/gpt-5.5 --owner cli-test --yes >/dev/null
(
  cd "$TMP/demo"
  "$BIN" generate tool refund_customer --side-effects money --dry-run --json >/dev/null
  "$BIN" generate tool refund_customer --side-effects money --json >/dev/null
  "$BIN" generate skill handle_refund --json >/dev/null
  "$BIN" generate subagent researcher --json >/dev/null
  "$BIN" generate channel slack --json >/dev/null
  "$BIN" generate schedule weekday_triage --schedule "0 9 * * 1-5" --json >/dev/null
  "$BIN" generate approval refund_customer --json >/dev/null
  "$BIN" generate eval refund_policy --json >/dev/null
  "$BIN" generate memory customer_profile --retention 180d --json >/dev/null
  "$BIN" generate migration customer_profile_v2 --json >/dev/null
  "$BIN" generate agent support \
    --owner cli-test \
    --model openai/gpt-5.5 \
    --description "Support smoke agent." \
    --with-tools refund_customer \
    --with-skills handle_refund \
    --with-subagents researcher \
    --with-channels slack \
    --with-schedules weekday_triage \
    --with-evals refund_policy \
    --with-memory customer_profile \
    --approval required \
    --auth http-basic-env \
    --visibility internal \
    --cost-budget 10 \
    --token-budget 100000 \
    --timeout 30s \
    --json >/dev/null
  "$BIN" plan manifests/agents.yml --json >/dev/null
  "$BIN" apply manifests/agents.yml --json >/dev/null
  "$BIN" render --all --check >/dev/null
  "$BIN" doctor --all --templates --updates --env production --connections --budgets >/dev/null
  "$BIN" doctor --all --templates --updates --fix >/dev/null
  "$BIN" outdated manifests/agents.yml --json >/dev/null
  "$BIN" update --agent support --minor --plan --json >/dev/null
  "$BIN" hotload --agent support --current 1.0.0 skill:handle_refund@1.0.1 --json >/dev/null
  "$BIN" deploy --agent support --env production --require-evals --require-doctor --require-approvals --dry-run --json >/dev/null
  "$BIN" eval --agent support --dry-run --json >/dev/null
  "$BIN" test --agent support --dry-run --json >/dev/null
  "$BIN" preview --agent support --dry-run --json >/dev/null
  "$BIN" migrate --agent support --env production --dry-run --json >/dev/null
  "$BIN" rollback --agent support --to 1.0.0 --json >/dev/null
  "$BIN" inspect --agent support --json >/dev/null
  "$BIN" graph --all --format json >/dev/null
)

echo "CLI acceptance sweep passed."
