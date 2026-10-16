import subprocess
import unittest
from pathlib import Path


REPOSITORY_ROOT = Path(__file__).parents[3]


class DesktopJustfileTest(unittest.TestCase):
    def test_desktop_runs_runtime_build_and_staging_before_tauri(self) -> None:
        result = subprocess.run(
            ["just", "--dry-run", "desktop"],
            cwd=REPOSITORY_ROOT,
            check=True,
            capture_output=True,
            text=True,
        )

        dry_run = result.stdout + result.stderr
        self.assertIn(
            "cargo build -p ruxex-provider-host-example -p ruxex-code-mode-host",
            dry_run,
        )
        self.assertIn("stage:runtime", dry_run)
        self.assertIn("tauri dev", dry_run)


if __name__ == "__main__":
    unittest.main()
