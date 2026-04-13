from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path


class WorkerBootstrapTests(unittest.TestCase):
    def test_worker_module_imports(self) -> None:
        if importlib.util.find_spec("temporalio") is None:
            self.skipTest("temporalio is only installed in the worker virtualenv")

        worker_path = Path(__file__).resolve().parents[1] / "worker.py"
        spec = importlib.util.spec_from_file_location("temporal_worker_module", worker_path)
        self.assertIsNotNone(spec)
        self.assertIsNotNone(spec.loader)

        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)

        self.assertIn("cms-migrations", module.TASK_QUEUES)
