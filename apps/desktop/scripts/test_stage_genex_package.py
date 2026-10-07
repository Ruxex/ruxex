import importlib.util
import os
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("stage-genex-package.py")
SPEC = importlib.util.spec_from_file_location("stage_genex_package", SCRIPT)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class StageGenexPackageTest(unittest.TestCase):
    def test_stages_layout_consumed_by_install_context(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            inputs = root / "inputs"
            inputs.mkdir()
            for name in ("genex", "genex-code-mode-host", "rg", "bwrap"):
                path = inputs / name
                path.write_text(name, encoding="utf-8")
                path.chmod(0o755)

            output = root / "package"
            old_argv = MODULE.sys.argv
            try:
                MODULE.sys.argv = [
                    str(SCRIPT),
                    "--output",
                    str(output),
                    "--genex-bin",
                    str(inputs / "genex"),
                    "--code-mode-host",
                    str(inputs / "genex-code-mode-host"),
                    "--rg",
                    str(inputs / "rg"),
                    "--bwrap",
                    str(inputs / "bwrap"),
                    "--version",
                    "0.1.0",
                    "--target",
                    "x86_64-unknown-linux-gnu",
                ]
                MODULE.main()
            finally:
                MODULE.sys.argv = old_argv

            self.assertEqual(
                sorted(path.relative_to(output).as_posix() for path in output.rglob("*")),
                [
                    "bin",
                    "bin/genex",
                    "bin/genex-code-mode-host",
                    "genex-package.json",
                    "genex-path",
                    "genex-path/rg",
                    "genex-resources",
                    "genex-resources/bwrap",
                ],
            )
            self.assertTrue(os.access(output / "bin/genex", os.X_OK))
            MODULE.validate_package(output, "0.1.0", "x86_64-unknown-linux-gnu")


if __name__ == "__main__":
    unittest.main()
