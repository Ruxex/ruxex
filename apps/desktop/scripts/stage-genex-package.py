#!/usr/bin/env python3
"""Stage the complete local Genex runtime package consumed by the daemon."""

from __future__ import annotations

import argparse
import json
import os
import platform
import shutil
import stat
import sys
from pathlib import Path


def target_triple() -> str:
    machine = platform.machine()
    arch = {"AMD64": "x86_64", "aarch64": "aarch64", "arm64": "aarch64"}.get(
        machine, machine
    )
    if sys.platform == "darwin":
        return f"{arch}-apple-darwin"
    if sys.platform == "linux":
        libc = platform.libc_ver()[0]
        suffix = "unknown-linux-musl" if libc == "musl" else "unknown-linux-gnu"
        return f"{arch}-{suffix}"
    raise SystemExit(f"unsupported desktop runtime platform: {sys.platform}/{machine}")


def require_file(path: Path, label: str) -> Path:
    if not path.is_file():
        raise SystemExit(f"missing {label}: {path}")
    return path


def copy_executable(source: Path, destination: Path) -> None:
    require_file(source, destination.as_posix())
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)
    destination.chmod(destination.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


def validate_package(root: Path, version: str, target: str) -> None:
    required = [
        "genex-package.json",
        "bin/genex",
        "bin/genex-code-mode-host",
        "genex-path/rg",
    ]
    if sys.platform == "linux":
        required.append("genex-resources/bwrap")
    for relative in required:
        path = root / relative
        if not path.is_file():
            raise SystemExit(f"staged Genex package is missing {relative}")
        if relative != "genex-package.json" and not path.stat().st_mode & stat.S_IXUSR:
            raise SystemExit(f"staged Genex package file is not executable: {relative}")

    manifest = json.loads((root / "genex-package.json").read_text(encoding="utf-8"))
    if manifest != {"version": version, "target": target, "entrypoint": "bin/genex"}:
        raise SystemExit(f"unexpected staged Genex package manifest: {manifest!r}")


def discover_tool(name: str) -> Path | None:
    path = shutil.which(name)
    return Path(path) if path is not None else None


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--genex-bin", type=Path, required=True)
    parser.add_argument("--code-mode-host", type=Path, required=True)
    parser.add_argument("--rg", type=Path, default=discover_tool("rg"))
    parser.add_argument("--bwrap", type=Path, default=discover_tool("bwrap"))
    parser.add_argument("--version", default="0.1.0")
    parser.add_argument("--target", default=target_triple())
    args = parser.parse_args()

    if args.rg is None:
        raise SystemExit("missing rg: install ripgrep or pass --rg")
    if sys.platform == "linux" and args.bwrap is None:
        raise SystemExit("missing bwrap: install bubblewrap or pass --bwrap")
    output = args.output.resolve()
    if output == Path(output.anchor):
        raise SystemExit("refusing to stage into the filesystem root")
    ignore_contents = None
    ignore_file = output / ".gitignore"
    if ignore_file.is_file():
        ignore_contents = ignore_file.read_text(encoding="utf-8")
    if output.exists():
        shutil.rmtree(output)
    if ignore_contents is not None:
        output.mkdir(parents=True)
        ignore_file.write_text(ignore_contents, encoding="utf-8")
    (output / "genex-path").mkdir(parents=True)
    (output / "genex-resources").mkdir(parents=True)

    copy_executable(args.genex_bin, output / "bin/genex")
    copy_executable(args.code_mode_host, output / "bin/genex-code-mode-host")
    copy_executable(args.rg, output / "genex-path/rg")
    if args.bwrap is not None:
        copy_executable(args.bwrap, output / "genex-resources/bwrap")

    (output / "genex-package.json").write_text(
        json.dumps(
            {
                "version": args.version,
                "target": args.target,
                "entrypoint": "bin/genex",
            },
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )
    validate_package(output, args.version, args.target)
    print(output)


if __name__ == "__main__":
    main()
