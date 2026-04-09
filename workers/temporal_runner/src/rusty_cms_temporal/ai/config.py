from __future__ import annotations

import os
from dataclasses import dataclass

from rusty_cms_temporal.ai.contracts import AiProviderKind


def _env_flag(name: str, default: bool = False) -> bool:
    value = os.environ.get(name)
    if value is None:
        return default
    return value.strip().lower() in {"1", "true", "yes", "on"}


def _provider_from_env() -> AiProviderKind:
    configured = os.environ.get("CMS_AI_PROVIDER")
    if configured:
        return AiProviderKind(configured.strip().lower())
    if os.environ.get("ANTHROPIC_API_KEY"):
        return AiProviderKind.ANTHROPIC
    return AiProviderKind.MOCK


def _langsmith_enabled_from_env() -> bool:
    return _env_flag(
        "CMS_AI_ENABLE_LANGSMITH",
        default=_env_flag("LANGSMITH_TRACING", default=False),
    )


@dataclass(slots=True)
class AiRuntimeConfig:
    default_provider: AiProviderKind
    anthropic_api_key: str | None
    anthropic_model: str
    vertex_project: str | None
    vertex_location: str | None
    vertex_model: str
    langsmith_enabled: bool
    langsmith_tracing: bool
    langsmith_evals_enabled: bool
    langsmith_project: str | None

    @classmethod
    def from_env(cls) -> "AiRuntimeConfig":
        langsmith_enabled = _langsmith_enabled_from_env()
        return cls(
            default_provider=_provider_from_env(),
            anthropic_api_key=os.environ.get("ANTHROPIC_API_KEY"),
            anthropic_model=os.environ.get(
                "CMS_AI_ANTHROPIC_MODEL", "claude-sonnet-4-5-20250929"
            ),
            vertex_project=os.environ.get("CMS_AI_VERTEX_PROJECT"),
            vertex_location=os.environ.get("CMS_AI_VERTEX_LOCATION"),
            vertex_model=os.environ.get("CMS_AI_VERTEX_MODEL", "gemini-2.5-flash"),
            langsmith_enabled=langsmith_enabled,
            langsmith_tracing=_env_flag(
                "CMS_AI_ENABLE_LANGSMITH_TRACING",
                default=langsmith_enabled,
            )
            or _env_flag("LANGSMITH_TRACING", default=False),
            langsmith_evals_enabled=_env_flag(
                "CMS_AI_ENABLE_LANGSMITH_EVALS",
                default=langsmith_enabled,
            ),
            langsmith_project=os.environ.get("LANGSMITH_PROJECT"),
        )

    def default_model_for(self, provider: AiProviderKind) -> str:
        if provider == AiProviderKind.ANTHROPIC:
            return self.anthropic_model
        if provider == AiProviderKind.VERTEX:
            return self.vertex_model
        return "mock-cms-writer-v1"
