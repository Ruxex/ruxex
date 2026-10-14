#!/usr/bin/env python3

import argparse
import os
from pathlib import Path
import subprocess


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Regenerate vendored app-server schema fixtures"
    )
    parser.add_argument(
        "--schema-root",
        type=Path,
        help="root directory containing the schema fixtures",
    )
    parser.add_argument(
        "--experimental",
        action="store_true",
        help="regenerate the checked-in experimental TypeScript tree and precomputed exports",
    )
    args = parser.parse_args()

    workspace_root = Path(__file__).resolve().parents[1]
    repository_schema_root = (
        workspace_root / "crates" / "app-server-protocol" / "schema"
    )
    schema_root = args.schema_root or repository_schema_root

    env = os.environ.copy()
    env["RUXEX_APP_SERVER_SCHEMA_ROOT"] = str(schema_root)
    env["RUXEX_APP_SERVER_SCHEMA_EXPERIMENTAL"] = "1" if args.experimental else "0"
    subprocess.run(
        [
            "cargo",
            "test",
            "-p",
            "ruxex-app-server-protocol",
            "--lib",
            "schema_fixtures_tests::write_schema_fixtures_from_env",
            "--",
            "--exact",
            "--ignored",
        ],
        cwd=workspace_root,
        env=env,
        check=True,
    )


if __name__ == "__main__":
    main()
