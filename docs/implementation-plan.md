# Implementation Plan

Status legend:

- `implemented`: usable in the current codebase
- `scaffolded`: contract or shell exists, but the core feature is not complete
- `planned`: not started beyond design notes

## Platform foundation

- `implemented` Rust workspace and crate boundaries
- `implemented` Postgres migrations for core CMS, widget registry, and template-site linkage
- `implemented` `.env` loading and startup validation for Rust API and Temporal Python runner
- `implemented` Temporal workflow admission matrix and queue definitions
- `implemented` server preview viewer at `/viewer`

## API

- `implemented` runtime info route
- `implemented` Postgres-backed read routes for sites, branches, branch heads, widget definitions, and widget definition versions when `DATABASE_URL` is configured
- `implemented` fallback seeded catalog for local development when no database is configured
- `implemented` workflow request submit/trigger routes with workflow-request persistence and outbox emission
- `implemented` local widget source import route for registry inspection
- `implemented` migration create/review/approve routes
- `implemented` Postgres-backed migration job and review artifact persistence when `DATABASE_URL` is configured
- `implemented` manual migration discovery sync route from Temporal result into Postgres
- `scaffolded` widget command route contract
- `planned` branch CRUD, snapshot CRUD, page CRUD, asset CRUD, theme CRUD

## Widget registry

- `implemented` widget definition and version domain model
- `implemented` widget registry migrations and read queries
- `implemented` local repo importer that reads metadata from committed widget repos
- `scaffolded` widget signature-based migration detection model in schemas and workflow plans only
- `planned` registry persistence/import promotion from local repo into database rows and packaged artifacts
- `planned` widget version migration hooks

## Rendering and publishing

- `implemented` preview render stub with snapshot identity
- `implemented` publish state machine domain model
- `scaffolded` publish target model and artifact tables
- `implemented` imported-draft preview route for migration-generated change sets
- `planned` real snapshot renderer
- `planned` full-site static build pipeline
- `planned` atomic release promotion on disk
- `planned` publish rollback controls

## Workflows and workers

- `implemented` Temporal Python worker with queues for publish, restore, bulk, migrations, and agent ops
- `implemented` AI workflow runtime wrappers with provider abstraction, retrieval hooks, LangSmith flags, and Anthropic/Vertex adapter seams
- `implemented` site migration workflow kind, queue, schemas, and worker activity
- `implemented` crawl/discovery pass for homepage and discovered same-host pages
- `implemented` first-pass SEO extraction, schema discovery, layout summaries, and document candidates in the migration worker
- `implemented` site migration review artifact generation with persisted page artifacts
- `planned` durable workflow status tracking beyond workflow-request rows
- `planned` Rust or Bun workflow workers for non-Python tasks

## AI and evaluation

- `implemented` provider-neutral AI activity orchestration
- `implemented` mock provider path for deterministic local testing
- `implemented` Anthropic adapter seam and LangSmith gating flags
- `implemented` Vertex adapter seam
- `scaffolded` LangSmith evaluation metadata emission
- `planned` real retrieval layer
- `planned` evaluator execution and scoring persistence
- `planned` fine-tuned Vertex production path

## Migration system

- `implemented` migration request/output schemas
- `implemented` migration API request contract:
  - homepage URL
  - client association
  - location association
  - crawl and enrichment options
- `implemented` migration workflow admission and Temporal trigger path
- `implemented` migration review records and page-level review endpoints
- `implemented` homepage crawler and same-host route discovery
- `implemented` persisted migration page artifacts with SEO, schema, layout, text, and provisional document candidates
- `implemented` migration-to-draft import route that creates draft change sets from persisted discovery artifacts
- `scaffolded` DOM/template classifier with first-pass layout region detection
- `planned` registered-widget signature detection
- `planned` targeted legacy API enrichment
- `scaffolded` draft snapshot importer via provisional page-shell changes and document candidates
- `planned` screenshot and content diff validation

## Hybrid preview and selective publish

Phase 1 is now defined as the minimum path from migration discovery to draft preview.

Phase 1 goals:

1. Persist unpublished work as draft change sets instead of mutating a branch head directly.
2. Import migration results into provisional page-shell draft changes.
3. Render per-page draft previews from those imported changes.

Phase 1 shape:

- `implemented` `draft_change_sets` hold branch context, base snapshot, source kind, and status
- `implemented` `draft_changes` hold page-level change payloads keyed by change set
- `implemented` migration import currently produces `upsert_page_shell` changes with provisional metadata-driven payloads
- `implemented` preview routes can render imported page-shell changes without publishing
- `scaffolded` rich DOM-to-document extraction via SEO/layout/text/document candidates
- `planned` preview invalidation and Redis cache keys
- `planned` publish selection from arbitrary subsets of draft changes

Planned cache strategy:

- Postgres remains the source of truth for change sets, change payloads, and publish selections
- Redis will cache fragment/page preview output keyed by `site`, `branch`, `base_snapshot`, and selected change-set hash
- Redis will also hold dependency fanout and short-lived preview session state, not authoritative content

Planned selective publish strategy:

- branch head points to the last published snapshot
- unpublished work accumulates in draft change sets on top of that base snapshot
- publish creates a candidate snapshot from an explicit set of selected changes
- release promotion stays atomic because only the candidate snapshot is built and promoted

## UI

- `implemented` temporary server-rendered preview shell
- `scaffolded` API surfaces for the future migration and authoring UI
- `planned` Bun + SvelteKit management app
- `planned` migration review interface
- `planned` page tree and widget editing shell

## Immediate next steps

1. Replace manual migration discovery sync with automatic workflow result ingestion.
2. Promote document candidates into stable page-document schemas instead of loosely typed JSON.
3. Add widget signature metadata to the registry model and classify known legacy widgets during migration.
4. Add preview cache keys and Redis-backed invalidation for draft page renders.
5. Replace widget-command stubs with real draft change mutations on top of selected base snapshots.
