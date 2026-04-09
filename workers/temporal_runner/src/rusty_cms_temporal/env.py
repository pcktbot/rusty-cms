from __future__ import annotations

import os


def _env_flag(name: str, default: bool = False) -> bool:
    value = os.environ.get(name)
    if value is None:
        return default
    return value.strip().lower() in {"1", "true", "yes", "on"}


def _require_nonempty(name: str) -> str:
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
    _require_nonempty("TEMPORAL_GRPC_ENDPOINT")
    _require_nonempty("TEMPORAL_NAMESPACE")

    provider = os.environ.get("CMS_AI_PROVIDER", "").strip().lower()
    if not provider:
        if os.environ.get("ANTHROPIC_API_KEY"):
            provider = "anthropic"
        else:
            provider = "mock"

    if provider == "anthropic":
        _require_nonempty("ANTHROPIC_API_KEY")
    elif provider == "vertex":
        _require_nonempty("CMS_AI_VERTEX_PROJECT")
        _require_nonempty("CMS_AI_VERTEX_LOCATION")

    langsmith_enabled = _env_flag(
        "CMS_AI_ENABLE_LANGSMITH",
        default=_env_flag("LANGSMITH_TRACING", default=False),
    )
    langsmith_tracing = _env_flag(
        "CMS_AI_ENABLE_LANGSMITH_TRACING",
        default=langsmith_enabled,
    ) or _env_flag("LANGSMITH_TRACING", default=False)
    langsmith_evals = _env_flag(
        "CMS_AI_ENABLE_LANGSMITH_EVALS",
        default=langsmith_enabled,
    )

    if langsmith_tracing or langsmith_evals:
        _require_nonempty("LANGSMITH_API_KEY")
