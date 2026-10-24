#!/usr/bin/env python3
"""Generate mechanical entries for the Ruxex Codex upstream audit ledger."""

from __future__ import annotations

import argparse
import datetime as dt
import re
import subprocess
import sys
import textwrap
from dataclasses import dataclass
from pathlib import Path
from typing import Sequence


FORK_COMMIT_RE = re.compile(
    r"https://github\.com/openai/codex/commit/(?P<commit>[0-9a-f]{40})"
)
LEDGER_COMMIT_RE = re.compile(
    r"^- Upstream commit: `(?P<commit>[0-9a-f]{40})`$", re.MULTILINE
)


@dataclass(frozen=True)
class UpstreamCommit:
    commit_hash: str
    committer_at: dt.datetime
    message: str

    @property
    def utc_date(self) -> dt.date:
        return self.committer_at.astimezone(dt.timezone.utc).date()

    @property
    def subject(self) -> str:
        return self.message.splitlines()[0] if self.message.splitlines() else "(no subject)"


def validate_target_date(value: str, *, today: dt.date | None = None) -> dt.date:
    try:
        target = dt.datetime.strptime(value, "%Y-%m-%d").date()
    except ValueError as exc:
        raise ValueError(f"invalid date {value!r}; expected YYYY-MM-DD") from exc

    current_utc_date = today or dt.datetime.now(dt.timezone.utc).date()
    if target >= current_utc_date:
        raise ValueError(
            f"UTC day has not ended for {target.isoformat()} (current UTC date is "
            f"{current_utc_date.isoformat()})"
        )
    return target


def run_git(repository: Path, args: Sequence[str]) -> str:
    result = subprocess.run(
        ["git", "-C", str(repository), *args],
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout


def read_fork_point(repository_root: Path) -> str:
    readme = repository_root / "docs" / "upstream-audit" / "README.md"
    match = FORK_COMMIT_RE.search(readme.read_text(encoding="utf-8"))
    if match is None:
        raise ValueError(f"could not find the OpenAI Codex fork commit in {readme}")
    return match.group("commit")


def parse_git_log_output(output: str, target_date: dt.date) -> list[UpstreamCommit]:
    fields = output.split("\x00")
    if fields and not fields[-1].strip():
        fields.pop()
    if len(fields) % 3 != 0:
        raise RuntimeError("unexpected git log output while reading upstream commits")

    commits: list[UpstreamCommit] = []
    for index in range(0, len(fields), 3):
        commit_hash, committer_text, message = fields[index : index + 3]
        commit_hash = commit_hash.strip()
        committer_at = dt.datetime.fromisoformat(committer_text.replace("Z", "+00:00"))
        committer_at = committer_at.astimezone(dt.timezone.utc)
        commit = UpstreamCommit(commit_hash, committer_at, message)
        if commit.utc_date == target_date:
            commits.append(commit)
    return commits


def collect_commits(
    upstream_repository: Path,
    fork_point: str,
    target_date: dt.date,
) -> list[UpstreamCommit]:
    output = run_git(
        upstream_repository,
        [
            "log",
            "--reverse",
            "--date-order",
            "--format=%H%x00%cI%x00%B%x00",
            f"{fork_point}..main",
        ],
    )
    return parse_git_log_output(output, target_date)


def render_entry(commit: UpstreamCommit) -> str:
    message = commit.message.rstrip("\n")
    indented_message = textwrap.indent(message, "  ")
    return (
        f"## `{commit.commit_hash}` — {commit.subject}\n\n"
        f"- Upstream commit: `{commit.commit_hash}`\n"
        f"- Upstream committer date (UTC): `{commit.committer_at.strftime('%Y-%m-%dT%H:%M:%SZ')}`\n"
        "- Upstream commit message:\n\n"
        "  ````text\n"
        f"{indented_message}\n"
        "  ````\n\n"
        "- Diff audit:\n"
        "  - <to be audited>\n"
        "- Ruxex summary: <to be written after audit>\n"
        "- Provisional decision: <pending audit>\n"
        "- Reason: <to be written after audit>\n"
        "- Reflection status: not audited\n"
        "- Ruxex-side commit: `N/A`\n"
        "- Notes: <optional>\n"
    )


def ledger_header(target_date: str) -> str:
    return (
        f"# Codex Upstream Audit Ledger — {target_date}\n\n"
        "## Audit Entries\n\n"
        "<!-- Follow the entry format in README.md. -->\n"
    )


def append_missing_entries(
    ledger_path: Path,
    target_date: str,
    commits: Sequence[UpstreamCommit],
) -> int:
    if ledger_path.exists():
        content = ledger_path.read_text(encoding="utf-8")
    else:
        ledger_path.parent.mkdir(parents=True, exist_ok=True)
        content = ledger_header(target_date)

    known_commits = set(LEDGER_COMMIT_RE.findall(content))
    missing = [commit for commit in commits if commit.commit_hash not in known_commits]
    if not missing:
        return 0

    updated = content.rstrip() + "\n\n" + "\n".join(render_entry(commit) for commit in missing)
    ledger_path.write_text(updated.rstrip() + "\n", encoding="utf-8")
    return len(missing)


def generate_for_date(
    *,
    repository_root: Path,
    upstream_repository: Path,
    audit_directory: Path,
    target_date: str,
) -> tuple[Path, int, int]:
    parsed_date = validate_target_date(target_date)
    fork_point = read_fork_point(repository_root)
    commits = collect_commits(upstream_repository, fork_point, parsed_date)
    ledger_path = audit_directory / f"{parsed_date.isoformat()}.md"
    added = append_missing_entries(ledger_path, parsed_date.isoformat(), commits)
    return ledger_path, len(commits), added


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate mechanical upstream audit entries for one completed UTC date."
    )
    parser.add_argument("--date", required=True, help="completed UTC date (YYYY-MM-DD)")
    parser.add_argument(
        "--upstream-repo",
        type=Path,
        help="path to the OpenAI Codex clone (default: ../codex-upstream)",
    )
    parser.add_argument(
        "--audit-dir",
        type=Path,
        help="audit ledger directory (default: docs/upstream-audit)",
    )
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    repository_root = Path(__file__).resolve().parents[1]
    upstream_repository = (args.upstream_repo or repository_root.parent / "codex-upstream").resolve()
    audit_directory = (args.audit_dir or repository_root / "docs" / "upstream-audit").resolve()

    try:
        ledger_path, commit_count, added_count = generate_for_date(
            repository_root=repository_root,
            upstream_repository=upstream_repository,
            audit_directory=audit_directory,
            target_date=args.date,
        )
    except (OSError, RuntimeError, ValueError, subprocess.CalledProcessError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    print(
        f"{ledger_path}: found {commit_count} upstream commits, "
        f"added {added_count} entries"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
