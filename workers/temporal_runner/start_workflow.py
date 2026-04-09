from __future__ import annotations

import asyncio
import json
import os
import sys

from temporalio.client import Client

from rusty_cms_temporal.env import load_environment, validate_runtime_environment
from rusty_cms_temporal.workflows import CmsWorkflowRequest


async def main() -> None:
    load_environment()
    validate_runtime_environment()
    request = json.load(sys.stdin)
    endpoint = os.environ.get("TEMPORAL_GRPC_ENDPOINT", "localhost:7233")
    namespace = os.environ.get("TEMPORAL_NAMESPACE", "default")

    client = await Client.connect(endpoint, namespace=namespace)
    handle = await client.start_workflow(
        CmsWorkflowRequest.run,
        request,
        id=f"cms-{request['id']}",
        task_queue=request["temporal_queue"],
    )

    json.dump(
        {
            "workflow_id": handle.id,
            "run_id": getattr(handle, "run_id", None),
            "task_queue": request["temporal_queue"],
            "namespace": namespace,
        },
        sys.stdout,
    )
    sys.stdout.write("\n")


if __name__ == "__main__":
    asyncio.run(main())
