# Basic Fleet Example

This example contains a small Eve Rails fleet with two generated Eve agents:

- `support`
- `billing`

The fleet also includes one reusable schedule, `weekday_triage`, rendered to `agent/schedules/weekday_triage.ts` with an Eve `defineSchedule` cron.

It is meant for local smoke tests, documentation, and validating generated Eve-compatible output.

## Plan The Fleet

From the repository root:

```sh
cargo run -- plan examples/basic-fleet/manifests/agents.yml \
  --catalog examples/basic-fleet/manifests/catalog.yml
```

## Run Doctor

```sh
cargo run -- doctor --all \
  --manifest examples/basic-fleet/manifests/agents.yml \
  --catalog examples/basic-fleet/manifests/catalog.yml
```

## Inspect And Graph

```sh
cargo run -- inspect --agent support \
  --manifest examples/basic-fleet/manifests/agents.yml \
  --catalog examples/basic-fleet/manifests/catalog.yml

cargo run -- graph --all \
  --manifest examples/basic-fleet/manifests/agents.yml \
  --catalog examples/basic-fleet/manifests/catalog.yml \
  --format mermaid
```

## Test A Generated Eve Agent

```sh
cd examples/basic-fleet/agents/support
npm install
npm run typecheck
npm exec -- eve info --json
npm exec -- eve dev --no-ui
```

For a live model call, configure Vercel AI Gateway credentials first:

```sh
export AI_GATEWAY_API_KEY="..."
# or
npm exec -- eve link
```

Then create a session against the local Eve dev server:

```sh
curl -sS -X POST http://127.0.0.1:2000/eve/v1/session \
  -H 'content-type: application/json' \
  -d '{"message":"Confirm this generated Eve agent is alive."}'
```

Use the returned `sessionId` and `continuationToken` with:

```txt
GET /eve/v1/session/:sessionId/stream?continuationToken=...
```
