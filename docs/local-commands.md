# Local Commands

Minimal command reference for this repo.

## One-time setup

### Worker dependencies

```bash
cd /Users/david.miller/Documents/current/test-cms-rewrite/workers/temporal_runner
uv venv
uv pip install -e .
```

Optional AI extras:

```bash
cd /Users/david.miller/Documents/current/test-cms-rewrite/workers/temporal_runner
uv pip install -e .[anthropic]
uv pip install -e .[vertex]
```

### Environment

Create a root `.env` from `.env.example` and set at least:

```env
DATABASE_URL=postgres://postgres:postgres@localhost:5432/rusty_cms
TEMPORAL_UI_URL=http://localhost:8233
TEMPORAL_GRPC_ENDPOINT=localhost:7233
TEMPORAL_NAMESPACE=default
TEMPORAL_RUNNER_PYTHON=/Users/david.miller/Documents/current/test-cms-rewrite/workers/temporal_runner/.venv/bin/python
TEMPORAL_RUNNER_START_SCRIPT=/Users/david.miller/Documents/current/test-cms-rewrite/workers/temporal_runner/start_workflow.py
TEMPORAL_RUNNER_RESULT_SCRIPT=/Users/david.miller/Documents/current/test-cms-rewrite/workers/temporal_runner/get_workflow_result.py
```

Migration crawler TLS options:

```env
CMS_MIGRATION_CA_BUNDLE=/absolute/path/to/corporate-ca.pem
CMS_MIGRATION_ALLOW_INSECURE_TLS=false
```

Notes:

- Prefer `CMS_MIGRATION_CA_BUNDLE` if your VPN or proxy injects a corporate cert.
- Only use `CMS_MIGRATION_ALLOW_INSECURE_TLS=true` as a fallback for local development.

## Database

Run migrations:

```bash
cd /Users/david.miller/Documents/current/test-cms-rewrite
cargo run -p cms-worker -- migrate
```

## Run the app

### Start the Temporal worker

From the repo root:

```bash
cd /Users/david.miller/Documents/current/test-cms-rewrite
workers/temporal_runner/.venv/bin/python workers/temporal_runner/worker.py
```

If `.env` is not loading yet, run it with explicit env:

```bash
TEMPORAL_GRPC_ENDPOINT=localhost:7233 \
TEMPORAL_NAMESPACE=default \
/Users/david.miller/Documents/current/test-cms-rewrite/workers/temporal_runner/.venv/bin/python \
/Users/david.miller/Documents/current/test-cms-rewrite/workers/temporal_runner/worker.py
```

### Start the Rust API

```bash
cd /Users/david.miller/Documents/current/test-cms-rewrite
cargo run -p cms-api
```

## Test

Run all Rust tests:

```bash
cd /Users/david.miller/Documents/current/test-cms-rewrite
cargo test
```

Run worker Python tests:

```bash
cd /Users/david.miller/Documents/current/test-cms-rewrite
python3 -m unittest discover -s workers/temporal_runner/tests
```

## Useful URLs

With the API running:

- `http://127.0.0.1:4000/viewer`
- `http://127.0.0.1:4000/migration-console`
- `http://127.0.0.1:4000/api/runtime`

With Temporal running:

- `http://localhost:8233`

## Typical local flow

1. Start Postgres and Temporal.
2. Run `cargo run -p cms-worker -- migrate`.
3. Start the Temporal worker.
4. Start `cargo run -p cms-api`.
5. Open `/migration-console`.
6. Create a migration.
7. Click `Sync discovery` after the workflow finishes.
8. Approve it if needed.
9. Import it into a draft change set.

## Current caveat

Migration workflow results can now be synced into the database from the console, but that sync is still manual rather than automatic.
