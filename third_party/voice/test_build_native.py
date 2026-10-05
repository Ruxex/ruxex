"""Check native build preconditions and subprocess failure propagation."""

import json
import os
from pathlib import Path
import platform
import shlex
import shutil
import subprocess
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

from build_native import NativeBuild, validate_target


class NativeBuildTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        machine = platform.machine().lower()
        architecture = {"arm64": "aarch64", "amd64": "x86_64"}.get(machine, machine)
        suffix = {
            "Darwin": "apple-darwin",
            "Linux": "unknown-linux-gnu",
        }[platform.system()]
        self.args = SimpleNamespace(
            target=f"{architecture}-{suffix}",
            deployment_target="11.0",
            output=self.root / "build output",
            archives=self.root / "archives",
            cc=Path(sys.executable),
            cxx=Path(sys.executable),
            cmake=Path(sys.executable),
            make=Path(sys.executable),
            pkg_config=Path(sys.executable),
            shell=Path(sys.executable),
            bootstrap_make=None,
            jobs=2,
        )
        self.environment = dict(os.environ)

    def test_cli_entrypoint_imports_its_sibling_with_safe_path_enabled(self):
        result = subprocess.run(
            [
                sys.executable,
                "-P",
                str(Path(__file__).with_name("build_native.py")),
                "--help",
            ],
            cwd=self.root,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("--archives", result.stdout)

    def test_rejects_cross_host_and_musl_builds(self):
        for target, system, machine, libc in [
            ("aarch64-apple-darwin", "Darwin", "x86_64", ""),
            ("x86_64-unknown-linux-gnu", "Linux", "x86_64", "musl"),
            ("x86_64-unknown-linux-musl", "Linux", "x86_64", "musl"),
        ]:
            with self.subTest(target=target, system=system):
                with self.assertRaises(ValueError):
                    validate_target(target, system, machine, libc, "11.0")

    def test_requires_explicit_macos_deployment_target(self):
        with self.assertRaisesRegex(ValueError, "Declare the macOS deployment target"):
            validate_target("aarch64-apple-darwin", "Darwin", "arm64", "", None)

    def test_missing_tool_does_not_create_output(self):
        self.args.cc = self.root / "missing compiler"
        with self.assertRaisesRegex(ValueError, "Missing build tool"):
            NativeBuild(self.args, self.environment)
        self.assertFalse(self.args.output.exists())

    def test_bootstrap_links_cpp_runtime_through_compiler_symlink(self):
        tools = {name: shutil.which(name) for name in ("clang", "cmake", "make")}
        if not all(tools.values()):
            self.skipTest("Requires clang, CMake and make")
        compiler = self.root / "clang++"
        compiler.symlink_to(Path(tools["clang"]).resolve())
        self.args.cc = Path(tools["clang"])
        self.args.cxx = compiler
        self.args.cmake = Path(tools["cmake"])
        self.args.make = Path(tools["make"])
        headers = self.root / "declared headers"
        headers.mkdir()
        (headers / "build_flag.h").write_text(
            '#define BUILD_FLAG "linked with flags"\n'
        )
        self.args.cxx_flag = [f"-I{headers}"]
        for name in ("ar", "ranlib"):
            tool = shutil.which(name)
            if tool is None:
                self.skipTest(f"Requires {name}")
            wrapper = self.root / f"declared {name}"
            wrapper.write_text(
                "#!/bin/sh\n"
                f"touch {shlex.quote(str(self.root / (name + '.used')))}\n"
                f'exec {shlex.quote(tool)} "$@"\n'
            )
            wrapper.chmod(0o755)
            setattr(self.args, name, wrapper)
        source = self.root / "cpp-source"
        source.mkdir()
        (source / "CMakeLists.txt").write_text(
            "cmake_minimum_required(VERSION 3.15)\n"
            "project(cpp_driver LANGUAGES CXX)\n"
            "add_library(message STATIC message.cpp)\n"
            "add_executable(cpp_driver main.cpp)\n"
            "target_link_libraries(cpp_driver PRIVATE message)\n"
            "install(TARGETS cpp_driver DESTINATION bin)\n"
        )
        (source / "main.cpp").write_text(
            "#include <iostream>\nconst char* message();\nint main() { std::cout << message(); }\n"
        )
        (source / "message.cpp").write_text(
            '#include "build_flag.h"\nconst char* message() { return BUILD_FLAG; }\n'
        )
        build = NativeBuild(self.args, self.environment)
        build.output.mkdir()
        build.sources = {"cpp-driver": source}
        build.cmake("cpp-driver", [], bootstrap=True)
        result = subprocess.run(
            [build.tools / "bin" / "cpp_driver"],
            check=True,
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.stdout, "linked with flags")
        self.assertTrue(
            all((self.root / (name + ".used")).is_file() for name in ("ar", "ranlib"))
        )

    def test_libffi_rejects_quoted_archive_tool_paths(self):
        for name in ("ar", "ranlib"):
            for index, path in enumerate(("declared tool", "declared'tool")):
                with self.subTest(name=name, path=path):
                    (self.root / path).touch()
                    setattr(self.args, name, self.root / path)
                    self.args.output = self.root / f"{name}-{index}"
                    build = NativeBuild(self.args, self.environment)
                    with (
                        patch(
                            "build_native.prepare_sources",
                            side_effect=lambda *args: (build.output / "build").mkdir(),
                        ),
                        patch.object(build, "cmake"),
                        patch.object(build, "meson"),
                        patch.object(build, "run"),
                        self.assertRaisesRegex(ValueError, "libffi .* path"),
                    ):
                        build.build()
                    setattr(self.args, name, None)

    def test_libffi_compiler_receives_literal_defines_without_shell_quotes(self):
        compiler = shutil.which("cc")
        if compiler is None:
            self.skipTest("Requires a C compiler")
        self.args.cc = Path(compiler)
        make = shutil.which("make")
        if make is None:
            self.skipTest("Requires Make")
        for index, (flag, expected) in enumerate(
            [
                ('-DBUILD_FLAG="redacted"', "redacted\n"),
                (r'-DBUILD_FLAG="C:\\voice"', "C:\\voice\n"),
                ('-DBUILD_FLAG="two words"', None),
            ]
        ):
            with self.subTest(flag=flag):
                self.args.c_flag = [flag]
                self.args.output = self.root / str(index)
                build = NativeBuild(self.args, self.environment)
                with (
                    patch(
                        "build_native.prepare_sources",
                        side_effect=lambda *args: (build.output / "build").mkdir(),
                    ),
                    patch.object(build, "cmake"),
                    patch.object(build, "meson"),
                    patch.object(build, "run") as run,
                ):
                    if " " in flag:
                        with self.assertRaisesRegex(ValueError, "libffi flags"):
                            build.build()
                        continue
                    build.build()
                environment = next(
                    call.kwargs["environment"]
                    for call in run.call_args_list
                    if call.args[0] == "libffi-configure"
                )
                (build.output / "probe.c").write_text(
                    "#include <stdio.h>\nint main(void) { puts(BUILD_FLAG); }\n"
                )
                subprocess.run(
                    ["/bin/sh", "-c", "$CC $CFLAGS probe.c -o probe"],
                    cwd=build.output,
                    env=environment,
                    check=True,
                )
                result = subprocess.run(
                    [build.output / "probe"], check=True, capture_output=True, text=True
                )
                self.assertEqual(result.stdout, expected)
                (build.output / "Makefile").write_text(
                    f"CC={compiler}\nCFLAGS={environment['CFLAGS']} -fexceptions\n"
                    "MAKEOVERRIDES=\nall:\n\t$(MAKE) nested\n"
                    "nested:\n\t$(CC) $(CFLAGS) probe.c -o probe\n\t./probe\n"
                )
                result = subprocess.run(
                    [make, "--silent"],
                    cwd=build.output,
                    check=True,
                    capture_output=True,
                    text=True,
                )
                self.assertEqual(result.stdout, expected)

    def test_cmake_installs_relative_library_paths(self):
        for argument, tool in (
            ("cc", "cc"),
            ("cxx", "c++"),
            ("cmake", "cmake"),
            ("make", "make"),
        ):
            setattr(self.args, argument, Path(shutil.which(tool)))
        source = self.root / "library-source"
        source.mkdir()
        self.args.c_flag = ["-DBUILD_FLAG=42"]
        self.args.link_flag = ["-Wl,-rpath,declared-relative-path"]
        (source / "CMakeLists.txt").write_text(
            "cmake_minimum_required(VERSION 3.15)\nproject(relative_paths LANGUAGES C)\n"
            "add_library(fixture SHARED fixture.c)\ninstall(TARGETS fixture DESTINATION lib)\n"
        )
        (source / "fixture.c").write_text("int fixture(void) { return BUILD_FLAG; }\n")
        build = NativeBuild(self.args, self.environment)
        build.output.mkdir()
        build.sources = {"fixture": source}
        build.cmake("fixture", [], bootstrap=True)
        if sys.platform == "darwin":
            from macos_runtime import inspect

            metadata = inspect(build.tools / "lib/libfixture.dylib", self.args.target)
            self.assertEqual(
                (metadata.identity, metadata.rpaths),
                ("@rpath/libfixture.dylib", ("declared-relative-path", "@loader_path")),
            )
        else:
            from linux_runtime import inspect

            metadata = inspect(build.tools / "lib/libfixture.so", self.args.target)
            self.assertEqual(
                (metadata.identity, metadata.rpaths),
                ("libfixture.so", ("29=declared-relative-path:$ORIGIN",)),
            )

    def test_build_refuses_existing_output_before_reading_sources(self):
        self.args.output.mkdir()
        (self.args.output / "keep").write_text("untouched")
        with self.assertRaises(FileExistsError):
            NativeBuild(self.args, self.environment).build()
        self.assertEqual(
            {p.name: p.read_text() for p in self.args.output.iterdir()},
            {"keep": "untouched"},
        )

    def test_failure_retains_log_and_state_without_completion_marker(self):
        build = NativeBuild(self.args, self.environment)
        build.output.mkdir()
        command = [
            sys.executable,
            "-c",
            "print('synthetic failure'); raise SystemExit(23)",
        ]
        with self.assertRaises(subprocess.CalledProcessError) as error:
            build.run("failure", command)
        self.assertEqual(error.exception.returncode, 23)
        self.assertEqual(
            (build.output / "failure.log").read_text().strip(), "synthetic failure"
        )
        self.assertEqual(
            json.loads((build.output / "build-state.json").read_text()),
            {
                "target": self.args.target,
                "deployment_target": "11.0",
                "flags": {"CFLAGS": [], "CXXFLAGS": [], "LDFLAGS": []},
                "steps": [{"name": "failure", "command": command, "exit_code": 23}],
            },
        )
        self.assertFalse((build.output / "built.json").exists())

    def test_ambient_native_discovery_variables_are_not_inherited(self):
        inherited = {
            **self.environment,
            "PKG_CONFIG_PATH": "/ambient",
            "CMAKE_PREFIX_PATH": "/ambient",
            "CPATH": "/ambient",
            "CFLAGS": "-I/ambient",
            "LDFLAGS": "-L/ambient",
        }
        environment = NativeBuild(self.args, inherited).environment
        self.assertFalse(any("/ambient" in value for value in environment.values()))
        self.assertEqual(environment["PKG_CONFIG_PATH"], "")

    def test_meson_receives_private_link_inputs_with_spaces(self):
        if platform.system() == "Darwin":
            self.args.cc = Path(shutil.which("cc"))
        self.args.c_flag = ["-DDECLARED_C=1"]
        self.args.cxx_flag = ["-DDECLARED_CXX=1"]
        self.args.link_flag = ["-Ldeclared input with spaces"]
        build = NativeBuild(self.args, self.environment)
        self.assertEqual(
            build.record["flags"],
            {
                "CFLAGS": self.args.c_flag,
                "CXXFLAGS": self.args.cxx_flag,
                "LDFLAGS": self.args.link_flag,
            },
        )
        build.sources = {"meson": self.root / "meson", "glib": self.root / "glib"}
        with patch.object(build, "run"):
            build.meson("glib", [])
        if platform.system() == "Darwin":
            subprocess.run(
                [
                    build.environment["OBJC"],
                    *shlex.split(build.environment["OBJCFLAGS"]),
                    "-x",
                    "objective-c",
                    "-fsyntax-only",
                    "-",
                ],
                input="#if DECLARED_C != 1\n#error missing declared flags\n#endif\n",
                text=True,
                check=True,
            )
        quote = shlex.join
        include = f"-I{build.prefix / 'include'}"
        self.assertEqual(
            {name: build.environment[name] for name in ("CFLAGS", "CXXFLAGS")},
            {
                "CFLAGS": quote([*self.args.c_flag, include]),
                "CXXFLAGS": quote([*self.args.cxx_flag, include]),
            },
        )
        self.assertEqual(
            shlex.split(build.environment["LDFLAGS"]),
            [
                *self.args.link_flag,
                f"-L{build.prefix / 'lib'}",
                (
                    "-Wl,-rpath,$ORIGIN:$ORIGIN/.."
                    if build.args.target.endswith("unknown-linux-gnu")
                    else f"-Wl,-rpath,{build.prefix / 'lib'}"
                ),
            ],
        )
