import json
import subprocess
from pathlib import Path

import pytest

from .utils import Command, wait_for_port


@pytest.fixture(scope="session")
def cluster(tmp_path_factory):
    data = tmp_path_factory.mktemp("data")
    base_port = 10000
    config = Path(__file__).parent / "config.yaml"
    with subprocess.Popen(
        f"pystarport serve --config {config} --data {data} --base_port {base_port} "
        "--quiet",
        shell=True,
    ) as proc:
        try:
            print("start in path:", data, "base port:", base_port)
            print("wait for rpc of first node to be ready")
            wait_for_port(10007, timeout=10)
            wasmd = Command(
                "wasmd",
                data / "chainmaind" / "node0",
                "tcp://localhost:10007",
                "chainmaind",
            )
            wasmd.wait_for_block(1)
            yield wasmd
        finally:
            proc.terminate()
