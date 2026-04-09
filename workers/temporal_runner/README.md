# Temporal Runner

This worker uses the official Temporal Python SDK to host a minimal workflow
runner for `rusty-cms`.

## Setup

```bash
cd workers/temporal_runner
uv venv
uv pip install -e .
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
- `cms-agent-ops`

## Trigger from API

Point the Rust API at the virtualenv interpreter:

```bash
export TEMPORAL_RUNNER_PYTHON="$PWD/workers/temporal_runner/.venv/bin/python"
cargo run -p cms-api
```

Then `POST /api/sites/:site_id/workflow-requests/trigger` with a valid workflow
request payload.

