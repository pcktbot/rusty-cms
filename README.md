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

- `cargo test`
- `cargo run -p cms-api`
- Open `http://127.0.0.1:4000/viewer` for the temporary server-render preview UI.

Useful API routes:

- `GET /api/runtime`
- `GET /api/sites`
- `GET /api/sites/:site_id/branches`
- `GET /api/sites/:site_id/branches/:branch_name/head`
- `POST /api/sites/:site_id/pages/:page_id/widget-commands`
- `POST /api/sites/:site_id/workflow-requests`
- `GET /api/widget-definitions`
- `GET /api/widget-definitions/:slug/versions`

Temporal defaults:

- UI: `http://localhost:8233`
- gRPC endpoint for workers/SDKs: `localhost:7233`
