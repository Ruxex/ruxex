from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import generate_crate_dependency_graph as graph_generator


class CrateDependencyGraphTest(unittest.TestCase):
    def test_builds_member_only_graph_with_workspace_and_target_dependencies(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.write(
                root / "Cargo.toml",
                """
[workspace]
members = ["crates/*"]

[workspace.package]
version = "1.2.3"

[workspace.dependencies]
bravo = { path = "crates/bravo" }
""",
            )
            self.write(
                root / "crates/alpha/Cargo.toml",
                """
[package]
name = "alpha"
version.workspace = true

[dependencies]
bravo.workspace = true
renamed-charlie = { package = "charlie", path = "../charlie", optional = true }
serde = "1"

[target.'cfg(unix)'.dev-dependencies]
bravo = { workspace = true, features = ["testing"] }
""",
            )
            self.write(
                root / "crates/bravo/Cargo.toml",
                """
[package]
name = "bravo"
version = "0.1.0"

[build-dependencies]
charlie = { path = "../charlie" }
""",
            )
            self.write(
                root / "crates/charlie/Cargo.toml",
                """
[package]
name = "charlie"
version = "0.1.0"
""",
            )

            graph = graph_generator.build_graph(root)

        self.assertEqual(graph["summary"]["crate_count"], 3)
        self.assertEqual(graph["summary"]["dependency_edge_count"], 3)
        crates = {crate["name"]: crate for crate in graph["crates"]}
        self.assertEqual(crates["alpha"]["version"], "1.2.3")
        self.assertEqual(crates["alpha"]["direct_dependencies"], ["bravo", "charlie"])
        self.assertEqual(
            crates["alpha"]["transitive_dependencies"], ["bravo", "charlie"]
        )
        self.assertEqual(crates["charlie"]["transitive_dependents"], ["alpha", "bravo"])
        alpha_bravo = next(
            edge
            for edge in graph["edges"]
            if edge["from"] == "alpha" and edge["to"] == "bravo"
        )
        self.assertEqual(alpha_bravo["kinds"], ["development", "normal"])
        self.assertEqual(alpha_bravo["declarations"][0]["target"], "cfg(unix)")

    def test_reports_cycles_and_assigns_equal_depth_to_cycle_members(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.write(
                root / "Cargo.toml",
                """
[workspace]
members = ["a", "b", "leaf"]
""",
            )
            self.write(
                root / "a/Cargo.toml",
                '[package]\nname = "a"\nversion = "0.1.0"\n[dependencies]\nb = { path = "../b" }\n',
            )
            self.write(
                root / "b/Cargo.toml",
                '[package]\nname = "b"\nversion = "0.1.0"\n[dependencies]\na = { path = "../a" }\nleaf = { path = "../leaf" }\n',
            )
            self.write(
                root / "leaf/Cargo.toml",
                '[package]\nname = "leaf"\nversion = "0.1.0"\n',
            )

            graph = graph_generator.build_graph(root)

        self.assertEqual(graph["cycles"], [{"id": 1, "crates": ["a", "b"]}])
        crates = {crate["name"]: crate for crate in graph["crates"]}
        self.assertEqual(crates["a"]["dependency_depth"], 1)
        self.assertEqual(crates["b"]["dependency_depth"], 1)
        self.assertEqual(crates["leaf"]["dependency_depth"], 0)

    def test_output_is_deterministic_and_html_embeds_graph(self) -> None:
        graph = {
            "summary": {"crate_count": 0},
            "crates": [],
            "edges": [],
            "value": "</script>",
        }
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            json_path = root / "graph.json"
            html_path = root / "graph.html"
            graph_generator.write_outputs(graph, json_path, html_path)
            first_json = json_path.read_text(encoding="utf-8")
            first_html = html_path.read_text(encoding="utf-8")
            graph_generator.write_outputs(graph, json_path, html_path)

            self.assertEqual(json_path.read_text(encoding="utf-8"), first_json)
            self.assertEqual(html_path.read_text(encoding="utf-8"), first_html)
            self.assertEqual(json.loads(first_json), graph)
            self.assertIn("\\u003c/script\\u003e", first_html)

    @staticmethod
    def write(path: Path, content: str) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content.strip() + "\n", encoding="utf-8")


if __name__ == "__main__":
    unittest.main()
