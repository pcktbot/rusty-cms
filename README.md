# test-cms-rewrite

Fresh headless CMS rewrite workspace.

Current direction:

- Rust backend for authoring APIs, rendering, publishing, pubsub, and workflow integration.
- Bun + TypeScript + Svelte for the management UI.
- Postgres as the system of record.
- Static publish target on persistent disk, with optional S3-backed object storage later.
- Async workflow orchestration via Temporal-managed jobs and agent-driven requests.

See [`docs/architecture.md`](/Users/david.miller/Documents/current/test-cms-rewrite/docs/architecture.md) and [`docs/roadmap.md`](/Users/david.miller/Documents/current/test-cms-rewrite/docs/roadmap.md).

