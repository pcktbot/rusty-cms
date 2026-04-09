from __future__ import annotations

from typing import Protocol

from rusty_cms_temporal.ai.contracts import AiOperationRequest, RetrievalMode, RetrievedContext


class Retriever(Protocol):
    async def retrieve(self, request: AiOperationRequest) -> list[RetrievedContext]: ...


class NullRetriever:
    async def retrieve(self, request: AiOperationRequest) -> list[RetrievedContext]:
        return []


class InlineContextRetriever:
    async def retrieve(self, request: AiOperationRequest) -> list[RetrievedContext]:
        max_documents = max(request.retrieval.max_documents, 0)
        return request.retrieval.documents[:max_documents]


class VertexRagRetriever:
    async def retrieve(self, request: AiOperationRequest) -> list[RetrievedContext]:
        if not request.retrieval.corpus_id:
            raise ValueError(
                "retrieval.corpus_id is required when retrieval.mode is vertex_rag"
            )
        raise NotImplementedError(
            "Vertex RAG retrieval is not wired yet; pass inline context documents for now."
        )


def build_retriever(request: AiOperationRequest) -> Retriever:
    if request.retrieval.mode == RetrievalMode.INLINE:
        return InlineContextRetriever()
    if request.retrieval.mode == RetrievalMode.VERTEX_RAG:
        return VertexRagRetriever()
    return NullRetriever()
