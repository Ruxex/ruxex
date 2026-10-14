#!/usr/bin/env python3
"""Audit repository-owned Codex spellings that must not regress in Rust code.

External endpoint, third-party protocol, and historical/security spellings are
not blanket-approved here; they are reviewed separately in the compatibility
inventory. This check protects the internal names migrated by the Ruxex wave.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RUST_ROOT = ROOT / "crates"

# These are repository-owned runtime, protocol, metadata, and telemetry names.
# Keep this list explicit so a future provider-neutral rename can extend the
# audit without silently broadening the external-contract allowlist.
FORBIDDEN = (
    r"\bCODEX_(?:SESSION_ID|THREAD_ID|VERSION|PERMISSION_PROFILE)\b",
    r"\bCODEX_EXEC_SERVER_(?:URL|NOISE_AUTH_TOKEN)\b",
    r"\bCODEX_APP_SERVER_[A-Z0-9_]+\b",
    r"\bcodex_(?:rollout_budget_units|approval_kind|requires_user_input|request_type|sensitive_action)\b",
    r"x-codex-(?!(?:residency|inference-call-id)\b)",
    r"\.codex-plugin\b",
    r"\bcodex-(?:bundled(?:-alpha)?|primary-runtime|runtimes)\b",
    r"(?-i:\bCODEX_(?:API_KEY|ACCESS_TOKEN|GITHUB_TOKEN|GITHUB_PERSONAL_ACCESS_TOKEN|CONNECTORS_TOKEN|AUTH|INTERNAL_ORIGINATOR_OVERRIDE)\b)",
    r"\bcodex_app\b",
    r"\bcodex_exec\b",
    r"\bcodex_app_server_daemon\b",
    r"\bcodex_mcp_protocol_version\b",
    r"\bcodex[-_]tui\b",
    r"\bcodex_version_env_var\b",
    r"\bcodex_permission_profile_env_var\b",
)

PATTERN = re.compile("|".join(FORBIDDEN), re.IGNORECASE)


def main() -> int:
    failures: list[tuple[Path, int, str]] = []
    for path in sorted(RUST_ROOT.rglob("*.rs")):
        for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            if PATTERN.search(line):
                failures.append((path.relative_to(ROOT), line_number, line.strip()))

    if failures:
        for path, line_number, line in failures:
            print(f"{path}:{line_number}: {line}")
        print(f"found {len(failures)} forbidden Rust lines", file=sys.stderr)
        return 1

    print("no forbidden repository-owned Codex names remain in Rust")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
