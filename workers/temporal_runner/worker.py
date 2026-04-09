from __future__ import annotations

import asyncio
import os
from contextlib import AsyncExitStack

from temporalio.client import Client
from temporalio.worker import Worker

from rusty_cms_temporal.activities import execute_ai_content_operation
from rusty_cms_temporal.workflows import CmsWorkflowRequest


TASK_QUEUES = [
    "cms-publish",
    "cms-restore",
    "cms-bulk",
    "cms-agent-ops",
]


async def main() -> None:
    endpoint = os.environ.get("TEMPORAL_GRPC_ENDPOINT", "localhost:7233")
    namespace = os.environ.get("TEMPORAL_NAMESPACE", "default")
    client = await Client.connect(endpoint, namespace=namespace)

    async with AsyncExitStack() as stack:
        for task_queue in TASK_QUEUES:
            worker = Worker(
                client,
                task_queue=task_queue,
                activities=[execute_ai_content_operation],
                workflows=[CmsWorkflowRequest],
            )
            await stack.enter_async_context(worker)

        print(
            f"Temporal worker running for namespace={namespace} endpoint={endpoint} "
            f"queues={','.join(TASK_QUEUES)}",
            flush=True,
        )
        await asyncio.Future()


if __name__ == "__main__":
    asyncio.run(main())
