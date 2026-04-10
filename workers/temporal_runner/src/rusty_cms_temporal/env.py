from __future__ import annotations

import os
from pathlib import Path


def env_flag(name: str, default: bool = False) -> bool:
    value = os.environ.get(name)
    if value is None:
        return default
    return value.strip().lower() in {"1", "true", "yes", "on"}


def optional_env(name: str) -> str | None:
    value = os.environ.get(name, "").strip()
    return value or None


def require_nonempty(name: str) -> str:
    value = os.environ.get(name, "").strip()
    if not value:
        raise RuntimeError(f"{name} must be set")
    return value


def load_environment() -> None:
    try:
        from dotenv import load_dotenv
    except ImportError:
        return

    load_dotenv()


def validate_runtime_environment() -> None:
    require_nonempty("TEMPORAL_GRPC_ENDPOINT")
    require_nonempty("TEMPORAL_NAMESPACE")

    provider = os.environ.get("CMS_AI_PROVIDER", "").strip().lower()
    if not provider:
        if os.environ.get("ANTHROPIC_API_KEY"):
            provider = "anthropic"
        else:
            provider = "mock"

    if provider == "anthropic":
        require_nonempty("ANTHROPIC_API_KEY")
    elif provider == "vertex":
        require_nonempty("CMS_AI_VERTEX_PROJECT")
        require_nonempty("CMS_AI_VERTEX_LOCATION")

    langsmith_enabled = env_flag(
        "CMS_AI_ENABLE_LANGSMITH",
        default=env_flag("LANGSMITH_TRACING", default=False),
    )
    langsmith_tracing = env_flag(
        "CMS_AI_ENABLE_LANGSMITH_TRACING",
        default=langsmith_enabled,
    ) or env_flag("LANGSMITH_TRACING", default=False)
    langsmith_evals = env_flag(
        "CMS_AI_ENABLE_LANGSMITH_EVALS",
        default=langsmith_enabled,
    )

    if langsmith_tracing or langsmith_evals:
        require_nonempty("LANGSMITH_API_KEY")

    ca_bundle = optional_env("CMS_MIGRATION_CA_BUNDLE")
    if ca_bundle and not Path(ca_bundle).exists():
        raise RuntimeError(f"CMS_MIGRATION_CA_BUNDLE does not exist: {ca_bundle}")
