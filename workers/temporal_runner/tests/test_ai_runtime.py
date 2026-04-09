from __future__ import annotations

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
from rusty_cms_temporal.migrations import discover_pages, execute_site_migration


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
        html = """
        <html>
          <head>
            <title>Example Property</title>
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
        with patch("rusty_cms_temporal.migrations.fetch_html", return_value=html):
            result = await execute_site_migration(
                {
                    "id": "eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee",
                    "kind": "SiteMigration",
                    "site_id": "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb",
                    "branch_name": "migration/draft",
                    "requested_runtime": "Python",
                    "temporal_queue": "cms-migrations",
                    "input_payload": {
                        "homepage_url": "https://example.com/floorplans",
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
        self.assertEqual(result["migration"]["pages"][0]["path"], "/floorplans")
        self.assertIn(
            "hero-banner",
            result["migration"]["pages"][0]["widget_matches"],
        )

    def test_discover_pages_extracts_internal_links_and_widget_signals(self):
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

        pages = discover_pages("https://example.com/", html, detect_widgets=True)

        self.assertEqual(pages[0].title_guess, "Hearth Apartments")
        self.assertIn("floor-plans-plus", pages[0].widget_matches)
        self.assertEqual(pages[1].path, "/floor-plans")


if __name__ == "__main__":
    unittest.main()
