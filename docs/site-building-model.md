# Site Building Model

This document describes the target authoring model for the new CMS after the migration and preview foundations. It focuses on how sites should be composed, edited, evaluated, previewed, and published.

## Goals

- Keep the flexibility of the current slot-based template system without recreating traversal-heavy rendering.
- Make most production output static HTML with client islands where needed.
- Support inline editing for content and constrained editing for responsive layout.
- Let structured and freeform tasks coexist in the same authoring model.
- Treat performance, SEO, and accessibility evaluation as first-class editing inputs.

## Page structure

A page should be composed from a template shell plus slot contents.

- `page`
  - route metadata
  - page-level SEO
  - audience hints
  - template id
- `template`
  - named drop targets
  - allowed content rules per target
  - shared shell and chrome
- `page_document`
  - ordered block instances per target

Recommended default targets:

- `header`
- `before_main`
- `main`
- `after_main`
- `footer`
- `sticky`
- `overlay`
- `modals`
- `viewport_top`
- `viewport_bottom`

Each template target should be able to declare:

- allowed primitive kinds
- allowed widget kinds
- ordering rules
- optional nesting or width constraints
- preview hints for editor placement

The intent is to preserve the familiar "drop target" workflow while keeping rendering deterministic and snapshot-friendly.

## Block model

The primary authored unit should be a block instance. A block instance is either a primitive or a registered widget.

### Primitive blocks

Primitives cover common structure and content that should not require a separate component repository.

Content primitives:

- `heading_group`
- `rich_text`
- `image`
- `media_text`
- `cta_band`
- `stat_group`
- `quote`
- `faq_list`
- `divider`
- `spacer`

Layout primitives:

- `container`
- `row`
- `column_group`
- `aside`
- `stack`
- `grid`

Viewport-attached primitives:

- `sticky_cta`
- `modal`
- `drawer`
- `banner`

### Registry widgets

Registered widgets should continue to exist for heavier and more configurable functionality:

- forms
- floor plans
- inventory and pricing integrations
- maps
- media galleries
- chat and tracking integrations
- richer interactive experiences

Widgets should remain versioned and schema-driven. Primitive blocks and widgets should render through the same page-document pipeline so that authoring, preview, and publish all use one model.

## Block instance shape

A block instance should carry:

- `id`
- `kind`
  - `primitive`
  - `widget`
- `type`
  - primitive type or widget slug
- `version`
  - widget version when applicable
- `props`
  - content and settings payload
- `layout`
  - responsive-safe placement and width controls
- `visibility`
  - desktop/mobile visibility and preview flags
- `metadata`
  - provenance, migration notes, shared-fragment references

Example:

```json
{
  "id": "blk_hero_1",
  "kind": "primitive",
  "type": "media_text",
  "props": {
    "eyebrow": "Amenities",
    "heading": "Designed for comfort",
    "body": "Spaces that feel intentional and easy to use.",
    "cta": {
      "label": "Explore amenities",
      "href": "/amenities"
    },
    "image": {
      "asset_id": "asset_123",
      "alt": "Resident lounge"
    }
  },
  "layout": {
    "width": "wide",
    "orientation": "image_right",
    "column_ratio": "40_60",
    "mobile_order": "content_first",
    "spacing_top": "lg",
    "spacing_bottom": "lg"
  },
  "visibility": {
    "desktop": true,
    "mobile": true
  }
}
```

## Editing model

The authoring experience should have three edit surfaces.

### Inline editing

Users should be able to click directly into content on the page preview for:

- headings
- body copy
- CTA labels
- list items
- image alt text
- simple link text

Inline editing should always map back to structured block fields, not freeform DOM mutation.

### Structured inspector editing

Inspector editing should handle the settings that are not comfortable in-place:

- width
- orientation
- alignment
- spacing
- emphasis or background treatment
- mobile order
- widget settings
- integration config
- visibility rules

### Structural actions

Users should be able to:

- move blocks within a target
- move blocks between compatible targets
- duplicate blocks
- swap block type
- convert primitive to widget or widget to primitive when supported
- save a block or section as a reusable preset

## Responsive layout policy

Do not support arbitrary x/y placement for normal page composition.

The responsive-safe layout controls should be constrained to options such as:

- `width`
  - `narrow`
  - `standard`
  - `wide`
  - `full`
- `alignment`
  - `start`
  - `center`
  - `end`
- `orientation`
  - `image_left`
  - `image_right`
  - `stacked`
- `column_ratio`
  - `50_50`
  - `40_60`
  - `60_40`
  - `33_67`
- `mobile_order`
  - `content_first`
  - `media_first`
- `spacing`
  - tokenized top and bottom spacing values

This still lets users shift width and position while protecting mobile behavior.

## Audience model

Most sites serve two audiences at once.

Audience one is the brand or client:

- they need to recognize themselves in the site
- they care about tone, trust, polish, and consistency

Audience two is the renter or end customer:

- they need speed, clarity, mobile usability, and obvious calls to action

This means the system should bias toward:

- mobile-first defaults
- strong semantic structure
- scannable content blocks
- reusable brand styling
- measured support for richer marketing experiences

## Brand system

Brand configuration should be more than raw CSS tokens.

### Brand tokens

- color palette
- typography
- spacing scale
- radii
- borders
- shadows

### Semantic styles

- heading styles
- CTA styles
- card styles
- form styles
- navigation styles
- footer styles
- section intro styles

### Shared fragments

These are reusable branded sections that can be inserted or referenced from pages:

- page heading blocks
- CTA sections
- footers
- form wrappers
- intro rows
- branded media-text rows

Shared fragments should be renderable in isolation so the brand guide builder can preview and adjust them without requiring a full page render.

## Fragment SSR for brand-guide authoring

Production pages should stay mostly static, but fragment SSR is a good fit for authoring.

The brand guide builder should be able to render isolated fragments such as:

- a heading group
- a CTA band
- a media-text row
- a card
- a form shell
- a footer slice

Fragment SSR should accept:

- current brand tokens
- semantic style preset
- block payload
- optional prompt-generated variation request

This gives the system a safe place for AI-assisted design iteration without making the production site broadly server-rendered.

## Platform capabilities

Platform-level scripts and integrations should be modeled separately from page blocks.

Examples:

- analytics
- phone number swapping
- forms runtime
- floor-plan or amenities data bridges
- tag manager
- consent

Each capability should declare:

- environments where it is enabled
  - `preview`
  - `published`
  - `build_only`
  - `disabled`
- injection point
  - `head`
  - `body_start`
  - `body_end`
  - slot-bound
- configuration schema
- dependencies and ordering

This keeps build and preview environments controllable while still supporting customer-required scripts in published environments.

## Task model

The CMS should support both structured and freeform editing tasks.

### Structured tasks

Structured tasks map directly to typed commands:

- update block copy
- move a block
- change a layout preset
- swap an image
- update SEO fields
- change a brand token
- publish selected changes

### Freeform tasks

Freeform tasks are arbitrary instructions with scope and constraints:

- "make this hero feel more upscale"
- "tighten this page for mobile"
- "reduce clutter above the fold"
- "improve SEO without changing the CTA"

Freeform tasks should resolve into proposed typed changes plus evaluation deltas, not direct raw HTML edits.

Task payloads should be able to include:

- scope
  - site, page, block, fragment, or change set
- instruction
- constraints
- approval policy
- evaluation targets

## Evaluation model

Performance, SEO, and accessibility should be stored as first-class evaluation artifacts.

Core evaluation types:

- page speed benchmarking
- SEO evaluation
- WCAG scan results
- content QA and linting

Each evaluation run should be tied to:

- site
- page or fragment
- branch, change set, or snapshot
- tool version
- timestamp

Structured findings should cover:

- pagespeed metrics such as LCP, CLS, INP, asset weight, image weight
- SEO issues such as title length, missing H1, schema validity, internal links, canonical problems
- WCAG issues such as contrast, heading order, form labeling, keyboard access, alt text

These evaluations should feed both manual review and AI-assisted task flows.

## Render policy

The render boundary should be explicit.

### Pre-rendered static HTML

Default to static output for:

- page shell
- template chrome
- headings
- CTA blocks
- footers
- rich text
- most primitives
- page-level SEO and schema output

### Static HTML plus client hydration

Use islands for:

- forms
- floor-plan search and filtering
- tabs and accordions
- chat widgets
- analytics hooks
- integrations that need client state

### Authoring-only SSR

Use SSR in authoring flows for:

- fragment previews in the brand guide
- draft page previews
- migration reconstruction previews
- AI-assisted variation generation

The key distinction is that production delivery stays mostly static while authoring gets fast, targeted server-rendered previews.

## Draft and selective publish model

Unpublished work should not be stored as a single mutable branch head.

Instead:

- a published branch points at a base snapshot
- unpublished changes accumulate as draft change sets
- previews render selected change sets over a base snapshot
- publishing creates a candidate snapshot from selected changes

This supports:

- atomic publish
- selective publish
- preview of arbitrary change subsets
- rollback to prior snapshots

## Phased implementation

### Phase 1: page structure and primitive foundation

Goals:

- define stable page-document schemas
- define template targets and allowed block rules
- define the primitive catalog and block instance shape

Status:

- `implemented` migration draft previews and provisional page-shell changes
- `scaffolded` provisional document candidates in migration artifacts
- `planned` stable page-document schema
- `planned` template-target schema and validation
- `planned` primitive catalog in code

### Phase 2: authoring mutations and inline editing

Goals:

- replace widget-command stubs with real draft mutations
- support inline field editing and inspector editing
- support structural move/duplicate/swap actions

Status:

- `implemented` draft change sets and draft changes
- `scaffolded` widget-command route contract
- `planned` block-level mutation commands
- `planned` inline editing surface
- `planned` responsive layout controls by primitive type

### Phase 3: brand system and fragment SSR

Goals:

- define brand tokens and semantic style presets
- support shared fragments
- render isolated fragments with SSR for authoring

Status:

- `planned` brand token schema
- `planned` semantic style presets
- `planned` shared fragment library
- `planned` fragment SSR API and preview surface

### Phase 4: tasks and evaluations

Goals:

- support typed and freeform authoring tasks
- persist evaluation runs, findings, and scores
- use evaluations as constraints and feedback for editing

Status:

- `implemented` AI workflow provider abstraction and LangSmith gating seams
- `planned` typed task schemas for authoring operations
- `planned` freeform task-to-command proposal flow
- `planned` evaluation ingestion and persistence
- `planned` task-linked evaluation reporting

### Phase 5: render cache and selective publish

Goals:

- add Redis-backed preview cache and invalidation
- render selected draft changes against base snapshots
- publish only chosen changes as an atomic new snapshot

Status:

- `implemented` draft preview route for imported migration changes
- `planned` Redis-backed preview cache
- `planned` dependency-aware invalidation
- `planned` snapshot materialization from selected change sets
- `planned` selective publish UI and workflow
