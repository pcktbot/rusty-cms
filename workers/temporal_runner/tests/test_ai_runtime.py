from __future__ import annotations

import ssl
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "src"))

from rusty_cms_temporal.ai.config import AiRuntimeConfig
from rusty_cms_temporal.ai.contracts import (
    AiOperationRequest,
    AiProviderKind,
    RetrievalMode,
)
from rusty_cms_temporal.ai.orchestrator import execute_ai_workflow
from rusty_cms_temporal.ai.retrieval import build_retriever
from rusty_cms_temporal.migrations import (
    FetchResult,
    build_ssl_context,
    crawl_page,
    discover_paths,
    extract_page_document_candidate,
    execute_site_migration,
)


def sample_workflow_request() -> dict:
    return {
        "id": "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
        "kind": "AiContentOperation",
        "site_id": "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb",
        "branch_name": "draft",
        "requested_runtime": "Python",
        "temporal_queue": "cms-agent-ops",
        "input_payload": {
            "instruction": "Refresh the homepage hero and CTA copy.",
            "provider": "mock",
            "component_id": "hero.v1",
            "context_documents": [
                {
                    "id": "homepage",
                    "title": "Homepage",
                    "content": "Current hero headline focuses on move-ins and tours.",
                },
                {
                    "id": "brand",
                    "title": "Brand Voice",
                    "content": "Confident, direct, and hospitality-led.",
                },
            ],
            "evaluation": {
                "provider": "langsmith",
                "project": "rusty-cms",
                "evaluators": ["conciseness", "brand_voice"],
                "tags": ["ai-content", "draft"],
            },
        },
    }


class AiRuntimeTests(unittest.IsolatedAsyncioTestCase):
    async def test_orchestrator_uses_mock_provider_and_emits_langsmith_metadata(self):
        config = AiRuntimeConfig(
            default_provider=AiProviderKind.MOCK,
            anthropic_api_key=None,
            anthropic_model="claude-sonnet-4-5-20250929",
            vertex_project=None,
            vertex_location=None,
            vertex_model="gemini-2.5-flash",
            langsmith_enabled=True,
            langsmith_tracing=False,
            langsmith_evals_enabled=True,
            langsmith_project="rusty-cms",
        )

        result = await execute_ai_workflow(sample_workflow_request(), config)

        self.assertTrue(result["accepted"])
        self.assertEqual(result["ai_execution"]["provider"], "mock")
        self.assertEqual(result["ai_execution"]["retrieval_mode"], "inline")
        self.assertEqual(
            len(result["ai_execution"]["retrieved_contexts"]),
            2,
        )
        self.assertEqual(
            result["ai_execution"]["evaluation"]["provider"],
            "langsmith",
        )

    async def test_inline_retriever_limits_documents(self):
        request = AiOperationRequest.from_workflow_request(
            sample_workflow_request(),
            default_provider=AiProviderKind.MOCK,
            default_model="mock-cms-writer-v1",
        )
        request.retrieval.max_documents = 1

        contexts = await build_retriever(request).retrieve(request)

        self.assertEqual(len(contexts), 1)
        self.assertEqual(contexts[0].document_id, "homepage")

    async def test_request_parses_context_documents_into_inline_retrieval(self):
        request = AiOperationRequest.from_workflow_request(
            sample_workflow_request(),
            default_provider=AiProviderKind.MOCK,
            default_model="mock-cms-writer-v1",
        )

        self.assertEqual(request.provider, AiProviderKind.MOCK)
        self.assertEqual(request.retrieval.mode, RetrievalMode.INLINE)
        self.assertEqual(len(request.retrieval.documents), 2)

    async def test_site_migration_stub_returns_review_ready_shape(self):
        homepage_html = """
        <html>
          <head>
            <title>Example Property</title>
            <script type="application/ld+json">
              {"@context":"https://schema.org","@type":"WebPage"}
            </script>
          </head>
          <body>
            <div class="hero-banner">Hero</div>
            <nav>
              <a href="/floorplans">Floor Plans</a>
              <a href="/amenities">Amenities</a>
            </nav>
            <script src="/assets/hero-banner.js"></script>
          </body>
        </html>
        """
        inner_html = """
        <html>
          <head><title>Floor Plans</title></head>
          <body>
            <main>
              <section class="row hero-banner feature">
                <img src="/images/floorplans-hero.jpg" alt="Bright living room" />
                <h1>Choose Your Layout</h1>
                <p>Browse studio, one-, and two-bedroom homes.</p>
              </section>
            </main>
          </body>
        </html>
        """

        def fake_fetch(url: str, timeout_seconds: float = 10.0) -> FetchResult:
            if url.endswith("/floorplans"):
                return FetchResult(
                    source_url=url,
                    final_url=url,
                    http_status=200,
                    content_type="text/html",
                    html=inner_html,
                )
            return FetchResult(
                source_url=url,
                final_url=url,
                http_status=200,
                content_type="text/html",
                html=homepage_html,
            )

        with patch("rusty_cms_temporal.migrations.fetch_html", side_effect=fake_fetch):
            result = await execute_site_migration(
                {
                    "id": "eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee",
                    "kind": "SiteMigration",
                    "site_id": "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb",
                    "branch_name": "migration/draft",
                    "requested_runtime": "Python",
                    "temporal_queue": "cms-migrations",
                    "input_payload": {
                        "homepage_url": "https://example.com/",
                        "client_id": "cccccccc-cccc-cccc-cccc-cccccccccccc",
                        "location_id": "dddddddd-dddd-dddd-dddd-dddddddddddd",
                        "options": {
                            "detect_registered_widgets": True,
                            "use_legacy_api_enrichment": True,
                        },
                    },
                }
            )

        self.assertTrue(result["accepted"])
        self.assertEqual(result["migration"]["status"], "review_ready")
        self.assertEqual(result["migration"]["pages"][0]["path"], "/")
        self.assertIn(
            "hero-banner",
            result["migration"]["pages"][0]["widget_matches"],
        )
        self.assertEqual(result["migration"]["pages"][1]["path"], "/floorplans")
        self.assertIn("WebPage", result["migration"]["pages"][0]["schema_types"])
        self.assertTrue(result["migration"]["pages"][1]["layout"]["regions"])
        self.assertTrue(result["migration"]["pages"][1]["images"])
        self.assertTrue(result["migration"]["pages"][1]["media_text_regions"])

    async def test_site_migration_can_run_page_document_extraction_action(self):
        html = """
        <html>
          <head><title>Floor Plans</title></head>
          <body>
            <main>
              <div id="drop-target-aside-before-main">
                <section class="feature-row image-right">
                  <img src="/images/lounge.jpg" alt="Resident lounge seating" />
                  <h2>Made for gathering</h2>
                  <p>Comfortable shared spaces and natural light throughout.</p>
                </section>
              </div>
            </main>
          </body>
        </html>
        """

        with patch(
            "rusty_cms_temporal.migrations.fetch_html",
            return_value=FetchResult(
                source_url="https://example.com/floorplans",
                final_url="https://example.com/floorplans",
                http_status=200,
                content_type="text/html",
                html=html,
            ),
        ):
            result = await execute_site_migration(
                {
                    "id": "ffffffff-eeee-dddd-cccc-bbbbbbbbbbbb",
                    "kind": "SiteMigration",
                    "site_id": "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb",
                    "branch_name": "migration/draft",
                    "requested_runtime": "Python",
                    "temporal_queue": "cms-migrations",
                    "input_payload": {
                        "action": "extract_page_documents",
                        "migration_job_id": "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
                        "homepage_url": "https://example.com/",
                        "client_id": "cccccccc-cccc-cccc-cccc-cccccccccccc",
                        "location_id": "dddddddd-dddd-dddd-dddd-dddddddddddd",
                        "pages": [
                            {
                                "path": "/floorplans",
                                "source_url": "https://example.com/floorplans",
                                "title_guess": "Floor Plans",
                                "widget_matches": [],
                            }
                        ],
                    },
                }
            )

        self.assertEqual(result["migration"]["action"], "extract_page_documents")
        page = result["migration"]["pages"][0]
        blocks = page["document_candidate"]["regions"]["main"]
        self.assertTrue(any(block["primitive_type"] == "media_text" for block in blocks))

    def test_discovery_extracts_internal_links_and_widget_signals(self):
        html = """
        <html>
          <head>
            <title>Hearth Apartments</title>
          </head>
          <body>
            <section data-widget="floor-plans-plus"></section>
            <a href="/floor-plans">Floor Plans</a>
            <a href="https://example.com/amenities">Amenities</a>
          </body>
        </html>
        """

        paths = discover_paths("https://example.com/", html)

        self.assertEqual(paths[0], "/")
        self.assertEqual(paths[1], "/floor-plans")

    def test_crawl_page_extracts_schema_and_layout_summary(self):
        html = """
        <html>
          <head>
            <title>Hearth Apartments</title>
            <meta name="description" content="Live well in Shelburne." />
            <script type="application/ld+json">
              {"@context":"https://schema.org","@type":["WebPage","FAQPage"]}
            </script>
          </head>
          <body>
            <main>
              <section class="hero-banner row">
                <h1>Find Home</h1>
                <p>Warm residences near the lake.</p>
              </section>
              <aside class="sidebar">Leasing info</aside>
            </main>
          </body>
        </html>
        """
        with patch(
            "rusty_cms_temporal.migrations.fetch_html",
            return_value=FetchResult(
                source_url="https://example.com/",
                final_url="https://example.com/",
                http_status=200,
                content_type="text/html",
                html=html,
            ),
        ):
            page = crawl_page("/", "https://example.com/", detect_widgets=True)

        self.assertEqual(page.seo["meta_description"], "Live well in Shelburne.")
        self.assertIn("FAQPage", page.schema_types)
        self.assertTrue(page.layout["regions"])
        self.assertTrue(page.document_candidate["regions"]["main"])

    def test_crawl_page_extracts_images_and_media_text_regions(self):
        html = """
        <html>
          <head><title>Features</title></head>
          <body>
            <section class="feature-row image-right">
              <img src="/images/lounge.jpg" alt="Resident lounge seating" />
              <h2>Made for gathering</h2>
              <p>Comfortable shared spaces and natural light throughout.</p>
            </section>
          </body>
        </html>
        """
        with patch(
            "rusty_cms_temporal.migrations.fetch_html",
            return_value=FetchResult(
                source_url="https://example.com/features",
                final_url="https://example.com/features",
                http_status=200,
                content_type="text/html",
                html=html,
            ),
        ):
            page = crawl_page("/features", "https://example.com/features", detect_widgets=True)

        self.assertEqual(page.images[0]["alt"], "Resident lounge seating")
        self.assertTrue(page.media_text_regions)
        self.assertIn(page.media_text_regions[0]["orientation"], {"image_left", "image_right"})
        self.assertTrue(
            any(block["kind"] == "media_text" for block in page.document_candidate["regions"]["main"])
        )

    def test_extract_page_document_candidate_flattens_before_main_wrappers(self):
        html = """
        <html>
          <head><title>Features</title></head>
          <body>
            <main>
              <aside id="drop-target-aside-before-main">
                <div class="feature-row">
                  <img src="/images/pool.jpg" alt="Pool deck" />
                  <h2>Spaces to unwind</h2>
                  <p>Sunny seating and resort-inspired details.</p>
                </div>
              </aside>
            </main>
          </body>
        </html>
        """
        with patch(
            "rusty_cms_temporal.migrations.fetch_html",
            return_value=FetchResult(
                source_url="https://example.com/features",
                final_url="https://example.com/features",
                http_status=200,
                content_type="text/html",
                html=html,
            ),
        ):
            page = extract_page_document_candidate(
                "/features",
                "https://example.com/features",
                "Features",
                [],
            )

        self.assertTrue(page.layout["regions"])
        self.assertEqual(page.layout["regions"][0]["kind"], "feature")
        self.assertTrue(page.document_candidate["regions"]["main"])

    def test_build_ssl_context_can_disable_verification(self):
        with patch.dict(
            "os.environ",
            {"CMS_MIGRATION_ALLOW_INSECURE_TLS": "true"},
            clear=False,
        ):
            context = build_ssl_context()

        self.assertEqual(context.verify_mode, ssl.CERT_NONE)
        self.assertFalse(context.check_hostname)


if __name__ == "__main__":
    unittest.main()
