from __future__ import annotations

from datetime import timedelta

from temporalio import workflow


@workflow.defn(name="cms_workflow_request")
class CmsWorkflowRequest:
    @workflow.run
    async def run(self, request: dict) -> dict:
        if request["kind"] == "SiteMigration":
            payload = dict(request.get("input_payload") or {})
            discovery = await workflow.execute_activity(
                "execute_site_discovery",
                request,
                start_to_close_timeout=timedelta(minutes=20),
            )
            discovery_payload = discovery.get("migration", {})
            extraction_request = {
                **request,
                "input_payload": {
                    **payload,
                    "pages": discovery_payload.get("pages", []),
                },
            }
            extraction = await workflow.execute_activity(
                "execute_page_document_extraction",
                extraction_request,
                start_to_close_timeout=timedelta(minutes=20),
            )
            extraction_payload = extraction.get("migration", {})

            return {
                "accepted": True,
                "workflow_kind": request["kind"],
                "site_id": request["site_id"],
                "branch_name": request["branch_name"],
                "requested_runtime": request["requested_runtime"],
                "temporal_queue": request["temporal_queue"],
                "migration": {
                    "status": extraction_payload.get("status", "review_ready"),
                    "homepage_url": discovery_payload.get("homepage_url"),
                    "client_id": discovery_payload.get("client_id"),
                    "location_id": discovery_payload.get("location_id"),
                    "page_count_guess": extraction_payload.get("page_count_guess", 0),
                    "pages": extraction_payload.get("pages", []),
                    "warnings": [
                        *discovery_payload.get("warnings", []),
                        *extraction_payload.get("warnings", []),
                    ],
                },
            }

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
