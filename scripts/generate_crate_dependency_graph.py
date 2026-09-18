#!/usr/bin/env python3

"""Generate a JSON and standalone HTML graph of workspace member dependencies.

This script deliberately reads Cargo.toml files directly. It does not invoke Cargo,
resolve registry packages, or inspect dependencies outside ``workspace.members``.
"""

from __future__ import annotations

import argparse
import glob
import json
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any

import tomllib

DEPENDENCY_SECTIONS = {
    "dependencies": "normal",
    "dev-dependencies": "development",
    "build-dependencies": "build",
}


def relative_path(path: Path, root: Path) -> str:
    return path.resolve().relative_to(root.resolve()).as_posix()


def read_toml(path: Path) -> dict[str, Any]:
    with path.open("rb") as file:
        return tomllib.load(file)


def matches_any(path: Path, patterns: list[str], root: Path) -> bool:
    relative = relative_path(path, root)
    return any(
        path.match(pattern) or Path(relative).match(pattern) for pattern in patterns
    )


def workspace_manifest_paths(root: Path, root_manifest: dict[str, Any]) -> list[Path]:
    workspace = root_manifest.get("workspace")
    if not isinstance(workspace, dict):
        raise TypeError(f"{root / 'Cargo.toml'} does not define [workspace]")

    members = workspace.get("members", [])
    if not isinstance(members, list) or not all(
        isinstance(item, str) for item in members
    ):
        raise ValueError("workspace.members must be an array of strings")
    excludes = workspace.get("exclude", [])
    if not isinstance(excludes, list) or not all(
        isinstance(item, str) for item in excludes
    ):
        raise ValueError("workspace.exclude must be an array of strings")

    paths: set[Path] = set()
    for member in members:
        candidates = sorted(Path(item) for item in glob.glob(str(root / member)))
        if not candidates:
            candidates = [root / member]
        for candidate in candidates:
            manifest = (
                candidate
                if candidate.name == "Cargo.toml"
                else candidate / "Cargo.toml"
            )
            if matches_any(manifest.parent, excludes, root):
                continue
            if not manifest.is_file():
                raise FileNotFoundError(f"workspace member has no Cargo.toml: {member}")
            paths.add(manifest.resolve())

    # A root package is an implicit member when [package] and [workspace] coexist.
    if "package" in root_manifest:
        paths.add((root / "Cargo.toml").resolve())
    return sorted(paths, key=lambda path: relative_path(path, root))


def dependency_tables(
    manifest: dict[str, Any],
) -> list[tuple[str, str | None, dict[str, Any]]]:
    tables: list[tuple[str, str | None, dict[str, Any]]] = []
    for section, kind in DEPENDENCY_SECTIONS.items():
        value = manifest.get(section, {})
        if isinstance(value, dict):
            tables.append((kind, None, value))

    targets = manifest.get("target", {})
    if isinstance(targets, dict):
        for target, target_table in targets.items():
            if not isinstance(target_table, dict):
                continue
            for section, kind in DEPENDENCY_SECTIONS.items():
                value = target_table.get(section, {})
                if isinstance(value, dict):
                    tables.append((kind, target, value))
    return tables


def dependency_spec(value: Any) -> dict[str, Any]:
    if isinstance(value, str):
        return {"version": value}
    if isinstance(value, dict):
        return dict(value)
    raise ValueError(f"unsupported dependency declaration: {value!r}")


def merge_workspace_dependency(
    alias: str,
    spec: dict[str, Any],
    workspace_dependencies: dict[str, Any],
) -> tuple[dict[str, Any], bool]:
    if spec.get("workspace") is not True:
        return spec, False
    if alias not in workspace_dependencies:
        raise ValueError(
            f"dependency {alias!r} uses workspace = true but is absent from "
            "[workspace.dependencies]"
        )
    inherited = dependency_spec(workspace_dependencies[alias])
    merged = dict(inherited)
    for key, value in spec.items():
        if key != "workspace":
            merged[key] = value
    inherited_features = inherited.get("features", [])
    local_features = spec.get("features", [])
    if inherited_features or local_features:
        merged["features"] = sorted(set(inherited_features) | set(local_features))
    return merged, True


def resolve_dependency_manifest(
    spec: dict[str, Any], base_directory: Path
) -> Path | None:
    path = spec.get("path")
    if not isinstance(path, str):
        return None
    dependency_path = (base_directory / path).resolve()
    if dependency_path.name == "Cargo.toml":
        return dependency_path
    return dependency_path / "Cargo.toml"


def package_value(package: dict[str, Any], workspace: dict[str, Any], key: str) -> Any:
    value = package.get(key)
    if isinstance(value, dict) and value.get("workspace") is True:
        return workspace.get("package", {}).get(key)
    return value


def strongly_connected_components(
    names: list[str], adjacency: dict[str, set[str]]
) -> list[list[str]]:
    index = 0
    stack: list[str] = []
    on_stack: set[str] = set()
    indices: dict[str, int] = {}
    lowlinks: dict[str, int] = {}
    components: list[list[str]] = []

    def visit(name: str) -> None:
        nonlocal index
        indices[name] = index
        lowlinks[name] = index
        index += 1
        stack.append(name)
        on_stack.add(name)

        for dependency in sorted(adjacency[name]):
            if dependency not in indices:
                visit(dependency)
                lowlinks[name] = min(lowlinks[name], lowlinks[dependency])
            elif dependency in on_stack:
                lowlinks[name] = min(lowlinks[name], indices[dependency])

        if lowlinks[name] == indices[name]:
            component: list[str] = []
            while True:
                member = stack.pop()
                on_stack.remove(member)
                component.append(member)
                if member == name:
                    break
            components.append(sorted(component))

    for name in names:
        if name not in indices:
            visit(name)
    return sorted(components, key=lambda component: component[0])


def transitive_reachable(start: str, adjacency: dict[str, set[str]]) -> list[str]:
    seen: set[str] = set()
    pending = list(adjacency[start])
    while pending:
        name = pending.pop()
        if name == start or name in seen:
            continue
        seen.add(name)
        pending.extend(adjacency[name] - seen)
    return sorted(seen)


def component_depths(
    components: list[list[str]], adjacency: dict[str, set[str]]
) -> dict[str, int]:
    component_for = {
        name: component_index
        for component_index, component in enumerate(components)
        for name in component
    }
    component_edges: dict[int, set[int]] = defaultdict(set)
    for source, dependencies in adjacency.items():
        source_component = component_for[source]
        for dependency in dependencies:
            target_component = component_for[dependency]
            if source_component != target_component:
                component_edges[source_component].add(target_component)

    cache: dict[int, int] = {}

    def depth(component: int) -> int:
        if component not in cache:
            dependencies = component_edges[component]
            cache[component] = (
                0 if not dependencies else 1 + max(depth(item) for item in dependencies)
            )
        return cache[component]

    return {name: depth(component_for[name]) for name in adjacency}


def build_graph(root: Path) -> dict[str, Any]:
    root = root.resolve()
    root_manifest = read_toml(root / "Cargo.toml")
    manifest_paths = workspace_manifest_paths(root, root_manifest)
    manifests = {path: read_toml(path) for path in manifest_paths}

    names_by_manifest: dict[Path, str] = {}
    manifests_by_name: dict[str, tuple[Path, dict[str, Any]]] = {}
    for path, manifest in manifests.items():
        package = manifest.get("package")
        if not isinstance(package, dict) or not isinstance(package.get("name"), str):
            raise TypeError(f"workspace member has no package.name: {path}")
        name = package["name"]
        if name in manifests_by_name:
            raise ValueError(f"duplicate workspace package name: {name}")
        names_by_manifest[path] = name
        manifests_by_name[name] = (path, manifest)

    workspace = root_manifest.get("workspace", {})
    workspace_dependencies = workspace.get("dependencies", {})
    edges_by_pair: dict[tuple[str, str], list[dict[str, Any]]] = defaultdict(list)

    for source in sorted(manifests_by_name):
        manifest_path, manifest = manifests_by_name[source]
        for kind, target, dependencies in dependency_tables(manifest):
            for alias, raw_spec in sorted(dependencies.items()):
                spec = dependency_spec(raw_spec)
                spec, inherited = merge_workspace_dependency(
                    alias, spec, workspace_dependencies
                )
                dependency_manifest = resolve_dependency_manifest(
                    spec,
                    root if inherited else manifest_path.parent,
                )
                if dependency_manifest not in names_by_manifest:
                    continue
                destination = names_by_manifest[dependency_manifest]
                edges_by_pair[(source, destination)].append(
                    {
                        "alias": alias,
                        "kind": kind,
                        "target": target,
                        "optional": bool(spec.get("optional", False)),
                        "default_features": bool(spec.get("default-features", True)),
                        "features": sorted(spec.get("features", [])),
                        "workspace_inherited": inherited,
                    }
                )

    names = sorted(manifests_by_name)
    adjacency = {name: set() for name in names}
    reverse = {name: set() for name in names}
    edges: list[dict[str, Any]] = []
    for (source, destination), declarations in sorted(edges_by_pair.items()):
        adjacency[source].add(destination)
        reverse[destination].add(source)
        declarations.sort(
            key=lambda item: (
                item["kind"],
                item["target"] or "",
                item["alias"],
                item["optional"],
            )
        )
        edges.append(
            {
                "from": source,
                "to": destination,
                "kinds": sorted({item["kind"] for item in declarations}),
                "optional": all(item["optional"] for item in declarations),
                "declarations": declarations,
            }
        )

    components = strongly_connected_components(names, adjacency)
    cycles = [
        component
        for component in components
        if len(component) > 1 or component[0] in adjacency[component[0]]
    ]
    cycle_for = {
        name: cycle_index
        for cycle_index, cycle in enumerate(cycles, start=1)
        for name in cycle
    }
    depths = component_depths(components, adjacency)

    crates: list[dict[str, Any]] = []
    for name in names:
        manifest_path, manifest = manifests_by_name[name]
        package = manifest["package"]
        crates.append(
            {
                "name": name,
                "manifest_path": relative_path(manifest_path, root),
                "version": package_value(package, workspace, "version"),
                "dependency_depth": depths[name],
                "cycle": cycle_for.get(name),
                "direct_dependencies": sorted(adjacency[name]),
                "direct_dependents": sorted(reverse[name]),
                "transitive_dependencies": transitive_reachable(name, adjacency),
                "transitive_dependents": transitive_reachable(name, reverse),
            }
        )

    return {
        "schema_version": 1,
        "source": "Cargo.toml workspace.members and member manifests",
        "workspace_manifest": "Cargo.toml",
        "summary": {
            "crate_count": len(crates),
            "dependency_edge_count": len(edges),
            "cycle_count": len(cycles),
            "maximum_dependency_depth": max(depths.values(), default=0),
        },
        "crates": crates,
        "edges": edges,
        "cycles": [
            {"id": cycle_index, "crates": cycle}
            for cycle_index, cycle in enumerate(cycles, start=1)
        ],
    }


HTML_TEMPLATE = r"""<!doctype html>
<html lang="ja">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Workspace crate dependencies</title>
<style>
:root { color-scheme: dark; font-family: ui-sans-serif, system-ui, sans-serif; }
* { box-sizing: border-box; }
body { margin: 0; background: #0d1117; color: #e6edf3; overflow: hidden; }
header { height: 64px; display: flex; align-items: center; gap: 24px; padding: 10px 16px; border-bottom: 1px solid #30363d; }
h1 { margin: 0; font-size: 18px; }
#stats { color: #8b949e; font-size: 13px; }
#layout { display: grid; grid-template-columns: 280px 1fr 340px; height: calc(100vh - 64px); }
aside { padding: 14px; overflow: auto; border-right: 1px solid #30363d; }
#details { border-right: 0; border-left: 1px solid #30363d; }
label { display: block; margin: 9px 0; font-size: 13px; color: #c9d1d9; }
input[type=search] { width: 100%; padding: 8px; color: inherit; background: #161b22; border: 1px solid #30363d; border-radius: 6px; }
button { color: inherit; background: #21262d; border: 1px solid #30363d; border-radius: 6px; padding: 6px 10px; cursor: pointer; }
#crate-list { margin-top: 12px; }
.crate-item { display: block; width: 100%; padding: 7px 8px; text-align: left; border: 0; border-radius: 4px; background: transparent; }
.crate-item:hover, .crate-item.active { background: #21262d; }
#viewport { position: relative; overflow: hidden; }
svg { width: 100%; height: 100%; cursor: grab; user-select: none; }
svg.dragging { cursor: grabbing; }
.edge { fill: none; stroke: #484f58; stroke-width: 1.2; opacity: .5; marker-end: url(#arrow); }
.edge.development { stroke-dasharray: 5 4; }
.edge.build { stroke: #a371f7; }
.edge.optional { opacity: .25; }
.edge.highlight { stroke: #f2cc60; stroke-width: 2.5; opacity: 1; }
.node rect { fill: #161b22; stroke: #388bfd; stroke-width: 1.2; rx: 5; }
.node text { fill: #e6edf3; font-size: 11px; pointer-events: none; }
.node.cycle rect { stroke: #f0883e; stroke-width: 2; }
.node.dim { opacity: .14; }
.node.selected rect { fill: #1f6feb; stroke: #79c0ff; stroke-width: 2.5; }
.node:hover rect { fill: #21262d; }
.section { margin-bottom: 20px; }
.section h2 { margin: 0 0 8px; font-size: 14px; }
.pill { display: inline-block; margin: 2px; padding: 3px 6px; border-radius: 9px; background: #21262d; font-size: 11px; }
.muted { color: #8b949e; font-size: 12px; }
.empty { color: #8b949e; font-style: italic; }
#hint { position: absolute; left: 12px; bottom: 10px; color: #8b949e; font-size: 11px; pointer-events: none; }
</style>
</head>
<body>
<header><h1>Workspace crate dependencies</h1><div id="stats"></div><button id="fit">全体表示</button></header>
<div id="layout">
  <aside>
    <input id="search" type="search" placeholder="クレートを検索">
    <label><input class="kind" type="checkbox" value="normal" checked> 通常依存</label>
    <label><input class="kind" type="checkbox" value="development" checked> 開発依存</label>
    <label><input class="kind" type="checkbox" value="build" checked> ビルド依存</label>
    <label><input id="optional" type="checkbox" checked> optional依存を表示</label>
    <div id="crate-list"></div>
  </aside>
  <main id="viewport"><svg id="graph"><defs><marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse"><path d="M 0 0 L 10 5 L 0 10 z" fill="#6e7681"/></marker></defs><g id="scene"></g></svg><div id="hint">ドラッグ: 移動 / ホイール: 拡大縮小 / ノード選択: 影響範囲を強調</div></main>
  <aside id="details"><div class="empty">クレートを選択してください。</div></aside>
</div>
<script id="graph-data" type="application/json">__GRAPH_DATA__</script>
<script>
const data = JSON.parse(document.getElementById('graph-data').textContent);
const byName = new Map(data.crates.map(item => [item.name, item]));
const edgeByKey = new Map(data.edges.map(edge => [`${edge.from}\0${edge.to}`, edge]));
const scene = document.getElementById('scene');
const svg = document.getElementById('graph');
const list = document.getElementById('crate-list');
const details = document.getElementById('details');
const search = document.getElementById('search');
let selected = null;
let transform = { x: 30, y: 30, scale: 1 };

document.getElementById('stats').textContent = `${data.summary.crate_count} crates / ${data.summary.dependency_edge_count} edges / ${data.summary.cycle_count} cycles`;

function visibleEdges() {
  const kinds = new Set([...document.querySelectorAll('.kind:checked')].map(item => item.value));
  const optional = document.getElementById('optional').checked;
  return data.edges.filter(edge => edge.kinds.some(kind => kinds.has(kind)) && (optional || !edge.optional));
}

function positions() {
  const columns = new Map();
  for (const crate of data.crates) {
    if (!columns.has(crate.dependency_depth)) columns.set(crate.dependency_depth, []);
    columns.get(crate.dependency_depth).push(crate.name);
  }
  const maxDepth = data.summary.maximum_dependency_depth;
  const result = new Map();
  for (const [depth, names] of columns) {
    names.sort();
    names.forEach((name, index) => result.set(name, { x: 40 + (maxDepth - depth) * 210, y: 40 + index * 42 }));
  }
  return result;
}

const coords = positions();

function relationSet(name) {
  if (!name) return new Set();
  const item = byName.get(name);
  return new Set([name, ...item.transitive_dependencies, ...item.transitive_dependents]);
}

function renderGraph() {
  scene.replaceChildren();
  const visible = visibleEdges();
  const relations = relationSet(selected);
  for (const edge of visible) {
    const from = coords.get(edge.from), to = coords.get(edge.to);
    const path = document.createElementNS('http://www.w3.org/2000/svg', 'path');
    const middle = (from.x + to.x) / 2;
    path.setAttribute('d', `M ${from.x + 166} ${from.y + 14} C ${middle} ${from.y + 14}, ${middle} ${to.y + 14}, ${to.x} ${to.y + 14}`);
    const primaryKind = edge.kinds.includes('normal') ? 'normal' : edge.kinds[0];
    path.setAttribute('class', `edge ${primaryKind}${edge.optional ? ' optional' : ''}${selected && (edge.from === selected || edge.to === selected) ? ' highlight' : ''}`);
    scene.appendChild(path);
  }
  for (const crate of data.crates) {
    const point = coords.get(crate.name);
    const group = document.createElementNS('http://www.w3.org/2000/svg', 'g');
    group.setAttribute('transform', `translate(${point.x},${point.y})`);
    group.setAttribute('class', `node${crate.cycle ? ' cycle' : ''}${selected === crate.name ? ' selected' : ''}${selected && !relations.has(crate.name) ? ' dim' : ''}`);
    group.dataset.name = crate.name;
    const rect = document.createElementNS('http://www.w3.org/2000/svg', 'rect');
    rect.setAttribute('width', 166); rect.setAttribute('height', 28);
    const text = document.createElementNS('http://www.w3.org/2000/svg', 'text');
    text.setAttribute('x', 8); text.setAttribute('y', 18); text.textContent = crate.name;
    group.append(rect, text);
    group.addEventListener('click', event => { event.stopPropagation(); select(crate.name); });
    scene.appendChild(group);
  }
  applyTransform();
}

function pills(names) {
  if (!names.length) return '<div class="empty">なし</div>';
  return names.map(name => `<button class="pill" data-name="${name}">${name}</button>`).join('');
}

function select(name) {
  selected = selected === name ? null : name;
  document.querySelectorAll('.crate-item').forEach(item => item.classList.toggle('active', item.dataset.name === selected));
  if (!selected) {
    details.innerHTML = '<div class="empty">クレートを選択してください。</div>';
  } else {
    const item = byName.get(selected);
    details.innerHTML = `<div class="section"><h2>${item.name}</h2><div class="muted">${item.manifest_path}</div><div class="muted">依存深度: ${item.dependency_depth}${item.cycle ? ` / cycle ${item.cycle}` : ''}</div></div>
      <div class="section"><h2>直接の依存先 (${item.direct_dependencies.length})</h2>${pills(item.direct_dependencies)}</div>
      <div class="section"><h2>直接の依存元 (${item.direct_dependents.length})</h2>${pills(item.direct_dependents)}</div>
      <div class="section"><h2>推移的な依存先 (${item.transitive_dependencies.length})</h2>${pills(item.transitive_dependencies)}</div>
      <div class="section"><h2>推移的な依存元 (${item.transitive_dependents.length})</h2>${pills(item.transitive_dependents)}</div>`;
    details.querySelectorAll('[data-name]').forEach(button => button.addEventListener('click', () => select(button.dataset.name)));
  }
  renderGraph();
}

function renderList() {
  const query = search.value.trim().toLowerCase();
  list.replaceChildren();
  for (const crate of data.crates.filter(item => item.name.toLowerCase().includes(query))) {
    const button = document.createElement('button');
    button.className = `crate-item${selected === crate.name ? ' active' : ''}`;
    button.dataset.name = crate.name;
    button.textContent = `${crate.name} (${crate.direct_dependents.length}↑ / ${crate.direct_dependencies.length}↓)`;
    button.addEventListener('click', () => select(crate.name));
    list.appendChild(button);
  }
}

function applyTransform() { scene.setAttribute('transform', `translate(${transform.x} ${transform.y}) scale(${transform.scale})`); }
function fit() {
  const box = scene.getBBox();
  if (!box.width || !box.height) return;
  const rect = svg.getBoundingClientRect();
  const scale = Math.min((rect.width - 60) / box.width, (rect.height - 60) / box.height, 1);
  transform = { x: (rect.width - box.width * scale) / 2 - box.x * scale, y: (rect.height - box.height * scale) / 2 - box.y * scale, scale };
  applyTransform();
}

let drag = null;
svg.addEventListener('pointerdown', event => { drag = { x: event.clientX, y: event.clientY, tx: transform.x, ty: transform.y }; svg.setPointerCapture(event.pointerId); svg.classList.add('dragging'); });
svg.addEventListener('pointermove', event => { if (!drag) return; transform.x = drag.tx + event.clientX - drag.x; transform.y = drag.ty + event.clientY - drag.y; applyTransform(); });
svg.addEventListener('pointerup', () => { drag = null; svg.classList.remove('dragging'); });
svg.addEventListener('wheel', event => { event.preventDefault(); const rect = svg.getBoundingClientRect(); const x = event.clientX - rect.left, y = event.clientY - rect.top; const next = Math.max(.08, Math.min(3, transform.scale * (event.deltaY < 0 ? 1.12 : .89))); transform.x = x - (x - transform.x) * next / transform.scale; transform.y = y - (y - transform.y) * next / transform.scale; transform.scale = next; applyTransform(); }, { passive: false });
svg.addEventListener('click', () => { if (selected) select(selected); });
search.addEventListener('input', renderList);
document.querySelectorAll('.kind, #optional').forEach(item => item.addEventListener('change', renderGraph));
document.getElementById('fit').addEventListener('click', fit);
renderList(); renderGraph(); requestAnimationFrame(fit);
</script>
</body>
</html>
"""


def write_outputs(graph: dict[str, Any], json_path: Path, html_path: Path) -> None:
    json_text = json.dumps(graph, ensure_ascii=False, indent=2, sort_keys=True) + "\n"
    json_path.parent.mkdir(parents=True, exist_ok=True)
    html_path.parent.mkdir(parents=True, exist_ok=True)
    json_path.write_text(json_text, encoding="utf-8")
    embedded = json.dumps(graph, ensure_ascii=False, separators=(",", ":"))
    embedded = (
        embedded.replace("<", "\\u003c").replace(">", "\\u003e").replace("&", "\\u0026")
    )
    html_path.write_text(
        HTML_TEMPLATE.replace("__GRAPH_DATA__", embedded), encoding="utf-8"
    )


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Generate the workspace member crate dependency graph."
    )
    parser.add_argument(
        "--workspace-root",
        type=Path,
        default=Path(__file__).resolve().parent.parent,
        help="Directory containing the workspace Cargo.toml.",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=Path("target/crate-dependency-graph"),
        help="Output directory, relative to the workspace root by default.",
    )
    args = parser.parse_args()

    root = args.workspace_root.resolve()
    output_dir = args.output_dir
    if not output_dir.is_absolute():
        output_dir = root / output_dir
    try:
        graph = build_graph(root)
        json_path = output_dir / "crate-dependencies.json"
        html_path = output_dir / "crate-dependencies.html"
        write_outputs(graph, json_path, html_path)
    except (
        FileNotFoundError,
        OSError,
        TypeError,
        tomllib.TOMLDecodeError,
        ValueError,
    ) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1

    summary = graph["summary"]
    print(
        f"Generated {summary['crate_count']} crates and "
        f"{summary['dependency_edge_count']} dependency edges."
    )
    print(f"JSON: {json_path}")
    print(f"HTML: {html_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
