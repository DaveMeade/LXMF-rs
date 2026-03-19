#!/usr/bin/env python3
import os
import subprocess
import sys
from pathlib import Path


SUPPORTED_CASES = {
    "direct_python_to_rust",
    "propagated_python_to_rust",
}


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: python_compat_harness.py <case_id>", file=sys.stderr)
        return 2

    case_id = sys.argv[1]
    if case_id not in SUPPORTED_CASES:
        print(f"unsupported compatibility case: {case_id}", file=sys.stderr)
        return 2

    repo_root = Path(__file__).resolve().parents[2]
    smoke_script = repo_root / "tools" / "scripts" / "python-lxmd-rust-lxmd-smoke.sh"
    env = os.environ.copy()
    env["COMPAT_CASE"] = case_id

    result = subprocess.run(
        [str(smoke_script)],
        cwd=repo_root,
        env=env,
        check=False,
    )
    return result.returncode


if __name__ == "__main__":
    raise SystemExit(main())
