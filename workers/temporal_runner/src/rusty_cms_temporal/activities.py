from __future__ import annotations

from temporalio import activity

from rusty_cms_temporal.ai.orchestrator import execute_ai_workflow


@activity.defn(name="execute_ai_content_operation")
async def execute_ai_content_operation(request: dict) -> dict:
    return await execute_ai_workflow(request)
