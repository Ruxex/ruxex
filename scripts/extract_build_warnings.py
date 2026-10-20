#!/usr/bin/env python3
"""Extract and compare warnings from the normal Linux/macOS build logs."""

from __future__ import annotations

import argparse
import posixpath
import re
from collections import defaultdict
from dataclasses import dataclass, field
from pathlib import Path
from typing import Iterable


ANSI_RE = re.compile(r"\x1b\[[0-?]*[ -/]*[@-~]")
RUN_ID_RE = re.compile(r"^logs_(\d+)")
BUILD_LOG_RE = re.compile(r"^\d+_build \(([^)]+)\)\.txt$")
PHASE_RE = re.compile(r"Run just (check|build(?: --workspace)?)")
WARNING_RE = re.compile(r"warning:")
LOCATION_RE = re.compile(
    r"(?P<path>(?:/[\w./-]+|[\w.-]+(?:/[\w./-]+)+)):(?P<line>\d+):(?P<column>\d+)"
)
PACKAGE_RE = re.compile(r"^[\w.-]+@[\w.+-]+$")
GENERATED_RE = re.compile(r"generated \d+ warnings?(?: \(.*\))?$")


@dataclass(frozen=True)
class WarningRecord:
    platform: str
    phase: str
    message: str
    location: str | None
    package: str | None
    source: str
    line_number: int

    @property
    def signature(self) -> str:
        return f"{self.message}|{self.location or '<unknown>'}"


@dataclass
class WarningGroup:
    message: str
    location: str | None
    packages: set[str] = field(default_factory=set)
    phases: set[str] = field(default_factory=set)
    sources: set[str] = field(default_factory=set)
    occurrences: int = 0


def clean_line(line: str) -> str:
    return ANSI_RE.sub("", line).rstrip("\r\n")


def normalize_path(path: str) -> str:
    path = posixpath.normpath(path.replace("\\", "/"))
    for marker in ("/vendor/", "/crates/"):
        if marker in path:
            return path[path.index(marker) + 1 :]
    return path.lstrip("./")


def find_location(line: str) -> str | None:
    matches = list(LOCATION_RE.finditer(line))
    if not matches:
        return None
    match = matches[-1]
    return f"{normalize_path(match.group('path'))}:{match.group('line')}:{match.group('column')}"


def warning_message(line: str) -> tuple[str, str | None] | None:
    markers = list(WARNING_RE.finditer(line))
    if not markers:
        return None

    message = line[markers[-1].end() :].strip()
    if not message or GENERATED_RE.search(message):
        return None

    # Cargo prefixes compiler output from a build script with the package
    # name. Keep the nested compiler warning, but ignore its wrapper lines.
    if len(markers) == 1 and re.match(r"^[\w.-]+@[\w.+-]+:\s", message):
        return None

    package = None
    if len(markers) > 1:
        prefix = line[markers[0].end() : markers[-1].start()].strip()
        candidate = prefix.split(":", 1)[0]
        if PACKAGE_RE.fullmatch(candidate):
            package = candidate
    return message, package


def extract_warnings(text: str, source: str, platform: str) -> list[WarningRecord]:
    lines = text.splitlines()
    records: list[WarningRecord] = []
    phase = "unknown"

    for index, raw_line in enumerate(lines):
        line = clean_line(raw_line)
        phase_match = PHASE_RE.search(line)
        if phase_match:
            phase = "build" if phase_match.group(1).startswith("build") else "check"
            continue

        parsed = warning_message(line)
        if parsed is None:
            continue
        message, package = parsed
        location = find_location(line)

        for following_raw_line in lines[index + 1 : index + 9]:
            following_line = clean_line(following_raw_line)
            if PHASE_RE.search(following_line) or WARNING_RE.search(following_line):
                break
            location = location or find_location(following_line)
            if location:
                break

        records.append(
            WarningRecord(
                platform=platform,
                phase=phase,
                message=message,
                location=location,
                package=package,
                source=source,
                line_number=index + 1,
            )
        )

    return records


def shared_warning_signatures(records: Iterable[WarningRecord]) -> set[str]:
    platforms_by_signature: dict[str, set[str]] = defaultdict(set)
    for record in records:
        platforms_by_signature[record.signature].add(record.platform)
    return {
        signature
        for signature, platforms in platforms_by_signature.items()
        if {"Linux", "macOS"}.issubset(platforms)
    }


def find_latest_run(logs_root: Path) -> Path:
    candidates: list[tuple[int, float, Path]] = []
    for path in logs_root.iterdir():
        if not path.is_dir():
            continue
        match = RUN_ID_RE.match(path.name)
        if match:
            candidates.append((int(match.group(1)), path.stat().st_mtime, path))
    if not candidates:
        raise FileNotFoundError(f"no extracted workflow log directories found under {logs_root}")
    return max(candidates)[2]


def build_logs(run_dir: Path) -> list[tuple[str, Path]]:
    result: list[tuple[str, Path]] = []
    for path in run_dir.iterdir():
        match = BUILD_LOG_RE.match(path.name)
        if not match:
            continue
        runner = match.group(1).lower()
        if runner.startswith("macos"):
            platform = "macOS"
        elif runner.startswith("ubuntu") or runner.startswith("linux"):
            platform = "Linux"
        else:
            platform = match.group(1)
        result.append((platform, path))
    return sorted(result, key=lambda item: item[1].name)


def group_records(records: Iterable[WarningRecord]) -> dict[str, dict[str, WarningGroup]]:
    grouped: dict[str, dict[str, WarningGroup]] = defaultdict(dict)
    for record in records:
        platform_groups = grouped[record.platform]
        group = platform_groups.setdefault(
            record.signature,
            WarningGroup(message=record.message, location=record.location),
        )
        group.occurrences += 1
        group.phases.add(record.phase)
        group.sources.add(record.source)
        if record.package:
            group.packages.add(record.package)
    return grouped


def render_report(run_dir: Path, records: list[WarningRecord]) -> str:
    grouped = group_records(records)
    shared = shared_warning_signatures(records)
    lines = [
        f"# Normal build warnings ({run_dir.name})",
        "",
        "対象は通常ビルドの `just check` と `just build --workspace` のログです。",
        "Cargoのサマリー行、ANSI装飾、Cargoがビルドスクリプト出力に付けるラッパー行は除外しています。",
        "同じ警告の `check` / `build` 間の再出力は発生回数として集約しています。",
        "",
        "## Summary",
        "",
        "| Platform | Occurrences | Unique warnings |",
        "| --- | ---: | ---: |",
    ]
    for platform in sorted(grouped):
        groups = grouped[platform]
        lines.append(
            f"| {platform} | {sum(group.occurrences for group in groups.values())} | {len(groups)} |"
        )
    lines.extend(
        [
            "",
            f"Linux/macOS共通警告: **{len(shared)}**",
            "",
            "## Shared warnings",
            "",
        ]
    )
    if not shared:
        lines.append("共通警告はありません。")
    else:
        for signature in sorted(shared):
            message, location = signature.rsplit("|", 1)
            lines.append(f"- `{message}` — `{location}`")

    for platform in sorted(grouped):
        lines.extend(["", f"## {platform}", ""])
        for signature, group in sorted(grouped[platform].items()):
            lines.append(f"### `{group.message}`")
            lines.append(f"- Location: `{group.location or 'unknown'}`")
            lines.append(f"- Occurrences: {group.occurrences}")
            lines.append(f"- Phases: {', '.join(sorted(group.phases))}")
            if group.packages:
                lines.append(f"- Packages: {', '.join(sorted(group.packages))}")
            lines.append(f"- Sources: {', '.join(f'`{source}`' for source in sorted(group.sources))}")

    return "\n".join(lines) + "\n"


def workflow_id(run_dir: Path) -> str:
    match = RUN_ID_RE.match(run_dir.name)
    return match.group(1) if match else "unknown"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--logs-root", type=Path, default=Path("ignore"))
    parser.add_argument("--run-dir", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()

    run_dir = args.run_dir or find_latest_run(args.logs_root)
    logs = build_logs(run_dir)
    if not logs:
        raise SystemExit(f"no normal build logs found under {run_dir}")

    records: list[WarningRecord] = []
    for platform, path in logs:
        records.extend(extract_warnings(path.read_text(encoding="utf-8", errors="replace"), path.name, platform))

    output = args.output or args.logs_root / f"build-warnings-{workflow_id(run_dir)}.md"
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(render_report(run_dir, records), encoding="utf-8")
    print(f"run: {run_dir}")
    print(f"logs: {', '.join(path.name for _, path in logs)}")
    print(f"warnings: {len(records)} occurrences, {len(group_records(records))} platforms")
    print(f"shared: {len(shared_warning_signatures(records))}")
    print(f"report: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
