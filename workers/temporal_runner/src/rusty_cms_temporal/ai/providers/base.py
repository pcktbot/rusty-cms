from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Protocol

from rusty_cms_temporal.ai.contracts import AiOperationRequest, RetrievedContext


@dataclass(slots=True)
class ProviderResponse:
    provider: str
    model: str
    output_text: str
    stop_reason: str | None = None
    usage: dict[str, Any] = field(default_factory=dict)
    raw_response: dict[str, Any] | None = None


class AiProvider(Protocol):
    async def generate(
        self,
        request: AiOperationRequest,
        contexts: list[RetrievedContext],
    ) -> ProviderResponse: ...
