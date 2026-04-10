# Temporal Runner

This worker uses the official Temporal Python SDK to host a minimal workflow
runner for `rusty-cms`.

## Setup

```bash
cd workers/temporal_runner
uv venv
uv pip install -e .
```

Install optional AI adapters as needed:

```bash
uv pip install -e .[anthropic]
uv pip install -e .[vertex]
```

## Run worker

```bash
TEMPORAL_GRPC_ENDPOINT=localhost:7233 \
TEMPORAL_NAMESPACE=default \
.venv/bin/python worker.py
```

This starts workers for:

- `cms-publish`
- `cms-restore`
- `cms-bulk`
- `cms-migrations`
- `cms-agent-ops`

## Trigger from API

Point the Rust API at the virtualenv interpreter:

```bash
export TEMPORAL_RUNNER_PYTHON="$PWD/workers/temporal_runner/.venv/bin/python"
export TEMPORAL_RUNNER_RESULT_SCRIPT="$PWD/workers/temporal_runner/get_workflow_result.py"
cargo run -p cms-api
```

Then `POST /api/sites/:site_id/workflow-requests/trigger` with a valid workflow
request payload. Migration discovery results can be pulled back into Postgres via
`POST /api/migrations/:migration_id/sync-discovery`.

## AI workflow runtime

`AiContentOperation` requests now execute in a Temporal activity rather than in
the workflow body. That keeps provider I/O deterministic from Temporal's point
of view and gives the runner a clean seam for Anthropic now and Vertex later.

Useful environment variables:

- `CMS_AI_PROVIDER=mock|anthropic|vertex`
- `CMS_AI_ANTHROPIC_MODEL=claude-sonnet-4-5-20250929`
- `ANTHROPIC_API_KEY=...`
- `CMS_AI_VERTEX_PROJECT=...`
- `CMS_AI_VERTEX_LOCATION=us-central1`
- `CMS_AI_VERTEX_MODEL=gemini-2.5-flash`
- `CMS_AI_ENABLE_LANGSMITH=false`
- `CMS_AI_ENABLE_LANGSMITH_TRACING=false`
- `CMS_AI_ENABLE_LANGSMITH_EVALS=false`
- `LANGSMITH_TRACING=true`
- `LANGSMITH_API_KEY=...`
- `LANGSMITH_PROJECT=rusty-cms`
- `CMS_MIGRATION_CA_BUNDLE=/absolute/path/to/corporate-ca.pem`
- `CMS_MIGRATION_ALLOW_INSECURE_TLS=false`

For migration crawling behind a corporate VPN or proxy:

- prefer `CMS_MIGRATION_CA_BUNDLE` so HTTPS verification still works with your corporate root CA
- use `CMS_MIGRATION_ALLOW_INSECURE_TLS=true` only as a local fallback when you cannot install the CA bundle yet

Current retrieval/eval posture:

- `retrieval.mode=inline` is wired today via `context_documents` or
  `retrieval.documents` in `input_payload`
- `retrieval.mode=vertex_rag` is modeled but intentionally not implemented yet
- `evaluation.provider=langsmith` emits evaluation metadata, and Anthropic runs
  can be traced through LangSmith when the optional package and env vars are set
