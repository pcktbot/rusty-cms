from __future__ import annotations

from datetime import timedelta

from temporalio import workflow


@workflow.defn(name="cms_workflow_request")
class CmsWorkflowRequest:
    @workflow.run
    async def run(self, request: dict) -> dict:
        if request["kind"] == "SiteMigration":
            return await workflow.execute_activity(
                "execute_site_migration",
                request,
                start_to_close_timeout=timedelta(minutes=20),
            )

        if request["kind"] == "AiContentOperation":
            return await workflow.execute_activity(
                "execute_ai_content_operation",
                request,
                start_to_close_timeout=timedelta(minutes=10),
            )

        return {
            "accepted": True,
            "workflow_kind": request["kind"],
            "site_id": request["site_id"],
            "branch_name": request["branch_name"],
            "requested_runtime": request["requested_runtime"],
            "temporal_queue": request["temporal_queue"],
        }
