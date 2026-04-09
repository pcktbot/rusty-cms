# Architecture

## Goals

- Headless CMS focused on authoring, page rendering, and static publishing.
- Fast render and publish paths built around immutable snapshots.
- Operational support for copying, restoring, and bulk multi-site changes.
- Pubsub-first integration surface for downstream systems.
- Workflow acceptance path for AI agents managed through Temporal.
- Persistent disk as the primary publish target, with optional S3 object storage.

## Non-goals for v1

- No server-side HTML management UI in the Rust service.
- No git dependency in the synchronous publish path.
- No attempt to preserve the current row-per-setting relational model.
- No hard dependency on external object storage when local persistent disk is available.

## Top-level shape

- `apps/cms-api`
  - Headless API for sites, branches, snapshots, page documents, assets, publish requests, workflow requests, and pubsub subscriptions.
- `apps/cms-worker`
  - Background worker for snapshot builds, page rendering, publishing, asset processing, pubsub fanout, and workflow execution.
- `crates/cms-core`
  - Core domain types and contracts.
- `crates/cms-render`
  - Snapshot-to-static renderer.
- `crates/cms-pubsub`
  - Pubsub abstraction and implementations.
- `crates/cms-workflows`
  - Temporal-facing workflow definitions, runtime admission rules, and agent handoff contracts.
- `ui/`
  - Bun + TypeScript + SvelteKit management application.

## Core domain model

Use Postgres for coarse relational entities plus JSONB page documents.

Suggested primary tables:

- `accounts`
- `sites`
- `site_domains`
- `branches`
- `snapshots`
- `pages`
- `page_documents`
- `themes`
- `component_packages`
- `assets`
- `publish_jobs`
- `publish_artifacts`
- `workflow_requests`
- `agent_runs`
- `audit_events`
- `component_sources`
- `widget_definitions`
- `widget_definition_versions`
- `widget_instances`

### Authoring model

- A `site` owns one or more `branches`.
- A `site` can either be a standard site or a template site.
- A standard site can optionally inherit from a template site as its source baseline.
- A `branch` points at a mutable head snapshot.
- A `snapshot` is immutable and represents the full publishable state for one site branch.
- Widget instances are the primary editable units on a page.
- Widget definitions are versioned artifacts resolved from either built-in primitives or registered component repos.
- Each `page_document` stores page-level metadata and optional denormalized structure, while `widget_instances` hold the editable widget tree for a snapshot.
- `pages` use a materialized path for hierarchy and ordering, so move/copy operations are path updates plus document rewrites.

### Widget model

- `component_source`
  - describes a registered component repo or other artifact source
- `widget_definition`
  - stable identity for a widget family such as `hero-banner` or `rich-text`
- `widget_definition_version`
  - immutable versioned contract for settings schema, editor schema, assets, runtime, and HTML support policy
- `widget_instance`
  - a page-level editable instance that points at a specific definition version and stores JSON settings

This preserves the flexibility of the current widget/component repo system without forcing settings back into many relational rows.

### Primitive vs registered widgets

- Primitive widgets should still look like widget definitions and widget versions in the data model.
- The difference is source:
  - built-in primitive widgets use `source_kind = builtin`
  - repo-backed widgets use `source_kind = registry_repo`

That avoids two incompatible editing systems.

### Template sites

Template sites should be first-class sites, not a separate content type.

- `site_kind = template` marks a site as a reusable baseline
- `source_template_site_id` links a standard site back to its source template
- copy and restore operations can then operate at snapshot level while still knowing where inherited content came from

This also makes “copy template to many sites” a predictable workflow primitive.

### Page document shape

Each page document should be a typed tree:

- root metadata
- layout id
- ordered block tree
- block props
- references to assets or data sources
- optional workflow-owned fields

Component props should validate against JSON Schema. That keeps the API and UI contract shared across Rust and TypeScript.

## Rendering pipeline

### Key decision

Render from immutable snapshots, not live relational traversal.

### Flow

1. Load snapshot manifest.
2. Resolve page hierarchy and routes.
3. Resolve widget definition versions, component packages, and theme assets.
4. Validate widget settings and page documents against schemas.
5. Render pages to HTML.
6. Emit fingerprinted assets and a publish manifest.
7. Atomically promote the completed build directory.

### Rendering rules

- Rendering should be deterministic from `snapshot_id`.
- Page HTML assembly should be tree-driven, not HTML-fragment mutation.
- Layouts and widgets are versioned packages.
- Server-side rendering should use a narrow template system and avoid arbitrary code execution inside templates.

### Widget runtime strategy

- Support mixed widget complexity levels.
- Some widgets are primitive and server-rendered directly.
- Some widgets are registry-backed and may ship client code in Svelte, React, Vue, or plain JS.
- The server should resolve widget contracts uniformly regardless of source.
- Allow raw HTML support only through explicit widget capabilities and policy flags.

### Template strategy

- Default: `minijinja` for new components and layouts.
- Compatibility bridge only if needed: Rust `liquid` crate for migrated legacy packages.
- Avoid making template execution responsible for major business logic.

## Publishing model

Persistent disk is the primary target.

### Publish stages

1. `queued`
2. `snapshotting`
3. `building`
4. `validating`
5. `promoting`
6. `completed`
7. `failed`

### Publish mechanics

- Build into a versioned temp directory such as `releases/<publish-id>/`.
- Write a manifest containing page hashes, asset hashes, snapshot id, component versions, and timestamps.
- Promote by atomically switching a symlink such as `current -> releases/<publish-id>`.
- Keep the last N releases for rollback.
- Publish completion emits a pubsub event and appends an audit event.

### Git strategy

- Git is asynchronous only.
- After successful promotion, a separate workflow can mirror the snapshot manifest and relevant generated artifacts to git for auditing or review.
- Publish success must not depend on git availability.

### Object storage

- Local disk is primary in v1.
- Optional asset offload later through `OpenDAL` with S3 backing.
- Keep the storage interface simple:
  - local filesystem implementation
  - S3 implementation
  - maybe hybrid mirrored implementation later

## Pubsub model

Pubsub is a first-class boundary, not a thin afterthought.

Event categories:

- `site.snapshot.created`
- `page.document.updated`
- `publish.requested`
- `publish.started`
- `publish.completed`
- `publish.failed`
- `workflow.requested`
- `workflow.completed`
- `workflow.failed`
- `agent.run.accepted`
- `agent.run.rejected`
- `widget.definition.published`
- `widget.instance.updated`
- `site.template.applied`

### v1 transport

- Start with an internal abstraction plus memory and durable implementations.
- The production transport can be one of:
  - Postgres-backed outbox plus dispatcher
  - cloud pubsub if infrastructure already exists

I would start with the outbox pattern in Postgres. It gives reliable delivery semantics and avoids binding the core application to one cloud vendor too early.

## Workflow and AI agent strategy

Temporal should own orchestration and retries. The CMS should own admission, validation, state transitions, and artifact boundaries.

### Workflow request model

Each workflow request should include:

- workflow type
- target site or snapshot
- actor identity
- requested runtime
- input payload
- expected output contract
- safety policy

### Supported agent runtimes

- Rust
- Bun/TypeScript
- Python

### Admission rules

- Only Temporal-managed workflows can request agent execution.
- Every agent request must declare an output schema.
- Agents should operate on snapshots or branch-local drafts, not mutate live publish targets directly.
- High-risk actions such as mass copy, restore, publish, or destructive edits require explicit workflow types and audit logging.

### Recommended boundary

- Temporal activities invoke runtime-specific workers.
- Runtime-specific workers write results back as typed artifacts.
- The CMS validates artifacts before applying them.
- Applied changes produce a new snapshot instead of mutating published state in place.

This keeps AI work reviewable and replayable. It also prevents the system from depending on any single agent runtime for correctness.

## UI strategy

The UI is headless-client-first:

- Bun package manager and runtime for local dev tasks.
- TypeScript for API contracts and schemas.
- SvelteKit for the authoring UI.

The UI should consume typed API contracts generated from Rust schemas where practical, or shared JSON Schema/OpenAPI documents otherwise.

## Testing strategy

Unit testing is required from the start.

### Rust backend

- Unit tests in each crate for domain rules, widget contract validation, renderer behavior, workflow admission, and pubsub contracts.
- Integration tests for API handlers and publish flows.
- Golden tests for rendered HTML and publish manifests.
- Contract tests for JSON Schema compatibility.

### UI

- Component and store tests in TypeScript.
- Contract tests against API schemas.
- End-to-end tests around authoring and publish initiation once the UI exists.

### Workflow and agent execution

- Temporal workflow tests for retries, timeouts, and compensation.
- Admission tests proving runtime restrictions and schema enforcement.
- Fixture-based tests for agent outputs in Rust, TypeScript, and Python shapes.

## Recommended Rust stack

- Runtime and API: `tokio`, `axum`, `tower-http`
- Serialization and schema: `serde`, `schemars`, `utoipa`
- Database: `sqlx` with Postgres
- Rendering: `minijinja`
- Storage abstraction: `opendal`
- Hashing and manifest identity: `blake3`
- Observability: `tracing`, `tracing-subscriber`
- Optional metrics: `metrics`, `metrics-exporter-prometheus`
- File watching in dev: `notify`
- Workflow integration: Temporal Rust SDK if mature enough for your use case, otherwise a thin service boundary around Temporal-operated workers

## Immediate implementation direction

- Build the snapshot model first.
- Build a minimal publish pipeline second.
- Add component package loading and page rendering third.
- Add Temporal workflow admission and agent contracts fourth.
- Build the Svelte UI only after the headless contracts stabilize.
