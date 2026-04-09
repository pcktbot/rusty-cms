from __future__ import annotations

import asyncio
import json

from rusty_cms_temporal.ai.config import AiRuntimeConfig
from rusty_cms_temporal.ai.contracts import AiOperationRequest, RetrievedContext
from rusty_cms_temporal.ai.providers.base import ProviderResponse


def _build_prompt(
    request: AiOperationRequest,
    contexts: list[RetrievedContext],
) -> str:
    sections = [request.instruction]
    if request.component_id:
        sections.append(f"Widget/component: {request.component_id}")
    if contexts:
        formatted_contexts = "\n\n".join(
            f"[{context.title or context.document_id}]\n{context.content}"
            for context in contexts
        )
        sections.append(f"Context:\n{formatted_contexts}")
    if request.expected_output_schema:
        sections.append(
            "Return JSON compatible with this schema fragment:\n"
            f"{json.dumps(request.expected_output_schema, indent=2, sort_keys=True)}"
        )
    return "\n\n".join(sections)


class VertexAiProvider:
    def __init__(self, config: AiRuntimeConfig) -> None:
        self.config = config

    async def generate(
        self,
        request: AiOperationRequest,
        contexts: list[RetrievedContext],
    ) -> ProviderResponse:
        if not self.config.vertex_project or not self.config.vertex_location:
            raise RuntimeError(
                "CMS_AI_VERTEX_PROJECT and CMS_AI_VERTEX_LOCATION are required "
                "when provider=vertex."
            )

        try:
            from google import genai
        except ImportError as exc:
            raise RuntimeError(
                "Google Gen AI SDK is not installed. Install the worker with "
                "`uv pip install -e .[vertex]`."
            ) from exc

        client = genai.Client(
            vertexai=True,
            project=self.config.vertex_project,
            location=self.config.vertex_location,
        )
        response = await asyncio.to_thread(
            client.models.generate_content,
            model=request.model,
            contents=_build_prompt(request, contexts),
        )

        return ProviderResponse(
            provider="vertex",
            model=request.model,
            output_text=getattr(response, "text", "") or "",
            stop_reason=None,
            usage={},
        )
