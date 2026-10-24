import datetime as dt
import tempfile
import unittest
from pathlib import Path

from generate_upstream_audit import (
    UpstreamCommit,
    append_missing_entries,
    parse_git_log_output,
    render_entry,
    validate_target_date,
)


class GenerateUpstreamAuditTests(unittest.TestCase):
    def test_parse_git_log_output_accepts_git_trailing_newline(self):
        first_hash = "c" * 40
        second_hash = "d" * 40
        output = (
            f"{first_hash}\x002026-10-23T01:02:03+00:00\x00First\n\x00\n"
            f"{second_hash}\x002026-10-23T02:03:04+00:00\x00Second\n\x00\n"
        )

        commits = parse_git_log_output(output, dt.date(2026, 10, 23))

        self.assertEqual(
            [commit.commit_hash for commit in commits], [first_hash, second_hash]
        )

    def test_rejects_the_current_utc_date(self):
        with self.assertRaisesRegex(ValueError, "UTC day has not ended"):
            validate_target_date("2026-10-24", today=dt.date(2026, 10, 24))

    def test_render_entry_keeps_full_message_and_leaves_audit_fields_open(self):
        commit = UpstreamCommit(
            commit_hash="a" * 40,
            committer_at=dt.datetime(2026, 10, 23, 12, 34, 56, tzinfo=dt.timezone.utc),
            message="Fix the thing\n\nDetailed reason\n",
        )

        rendered = render_entry(commit)

        self.assertIn("- Upstream commit: `" + "a" * 40 + "`", rendered)
        self.assertIn("- Upstream committer date (UTC): `2026-10-23T12:34:56Z`", rendered)
        self.assertIn("  Fix the thing\n\n  Detailed reason", rendered)
        self.assertIn("- Ruxex summary: <to be written after audit>", rendered)
        self.assertIn("- Provisional decision: <pending audit>", rendered)

    def test_append_missing_entries_preserves_existing_audit_notes(self):
        commit = UpstreamCommit(
            commit_hash="b" * 40,
            committer_at=dt.datetime(2026, 10, 22, tzinfo=dt.timezone.utc),
            message="Add a change\n",
        )
        existing = """# Codex Upstream Audit Ledger — 2026-10-22

## Audit Entries

既存の監査メモ。
"""

        with tempfile.TemporaryDirectory() as temp_dir:
            ledger = Path(temp_dir) / "2026-10-22.md"
            ledger.write_text(existing, encoding="utf-8")

            append_missing_entries(ledger, "2026-10-22", [commit])
            first_result = ledger.read_text(encoding="utf-8")
            append_missing_entries(ledger, "2026-10-22", [commit])
            second_result = ledger.read_text(encoding="utf-8")

        self.assertIn("既存の監査メモ。", first_result)
        self.assertEqual(first_result, second_result)
        self.assertEqual(first_result.count("- Upstream commit: `" + "b" * 40 + "`"), 1)


if __name__ == "__main__":
    unittest.main()
