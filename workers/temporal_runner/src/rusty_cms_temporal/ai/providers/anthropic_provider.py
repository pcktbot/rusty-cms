from __future__ import annotations

import json

from rusty_cms_temporal.ai.config import AiRuntimeConfig
from rusty_cms_temporal.ai.contracts import AiOperationRequest, RetrievedContext
from rusty_cms_temporal.ai.providers.base import ProviderResponse


def _render_context(contexts: list[RetrievedContext]) -> str:
    sections = []
    for context in contexts:
        header = context.title or context.document_id
        sections.append(f"[{header}]\n{context.content}")
    return "\n\n".join(sections)


def _build_user_prompt(
    request: AiOperationRequest,
    contexts: list[RetrievedContext],
) -> str:
    sections = [f"Instruction:\n{request.instruction}"]
    if request.component_id:
        sections.append(f"Target widget/component:\n{request.component_id}")
    if contexts:
        sections.append(f"Retrieved context:\n{_render_context(contexts)}")
    if request.expected_output_schema:
        sections.append(
            "Return JSON compatible with this schema fragment:\n"
            f"{json.dumps(request.expected_output_schema, indent=2, sort_keys=True)}"
        )
    return "\n\n".join(sections)


def _maybe_wrap_anthropic_client(client, enable_langsmith_tracing: bool):
    if not enable_langsmith_tracing:
        return client
    try:
        from langsmith.wrappers import wrap_anthropic
    except ImportError:
        return client
    return wrap_anthropic(client)


class AnthropicAiProvider:
    def __init__(self, config: AiRuntimeConfig) -> None:
        self.config = config

    async def generate(
        self,
        request: AiOperationRequest,
        contexts: list[RetrievedContext],
    ) -> ProviderResponse:
        if not self.config.anthropic_api_key:
            raise RuntimeError(
                "ANTHROPIC_API_KEY is required when provider=anthropic."
            )

        try:
            from anthropic import AsyncAnthropic
        except ImportError as exc:
            raise RuntimeError(
                "Anthropic SDK is not installed. Install the worker with "
                "`uv pip install -e .[anthropic]`."
            ) from exc

        client = AsyncAnthropic(api_key=self.config.anthropic_api_key)
        client = _maybe_wrap_anthropic_client(
            client,
            self.config.langsmith_enabled and self.config.langsmith_tracing,
        )
        message = await client.messages.create(
            model=request.model,
            max_tokens=request.max_tokens,
            temperature=request.temperature,
            system=request.system_prompt
            or "You are a CMS workflow assistant generating safe, structured website content changes.",
            messages=[
                {
                    "role": "user",
                    "content": _build_user_prompt(request, contexts),
                }
            ],
        )

        output_chunks = []
        for block in getattr(message, "content", []):
            text = getattr(block, "text", None)
            if text:
                output_chunks.append(text)

        usage = getattr(message, "usage", None)
        return ProviderResponse(
            provider="anthropic",
            model=request.model,
            output_text="\n".join(output_chunks).strip(),
            stop_reason=getattr(message, "stop_reason", None),
            usage={
                "input_tokens": getattr(usage, "input_tokens", None),
                "output_tokens": getattr(usage, "output_tokens", None),
            },
            raw_response=message.to_dict() if hasattr(message, "to_dict") else None,
        )
