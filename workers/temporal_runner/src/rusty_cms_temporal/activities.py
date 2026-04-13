from __future__ import annotations

from temporalio import activity

from rusty_cms_temporal.ai.orchestrator import execute_ai_workflow
from rusty_cms_temporal.migrations import execute_page_document_extraction, execute_site_discovery


@activity.defn(name="execute_ai_content_operation")
async def execute_ai_content_operation(request: dict) -> dict:
    return await execute_ai_workflow(request)


@activity.defn(name="execute_site_discovery")
async def execute_site_discovery_activity(request: dict) -> dict:
    return await execute_site_discovery(request)


@activity.defn(name="execute_page_document_extraction")
async def execute_page_document_extraction_activity(request: dict) -> dict:
    return await execute_page_document_extraction(request)
