# Roadmap

Current delivery status lives in [`docs/implementation-plan.md`](/Users/david.miller/Documents/current/test-cms-rewrite/docs/implementation-plan.md).

## Phase 0

- Establish Rust workspace and crate boundaries.
- Add baseline CI commands:
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features`
  - `cargo test`
- Add Bun workspace later when UI starts.

## Phase 1: Domain skeleton

- Add Postgres migrations for:
  - `sites`
  - `branches`
  - `snapshots`
  - `pages`
  - `page_documents`
  - `component_sources`
  - `widget_definitions`
  - `widget_definition_versions`
  - `widget_instances`
  - `publish_jobs`
  - `workflow_requests`
- Define typed DTOs and API envelopes.
- Add JSON Schema for page documents and workflow requests.

## Phase 2: Snapshot authoring

- Create branch and snapshot APIs.
- Implement page CRUD against JSONB page documents.
- Implement widget definition registry reads and widget instance CRUD.
- Add hierarchy operations:
  - move page
  - copy subtree
  - restore snapshot
  - bulk apply subtree to many sites
  - apply template site snapshot to derived sites

## Phase 3: Rendering and publishing

- Add component package loader.
- Add theme and layout resolution.
- Add widget definition resolution from built-in and repo-backed sources.
- Render one page from one snapshot.
- Expand to full-site snapshot builds.
- Write publish manifest and perform atomic release promotion on disk.
- Emit publish lifecycle pubsub events.

## Phase 4: Durable pubsub

- Add Postgres outbox tables.
- Add dispatcher worker.
- Add idempotent subscribers.
- Add delivery monitoring and dead-letter handling.

## Phase 5: Temporal and agent workflows

- Define workflow request schema and safety policy.
- Add Temporal-owned workflows for:
  - publish site
  - bulk copy content
  - restore snapshot
  - site migration
  - ai-assisted content operation
- Add runtime admission rules for Rust, Bun/TypeScript, and Python workers.
- Validate workflow outputs before they can create snapshots.

## Phase 5.5: Migration pipeline

- Add crawl-first site discovery workflow.
- Add widget signature registry for legacy output detection.
- Add optional legacy CMS API enrichment for widget-specific recovery.
- Add migration review artifacts and approval flow.
- Import approved migrations into draft snapshots.

## Phase 6: UI

- Create Bun + SvelteKit app.
- Add auth, site navigation, page tree, editor shell, publish controls, workflow history.
- Generate client types from OpenAPI or shared schema artifacts.

## First production target

The first serious target should be narrow:

- one site type
- one theme
- a small component catalog
- one publish target on local persistent disk
- reliable publish rollback
- workflow-backed publish and restore

That is enough to prove the architecture before migrating the current CMS feature surface.
