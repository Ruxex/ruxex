import tempfile
import unittest
from pathlib import Path

from extract_build_warnings import (
    extract_warnings,
    find_latest_run,
    shared_warning_signatures,
)


class ExtractBuildWarningsTests(unittest.TestCase):
    def test_extracts_rust_warning_and_c_warning_without_cargo_wrapper_lines(self):
        log = """
        2026-10-15T00:00:00Z ##[group]Run just check
        [33mwarning[0m: unused import: `crate::unused`
          [94m-->[0m crates/example/src/lib.rs:4:5
        warning: ruxex-bwrap@0.0.0: /home/runner/work/ruxex/ruxex/crates/bwrap/../vendor/bubblewrap/network.c: In function 'setup':
        warning: ruxex-bwrap@0.0.0: /home/runner/work/ruxex/ruxex/crates/bwrap/../vendor/bubblewrap/network.c:53:10: warning: missing initializer for field 'nl_pid' [-Wmissing-field-initializers]
        warning: ruxex-bwrap@0.0.0: /home/runner/work/ruxex/ruxex/crates/bwrap/../vendor/bubblewrap/network.c:142:10: note: 'nl_pid' declared here
        warning: `ruxex-example` (lib) generated 1 warning
        """

        records = extract_warnings(log, "ubuntu", "Linux")

        self.assertEqual([record.message for record in records], [
            "unused import: `crate::unused`",
            "missing initializer for field 'nl_pid' [-Wmissing-field-initializers]",
        ])
        self.assertEqual(records[0].location, "crates/example/src/lib.rs:4:5")
        self.assertEqual(records[1].location, "vendor/bubblewrap/network.c:53:10")
        self.assertEqual(records[0].phase, "check")

    def test_latest_run_uses_numeric_workflow_id(self):
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            (root / "logs_100").mkdir()
            (root / "logs_101 (1)").mkdir()
            (root / "logs_not-a-run").mkdir()

            self.assertEqual(find_latest_run(root).name, "logs_101 (1)")

    def test_shared_signatures_ignore_platform_absolute_paths(self):
        linux = extract_warnings(
            "warning: unused import: `crate::unused`\n --> /home/runner/work/ruxex/ruxex/crates/example/src/lib.rs:4:5\n",
            "ubuntu",
            "Linux",
        )
        macos = extract_warnings(
            "warning: unused import: `crate::unused`\n --> /Users/runner/work/ruxex/ruxex/crates/example/src/lib.rs:4:5\n",
            "macos",
            "macOS",
        )

        self.assertEqual(shared_warning_signatures(linux + macos), {
            "unused import: `crate::unused`|crates/example/src/lib.rs:4:5",
        })


if __name__ == "__main__":
    unittest.main()
