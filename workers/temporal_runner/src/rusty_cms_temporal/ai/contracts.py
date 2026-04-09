from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from typing import Any


class AiProviderKind(str, Enum):
    MOCK = "mock"
    ANTHROPIC = "anthropic"
    VERTEX = "vertex"


class RetrievalMode(str, Enum):
    NONE = "none"
    INLINE = "inline"
    VERTEX_RAG = "vertex_rag"


class EvaluationProvider(str, Enum):
    NONE = "none"
    LANGSMITH = "langsmith"


@dataclass(slots=True)
class RetrievedContext:
    document_id: str
    title: str | None = None
    content: str = ""
    source_uri: str | None = None
    metadata: dict[str, Any] = field(default_factory=dict)

    @classmethod
    def from_payload(
        cls, payload: dict[str, Any], fallback_document_id: str
    ) -> "RetrievedContext":
        return cls(
            document_id=str(
                payload.get("id")
                or payload.get("document_id")
                or payload.get("slug")
                or fallback_document_id
            ),
            title=payload.get("title"),
            content=str(payload.get("content") or payload.get("text") or ""),
            source_uri=payload.get("source_uri") or payload.get("url"),
            metadata=dict(payload.get("metadata") or {}),
        )


@dataclass(slots=True)
class RetrievalSpec:
    mode: RetrievalMode = RetrievalMode.NONE
    query: str | None = None
    max_documents: int = 8
    documents: list[RetrievedContext] = field(default_factory=list)
    corpus_id: str | None = None

    @classmethod
    def from_payload(cls, payload: dict[str, Any] | None) -> "RetrievalSpec":
        data = dict(payload or {})
        mode = RetrievalMode(str(data.get("mode") or RetrievalMode.NONE.value).lower())
        documents = [
            RetrievedContext.from_payload(document, f"context-{index + 1}")
            for index, document in enumerate(data.get("documents") or [])
        ]
        max_documents = int(data.get("max_documents") or len(documents) or 8)
        return cls(
            mode=mode,
            query=data.get("query"),
            max_documents=max_documents,
            documents=documents,
            corpus_id=data.get("corpus_id"),
        )


@dataclass(slots=True)
class EvaluationSpec:
    provider: EvaluationProvider = EvaluationProvider.NONE
    project: str | None = None
    evaluators: list[str] = field(default_factory=list)
    tags: list[str] = field(default_factory=list)
    metadata: dict[str, Any] = field(default_factory=dict)

    @classmethod
    def from_payload(cls, payload: dict[str, Any] | None) -> "EvaluationSpec":
        data = dict(payload or {})
        provider = EvaluationProvider(
            str(data.get("provider") or EvaluationProvider.NONE.value).lower()
        )
        evaluators = [str(evaluator) for evaluator in data.get("evaluators") or []]
        tags = [str(tag) for tag in data.get("tags") or []]
        return cls(
            provider=provider,
            project=data.get("project"),
            evaluators=evaluators,
            tags=tags,
            metadata=dict(data.get("metadata") or {}),
        )


@dataclass(slots=True)
class AiOperationRequest:
    workflow_id: str
    workflow_kind: str
    site_id: str
    branch_name: str
    requested_runtime: str
    temporal_queue: str
    provider: AiProviderKind
    model: str
    instruction: str
    component_id: str | None = None
    system_prompt: str | None = None
    temperature: float = 0.2
    max_tokens: int = 1200
    expected_output_schema: dict[str, Any] | None = None
    retrieval: RetrievalSpec = field(default_factory=RetrievalSpec)
    evaluation: EvaluationSpec = field(default_factory=EvaluationSpec)
    metadata: dict[str, Any] = field(default_factory=dict)
    raw_payload: dict[str, Any] = field(default_factory=dict)

    @classmethod
    def from_workflow_request(
        cls,
        request: dict[str, Any],
        default_provider: AiProviderKind,
        default_model: str,
    ) -> "AiOperationRequest":
        payload = dict(request.get("input_payload") or {})
        provider = AiProviderKind(
            str(payload.get("provider") or default_provider.value).lower()
        )
        model = str(payload.get("model") or default_model)

        retrieval_payload = payload.get("retrieval")
        if not retrieval_payload and payload.get("context_documents"):
            retrieval_payload = {
                "mode": RetrievalMode.INLINE.value,
                "documents": payload.get("context_documents"),
            }

        instruction = str(
            payload.get("instruction")
            or payload.get("prompt")
            or "Produce the requested CMS content update."
        )

        expected_schema = payload.get("expected_output_schema")
        if expected_schema is not None and not isinstance(expected_schema, dict):
            expected_schema = {"value": expected_schema}

        return cls(
            workflow_id=str(request["id"]),
            workflow_kind=str(request["kind"]),
            site_id=str(request["site_id"]),
            branch_name=str(request["branch_name"]),
            requested_runtime=str(request["requested_runtime"]),
            temporal_queue=str(request["temporal_queue"]),
            provider=provider,
            model=model,
            instruction=instruction,
            component_id=payload.get("component_id"),
            system_prompt=payload.get("system_prompt"),
            temperature=float(payload.get("temperature") or 0.2),
            max_tokens=int(payload.get("max_tokens") or 1200),
            expected_output_schema=expected_schema,
            retrieval=RetrievalSpec.from_payload(retrieval_payload),
            evaluation=EvaluationSpec.from_payload(payload.get("evaluation")),
            metadata=dict(payload.get("metadata") or {}),
            raw_payload=payload,
        )
