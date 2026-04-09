from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Callable, Protocol

from rusty_cms_temporal.ai.config import AiRuntimeConfig
from rusty_cms_temporal.ai.contracts import (
    AiOperationRequest,
    EvaluationProvider,
    RetrievedContext,
)
from rusty_cms_temporal.ai.providers.base import ProviderResponse


def traceable(name: str, run_type: str = "chain") -> Callable:
    try:
        from langsmith import traceable as langsmith_traceable
    except ImportError:
        def decorator(function: Callable) -> Callable:
            return function
        return decorator

    return langsmith_traceable(name=name, run_type=run_type)


@dataclass(slots=True)
class EvaluationSummary:
    provider: str
    trace_enabled: bool
    project: str | None = None
    evaluators: list[str] = field(default_factory=list)
    tags: list[str] = field(default_factory=list)
    metadata: dict[str, Any] = field(default_factory=dict)
    notes: list[str] = field(default_factory=list)


class EvaluationRecorder(Protocol):
    def summarize(
        self,
        request: AiOperationRequest,
        response: ProviderResponse,
        contexts: list[RetrievedContext],
    ) -> EvaluationSummary: ...


class NullEvaluationRecorder:
    def summarize(
        self,
        request: AiOperationRequest,
        response: ProviderResponse,
        contexts: list[RetrievedContext],
    ) -> EvaluationSummary:
        return EvaluationSummary(provider="none", trace_enabled=False)


class LangSmithEvaluationRecorder:
    def __init__(self, config: AiRuntimeConfig) -> None:
        self.config = config

    def summarize(
        self,
        request: AiOperationRequest,
        response: ProviderResponse,
        contexts: list[RetrievedContext],
    ) -> EvaluationSummary:
        metadata = {
            "workflow_id": request.workflow_id,
            "site_id": request.site_id,
            "branch_name": request.branch_name,
            "provider": request.provider.value,
            "model": response.model,
            "context_document_ids": [context.document_id for context in contexts],
        }
        metadata.update(request.evaluation.metadata)

        notes = [
            "Use LangSmith tracing wrappers or @traceable to capture production runs.",
            "Configure online evaluators on the traced project once the output contract stabilizes.",
        ]
        if not self.config.langsmith_tracing:
            notes.append(
                "LANGSMITH_TRACING is disabled, so this run exposes evaluation metadata only."
            )

        return EvaluationSummary(
            provider=EvaluationProvider.LANGSMITH.value,
            trace_enabled=self.config.langsmith_tracing,
            project=request.evaluation.project or self.config.langsmith_project,
            evaluators=request.evaluation.evaluators,
            tags=request.evaluation.tags,
            metadata=metadata,
            notes=notes,
        )


def build_evaluation_recorder(
    request: AiOperationRequest, config: AiRuntimeConfig
) -> EvaluationRecorder:
    if request.evaluation.provider == EvaluationProvider.LANGSMITH:
        return LangSmithEvaluationRecorder(config)
    return NullEvaluationRecorder()
