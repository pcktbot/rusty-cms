from __future__ import annotations

from temporalio import activity

from rusty_cms_temporal.ai.orchestrator import execute_ai_workflow
from rusty_cms_temporal.migrations import execute_site_migration


@activity.defn(name="execute_ai_content_operation")
async def execute_ai_content_operation(request: dict) -> dict:
    return await execute_ai_workflow(request)


@activity.defn(name="execute_site_migration")
async def execute_site_migration_activity(request: dict) -> dict:
    return await execute_site_migration(request)
