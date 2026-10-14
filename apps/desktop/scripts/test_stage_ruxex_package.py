import importlib.util
import os
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("stage-ruxex-package.py")
SPEC = importlib.util.spec_from_file_location("stage_ruxex_package", SCRIPT)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class StageRuxexPackageTest(unittest.TestCase):
    def test_preserves_repository_ignore_file(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            inputs = root / "inputs"
            inputs.mkdir()
            for name in ("ruxex", "ruxex-code-mode-host", "rg", "bwrap"):
                path = inputs / name
                path.write_text(name, encoding="utf-8")
                path.chmod(0o755)

            output = root / "package"
            output.mkdir()
            ignore_file = output / ".gitignore"
            ignore_file.write_text("*\n!.gitignore\n", encoding="utf-8")
            old_argv = MODULE.sys.argv
            try:
                MODULE.sys.argv = [
                    str(SCRIPT),
                    "--output",
                    str(output),
                    "--ruxex-bin",
                    str(inputs / "ruxex"),
                    "--code-mode-host",
                    str(inputs / "ruxex-code-mode-host"),
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

            self.assertEqual(ignore_file.read_text(encoding="utf-8"), "*\n!.gitignore\n")

    def test_discovers_runtime_tools_from_path(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            inputs = root / "inputs"
            inputs.mkdir()
            for name in ("ruxex", "ruxex-code-mode-host"):
                path = inputs / name
                path.write_text(name, encoding="utf-8")
                path.chmod(0o755)

            tools = root / "tools"
            tools.mkdir()
            for name in ("rg", "bwrap"):
                path = tools / name
                path.write_text(name, encoding="utf-8")
                path.chmod(0o755)

            output = root / "package"
            old_argv = MODULE.sys.argv
            old_path = os.environ.get("PATH")
            try:
                MODULE.sys.argv = [
                    str(SCRIPT),
                    "--output",
                    str(output),
                    "--ruxex-bin",
                    str(inputs / "ruxex"),
                    "--code-mode-host",
                    str(inputs / "ruxex-code-mode-host"),
                    "--version",
                    "0.1.0",
                    "--target",
                    "x86_64-unknown-linux-gnu",
                ]
                os.environ["PATH"] = str(tools)
                MODULE.main()
            finally:
                MODULE.sys.argv = old_argv
                if old_path is None:
                    os.environ.pop("PATH", None)
                else:
                    os.environ["PATH"] = old_path

            self.assertTrue((output / "ruxex-path/rg").is_file())
            self.assertTrue((output / "ruxex-resources/bwrap").is_file())

    def test_stages_layout_consumed_by_install_context(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            inputs = root / "inputs"
            inputs.mkdir()
            for name in ("ruxex", "ruxex-code-mode-host", "rg", "bwrap"):
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
                    "--ruxex-bin",
                    str(inputs / "ruxex"),
                    "--code-mode-host",
                    str(inputs / "ruxex-code-mode-host"),
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
                    "bin/ruxex",
                    "bin/ruxex-code-mode-host",
                    "ruxex-package.json",
                    "ruxex-path",
                    "ruxex-path/rg",
                    "ruxex-resources",
                    "ruxex-resources/bwrap",
                ],
            )
            self.assertTrue(os.access(output / "bin/ruxex", os.X_OK))
            MODULE.validate_package(output, "0.1.0", "x86_64-unknown-linux-gnu")


if __name__ == "__main__":
    unittest.main()
