from __future__ import annotations

import json

from rusty_cms_temporal.ai.contracts import AiOperationRequest, RetrievedContext
from rusty_cms_temporal.ai.providers.base import ProviderResponse


class MockAiProvider:
    async def generate(
        self,
        request: AiOperationRequest,
        contexts: list[RetrievedContext],
    ) -> ProviderResponse:
        structured_payload = {
            "instruction": request.instruction,
            "component_id": request.component_id,
            "context_document_ids": [context.document_id for context in contexts],
            "metadata": request.metadata,
        }

        return ProviderResponse(
            provider="mock",
            model=request.model,
            output_text=json.dumps(structured_payload, indent=2, sort_keys=True),
            stop_reason="mock_complete",
            usage={"input_documents": len(contexts)},
        )
