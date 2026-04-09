# test-cms-rewrite

Fresh headless CMS rewrite workspace.

Current direction:

- Rust backend for authoring APIs, rendering, publishing, pubsub, and workflow integration.
- Bun + TypeScript + Svelte for the management UI.
- Postgres as the system of record.
- Static publish target on persistent disk, with optional S3-backed object storage later.
- Async workflow orchestration via Temporal-managed jobs and agent-driven requests.

See [`docs/architecture.md`](/Users/david.miller/Documents/current/test-cms-rewrite/docs/architecture.md) and [`docs/roadmap.md`](/Users/david.miller/Documents/current/test-cms-rewrite/docs/roadmap.md).

Quick start:

- Copy `.env.example` to `.env` and stage your local settings.
- `cargo test`
- `cargo run -p cms-api`
- Open `http://127.0.0.1:4000/viewer` for the temporary server-render preview UI.

Environment loading:

- The Rust API and the Temporal Python runner both auto-load `.env`
- The API will fail fast if `CMS_REQUIRE_DATABASE=true` and `DATABASE_URL` is missing
- The Temporal runner will fail fast for invalid AI provider config such as `CMS_AI_PROVIDER=anthropic` without `ANTHROPIC_API_KEY`
- LangSmith tracing/evals can be gated with `CMS_AI_ENABLE_LANGSMITH`, `CMS_AI_ENABLE_LANGSMITH_TRACING`, and `CMS_AI_ENABLE_LANGSMITH_EVALS`

Useful API routes:

- `GET /api/runtime`
- `GET /api/sites`
- `GET /api/sites/:site_id/branches`
- `GET /api/sites/:site_id/branches/:branch_name/head`
- `POST /api/sites/:site_id/pages/:page_id/widget-commands`
- `POST /api/sites/:site_id/workflow-requests`
- `POST /api/sites/:site_id/workflow-requests/trigger`
- `GET /api/widget-definitions`
- `GET /api/widget-definitions/:slug/versions`
- `POST /api/widget-sources/import-local`

Temporal defaults:

- UI: `http://localhost:8233`
- gRPC endpoint for workers/SDKs: `localhost:7233`

Temporal worker staging:

- Worker package: [workers/temporal_runner/README.md](/Users/david.miller/Documents/current/test-cms-rewrite/workers/temporal_runner/README.md)
- Start the worker with `.venv/bin/python worker.py`
- Set `TEMPORAL_RUNNER_PYTHON` to the worker virtualenv interpreter before starting `cms-api`
- Trigger executions via `POST /api/sites/:site_id/workflow-requests/trigger`

Widget source import:

- Use `POST /api/widget-sources/import-local` with a local repo path
- Example path in this workspace context: `../cms-widget-floor-plans-plus`
