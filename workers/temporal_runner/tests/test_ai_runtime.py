from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "src"))

from rusty_cms_temporal.ai.config import AiRuntimeConfig
from rusty_cms_temporal.ai.contracts import (
    AiOperationRequest,
    AiProviderKind,
    RetrievalMode,
)
from rusty_cms_temporal.ai.orchestrator import execute_ai_workflow
from rusty_cms_temporal.ai.retrieval import build_retriever


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
            langsmith_tracing=False,
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


if __name__ == "__main__":
    unittest.main()
