from __future__ import annotations

import asyncio
import json
import os
import sys

from temporalio.client import Client

from rusty_cms_temporal.env import load_environment, validate_runtime_environment


async def main() -> None:
    load_environment()
    validate_runtime_environment()

    payload = json.load(sys.stdin)
    workflow_id = str(payload["workflow_id"])
    timeout_seconds = float(payload.get("timeout_seconds", 2.0))
    endpoint = os.environ.get("TEMPORAL_GRPC_ENDPOINT", "localhost:7233")
    namespace = os.environ.get("TEMPORAL_NAMESPACE", "default")

    client = await Client.connect(endpoint, namespace=namespace)
    handle = client.get_workflow_handle(workflow_id)

    try:
        result = await asyncio.wait_for(handle.result(), timeout=timeout_seconds)
    except TimeoutError:
        raise RuntimeError(
            f"workflow {workflow_id} has not completed within {timeout_seconds:.1f}s"
        ) from None

    json.dump(
        {
            "workflow_id": workflow_id,
            "namespace": namespace,
            "result": result,
        },
        sys.stdout,
    )
    sys.stdout.write("\n")


if __name__ == "__main__":
    asyncio.run(main())
