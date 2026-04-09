from __future__ import annotations

from dataclasses import asdict

from rusty_cms_temporal.ai.config import AiRuntimeConfig
from rusty_cms_temporal.ai.contracts import AiOperationRequest, AiProviderKind
from rusty_cms_temporal.ai.evals import build_evaluation_recorder, traceable
from rusty_cms_temporal.ai.providers.anthropic_provider import AnthropicAiProvider
from rusty_cms_temporal.ai.providers.base import AiProvider
from rusty_cms_temporal.ai.providers.mock_provider import MockAiProvider
from rusty_cms_temporal.ai.providers.vertex_provider import VertexAiProvider
from rusty_cms_temporal.ai.retrieval import build_retriever


def _build_provider(
    request: AiOperationRequest,
    config: AiRuntimeConfig,
) -> AiProvider:
    if request.provider == AiProviderKind.ANTHROPIC:
        return AnthropicAiProvider(config)
    if request.provider == AiProviderKind.VERTEX:
        return VertexAiProvider(config)
    return MockAiProvider()


@traceable(name="cms.ai-content-operation", run_type="chain")
async def execute_ai_workflow(
    workflow_request: dict,
    runtime_config: AiRuntimeConfig | None = None,
) -> dict:
    config = runtime_config or AiRuntimeConfig.from_env()
    payload = dict(workflow_request.get("input_payload") or {})
    requested_provider = payload.get("provider")
    provider = (
        AiProviderKind(str(requested_provider).lower())
        if requested_provider
        else config.default_provider
    )
    request = AiOperationRequest.from_workflow_request(
        workflow_request,
        default_provider=provider,
        default_model=config.default_model_for(provider),
    )

    retriever = build_retriever(request)
    contexts = await retriever.retrieve(request)
    provider_client = _build_provider(request, config)
    response = await provider_client.generate(request, contexts)
    evaluation = build_evaluation_recorder(request, config).summarize(
        request, response, contexts
    )

    return {
        "accepted": True,
        "workflow_kind": request.workflow_kind,
        "site_id": request.site_id,
        "branch_name": request.branch_name,
        "requested_runtime": request.requested_runtime,
        "temporal_queue": request.temporal_queue,
        "ai_execution": {
            "provider": request.provider.value,
            "model": response.model,
            "component_id": request.component_id,
            "retrieval_mode": request.retrieval.mode.value,
            "retrieved_contexts": [
                {
                    "document_id": context.document_id,
                    "title": context.title,
                    "source_uri": context.source_uri,
                }
                for context in contexts
            ],
            "evaluation": asdict(evaluation),
            "usage": response.usage,
            "stop_reason": response.stop_reason,
            "output_text": response.output_text,
        },
    }
