from __future__ import annotations

from temporalio import workflow


@workflow.defn(name="cms_workflow_request")
class CmsWorkflowRequest:
    @workflow.run
    async def run(self, request: dict) -> dict:
        return {
            "accepted": True,
            "workflow_kind": request["kind"],
            "site_id": request["site_id"],
            "branch_name": request["branch_name"],
            "requested_runtime": request["requested_runtime"],
            "temporal_queue": request["temporal_queue"],
        }

